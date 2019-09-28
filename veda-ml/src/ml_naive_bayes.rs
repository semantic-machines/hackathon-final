use naivebayes::NaiveBayes;
use rust_stemmers::Stemmer;
use scanlex::Scanner;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use v_api::{IndvOp, ResultCode};
use v_module::module::Module;
use v_onto::datatype::Lang;
use v_onto::individual::Individual;
use v_search::FTQuery;

#[derive(PartialEq, Debug, Clone)]
struct ResultNBC {
    tag: String,
    val: f64,
}

pub struct NBC {
    id: String,
    nb: NaiveBayes,
    dict: HashSet<String>,
}

pub struct NBSpecification {
    nb_id: String,
    class: String,
    src_prop: String,
    target_prop: String,
}

fn normailze_str(stemmer: &Stemmer, stopwords: &HashSet<String>, text: &str) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    let scan = Scanner::new(&text);
    for el in scan {
        if let Some(s) = el.as_iden() {
            let ss = s.to_lowercase();

            if !stopwords.contains(&ss) {
                stemmer.stem(&ss);
                v.push(stemmer.stem(&ss).to_string());
            }
        }
    }
    v
}

pub fn load_stopwords() -> HashSet<String> {
    let mut dict = HashSet::new();
    if let Ok(f) = File::open("stopwords.txt") {
        let file = BufReader::new(&f);
        for line in file.lines() {
            let l = line.unwrap();
            println!("{}", &l);
            dict.insert(l);
        }
    }
    dict
}

pub fn learn(indv: &mut Individual, id2nb: &mut HashMap<String, NBC>, stemmer: &Stemmer, stopwords: &HashSet<String>) {
    let wphrase = indv.get_first_literal("hack:keyPhrases");
    let wgroup_id = indv.get_first_literal("v-s:parent");
    let wtag_id = indv.get_first_literal("rdf:value");

    if wphrase.is_ok() && wgroup_id.is_ok() && wtag_id.is_ok() {
        let tag = wtag_id.unwrap();
        let phrase = wphrase.unwrap();
        let nb_id = wgroup_id.unwrap();

        let nb = id2nb.entry(nb_id.to_string()).or_insert(NBC {
            nb: NaiveBayes::new(),
            dict: HashSet::new(),
            id: nb_id,
        });

        for el in phrase.split('\n') {
            if el.trim().is_empty() {
                continue;
            }
            let nel = normailze_str(&stemmer, &stopwords, &el);

            println!("train {} {:?}: {:?}", nb.id, &tag, &nel);
            nb.nb.train(&nel, &tag);

            for el in nel {
                nb.dict.insert(el);
            }
        }
    }
}

pub fn load_specificstions(module: &mut Module) -> HashMap<String, Vec<NBSpecification>> {
    let mut s: HashMap<String, Vec<NBSpecification>> = HashMap::new();
    let res = module.fts.query(FTQuery::new_with_user("cfg:VedaSystem", "'rdf:type' == 'hack:BayesSpecification'"));
    if res.result_code == 200 && res.count > 0 {
        for el in &res.result {
            let mut indv: Individual = Individual::default();
            if module.storage.get_individual(&el, &mut indv) {
                let watch_class = indv.get_first_literal("hack:forClass");
                let src_prop = indv.get_first_literal("hack:sourceProperty");
                let target_prop = indv.get_first_literal("hack:targetProperty");
                let nb_id = indv.get_first_literal("hack:hasBayesClassifier");

                if watch_class.is_ok() && src_prop.is_ok() && target_prop.is_ok() && nb_id.is_ok() {
                    let watch_class = watch_class.unwrap();
                    let specs = s.entry(watch_class.to_string()).or_insert(Vec::new());
                    specs.push(NBSpecification {
                        nb_id: nb_id.unwrap(),
                        class: watch_class,
                        src_prop: src_prop.unwrap(),
                        target_prop: target_prop.unwrap(),
                    });
                }
            }
        }
    }
    s
}

pub fn prepare_user_request(
    indv: &mut Individual,
    module: &mut Module,
    systicket: &str,
    id2nb: &mut HashMap<String, NBC>,
    all_specs: &HashMap<String, Vec<NBSpecification>>,
    itype: &str,
    stemmer: &Stemmer,
    stopwords: &HashSet<String>,
) {
    if let Ok(b) = indv.get_first_bool("hack:isClassified") {
        if b == true {
            return;
        }
    }
    let mut trace_log = String::new();

    if let Some(specs) = all_specs.get(itype) {
        for spec in specs {
            if let Some(nb) = id2nb.get_mut(&spec.nb_id) {
                if let Ok(content) = indv.get_first_literal(&spec.src_prop) {
                    let req_n = &normailze_str(&stemmer, &stopwords, &content);
                    trace_log.push_str(&format!("request: {:?}\n", req_n));

                    let mut count_word = 0;
                    for el in req_n {
                        if nb.dict.contains(el) {
                            count_word += 1;
                        }
                    }
                    if count_word == 0 {
                        trace_log.push_str(&format!("{} ничего не найдено\n", nb.id));
                    } else {
                        let classification = nb.nb.classify(&req_n);

                        let mut res: Vec<ResultNBC> = Vec::new();

                        for (tag, v) in classification.iter() {
                            res.push(ResultNBC {
                                tag: tag.to_string(),
                                val: v.to_owned(),
                            });
                        }

                        res.sort_by(|a, b| {
                            if let Some(r) = b.val.partial_cmp(&a.val) {
                                return r;
                            }
                            std::cmp::Ordering::Less
                        });

                        if let Some(s) = res.get(0) {
                            indv.obj.set_uri(&spec.target_prop, &s.tag);
                        }

                        for el in res {
                            trace_log.push_str(&format!("{:?}\n", el));
                        }
                    }
                }
            }

            indv.obj.set_bool("hack:isClassified", true);
            indv.obj.set_string("rdfs:comment", &trace_log, Lang::RU);

            let res = module.api.update(systicket, IndvOp::Put, indv);

            if res.result != ResultCode::Ok {
                error!("fail update, uri={}, result_code={:?}", indv.obj.uri, res.result);
            } else {
                info!("success update, uri={}", indv.obj.uri);
            }
        }
    }
}
