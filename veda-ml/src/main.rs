#[macro_use]
extern crate log;

use crate::ml_naive_bayes::{learn, load_stopwords, NBC};
use chrono::Local;
use env_logger::Builder;
use log::LevelFilter;
use rust_stemmers::{Algorithm, Stemmer};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::{thread, time};
use v_api::IndvOp;
use v_module::module::*;
use v_module::onto::load_onto;
use v_onto::individual::{Individual, RawObj};
use v_onto::onto::Onto;
use v_onto::parser::*;
use v_queue::consumer::*;
use v_queue::record::ErrorQueue;
use v_search::FTQuery;
mod ml_naive_bayes;

fn main() -> std::io::Result<()> {
    let mut id2nb: HashMap<String, NBC> = HashMap::new();
    let stemmer = Stemmer::create(Algorithm::Russian);
    let stopwords = load_stopwords();

    let env_var = "RUST_LOG";
    match std::env::var_os(env_var) {
        Some(val) => println!("use env var: {}: {:?}", env_var, val.to_str()),
        None => std::env::set_var(env_var, "info"),
    }

    Builder::new()
        .format(|buf, record| writeln!(buf, "{} [{}] - {}", Local::now().format("%Y-%m-%dT%H:%M:%S%.3f"), record.level(), record.args()))
        .filter(None, LevelFilter::Info)
        .init();

    let mut module = Module::default();
    let mut onto = Onto::default();

    info!("load onto start");
    load_onto(&mut module.fts, &mut module.storage, &mut onto);
    info!("load onto end");

    //info!("onto: {}", onto);

    let mut queue_consumer = Consumer::new("./data/queue", "extract", "individuals-flow").expect("!!!!!!!!! FAIL QUEUE");
    let mut total_prepared_count: u64 = 0;

    loop {
        let mut size_batch = 0;

        // read queue current part info
        if let Err(e) = queue_consumer.queue.get_info_of_part(queue_consumer.id, true) {
            error!("{} get_info_of_part {}: {}", total_prepared_count, queue_consumer.id, e.as_str());
            continue;
        }

        if queue_consumer.queue.count_pushed - queue_consumer.count_popped == 0 {
            // if not new messages, read queue info
            queue_consumer.queue.get_info_queue();

            if queue_consumer.queue.id > queue_consumer.id {
                size_batch = 1;
            }
        } else if queue_consumer.queue.count_pushed - queue_consumer.count_popped > 0 {
            if queue_consumer.queue.id != queue_consumer.id {
                size_batch = 1;
            } else {
                size_batch = queue_consumer.queue.count_pushed - queue_consumer.count_popped;
            }
        }

        if size_batch > 0 {
            info!("queue: batch size={}", size_batch);
        }

        for _it in 0..size_batch {
            // пробуем взять из очереди заголовок сообщения
            if !queue_consumer.pop_header() {
                break;
            }

            let mut raw = RawObj::new(vec![0; (queue_consumer.header.msg_length) as usize]);

            // заголовок взят успешно, занесем содержимое сообщения в структуру Individual
            if let Err(e) = queue_consumer.pop_body(&mut raw.data) {
                if e == ErrorQueue::FailReadTailMessage {
                    break;
                } else {
                    error!("{} get msg from queue: {}", total_prepared_count, e.as_str());
                    break;
                }
            }

            if let Err(e) = prepare_queue_element(&mut module, &mut onto, &mut Individual::new_raw(raw), &mut id2nb, &stemmer, &stopwords) {
                error!("fail prepare queue element, err={}", e);
            }

            queue_consumer.commit_and_next();
            total_prepared_count += 1;

            if total_prepared_count % 1000 == 0 {
                info!("get from queue, count: {}", total_prepared_count);
            }
        }

        thread::sleep(time::Duration::from_millis(1000));
    }
}

fn prepare_queue_element(
    module: &mut Module,
    onto: &mut Onto,
    msg: &mut Individual,
    id2nb: &mut HashMap<String, NBC>,
    stemmer: &Stemmer,
    stopwords: &HashSet<String>,
) -> Result<(), i32> {
    if let Ok(uri) = parse_raw(msg) {
        msg.obj.uri = uri;

        let wcmd = msg.get_first_integer("cmd");
        if wcmd.is_err() {
            return Err(-1);
        }

        let cmd = IndvOp::from_i64(wcmd.unwrap_or_default());

        let prev_state = msg.get_first_binobj("prev_state");
        let mut prev_state_indv = if prev_state.is_ok() {
            let mut indv = Individual::new_raw(RawObj::new(prev_state.unwrap_or_default()));
            if let Ok(uri) = parse_raw(&mut indv) {
                indv.obj.uri = uri.clone();
            }
            indv
        } else {
            Individual::default()
        };

        let new_state = msg.get_first_binobj("new_state");
        if cmd != IndvOp::Remove && new_state.is_err() {
            return Err(-1);
        }

        let date = msg.get_first_integer("date");
        if date.is_err() {
            return Err(-1);
        }

        let mut new_state_indv = Individual::new_raw(RawObj::new(new_state.unwrap_or_default()));
        if let Ok(uri) = parse_raw(&mut new_state_indv) {
            new_state_indv.obj.uri = uri.clone();

            if let Ok(types) = new_state_indv.get_literals("rdf:type") {
                for itype in types {
                    if onto.is_some_entered(&itype, &["v-s:TrainData".to_owned()]) {
                        learn(&mut new_state_indv, id2nb, stemmer, &stopwords);
                    }
                }
            }
        }
    }

    Ok(())
}

fn full_learn(module: &mut Module) {
    let res = module.fts.query(FTQuery::new_with_user("cfg:VedaSystem", "*"));
    if res.result_code == 200 && res.count > 0 {
        for el in &res.result {
            let mut rindv: Individual = Individual::default();
            if module.storage.get_individual(&el, &mut rindv) {
                rindv.parse_all();
            } else {
                error!("fail read, uri={}", &el);
            }
        }
    }
}
