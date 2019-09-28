use naivebayes::NaiveBayes;
use rust_stemmers::Stemmer;
use scanlex::Scanner;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use v_onto::individual::Individual;

#[derive(PartialEq, Debug, Clone)]
struct ResultNBC {
    tag: String,
    val: f64,
}

pub struct NBC {
    nb: NaiveBayes,
    dict: HashSet<String>,
}

/*
fn to_vec(line: &str) -> Vec<String> {
    let tokens = line.split(" ");
    let mut v: Vec<String> = Vec::new();
    for token in tokens {
        v.push(token.to_string());
    }
    return v;
}
*/
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
    let wpfrase = indv.get_first_literal("pfrase");
    let wgroup_id = indv.get_first_literal("group_id");
    let wtag_id = indv.get_first_literal("tag_id");

    if wpfrase.is_ok() && wgroup_id.is_ok() && wtag_id.is_ok() {
        let tag = wtag_id.unwrap();

        let nb = id2nb.entry(wgroup_id.unwrap()).or_insert(NBC {
            nb: NaiveBayes::new(),
            dict: HashSet::new(),
        });

        let nel = normailze_str(&stemmer, &stopwords, &wpfrase.unwrap());

        println!("train {:?}: {:?}", &tag, &nel);
        nb.nb.train(&nel, &tag.to_lowercase());

        for el in nel {
            nb.dict.insert(el);
        }
    }
}

/*
fn main() {

    if let Ok(csv) = quick_csv::Csv::from_file("Категории Оборудования.csv") {
        for r in csv.into_iter() {
            match r {
                Err(e) => {
                    println!("{}", e);
                }

                Ok(row) => {
                    if let Ok(mut cs) = row.columns() {
                        if let Some(key) = cs.next() {
                            loop {
                                if let Some(el) = cs.next() {
                                    if !el.is_empty() {
                                        let nel = normailze_str(&stemmer, &stopwords, el.to_lowercase().as_ref());

                                        println!("train {:?}: {:?}", &key.to_lowercase(), &nel);
                                        nb.train(&nel, &key.to_lowercase());

                                        for el in nel {
                                            dict.insert(el);
                                        }
                                        //nel.into_iter().map(|el|dict.insert(el.to_string()));
                                    }
                                } else {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    //    let req: String = "вторую неделю вода из трубы, когда исправите".into();
    let req: String = "дуб в лесу зеленый горит".into();
    let req_n = &normailze_str(&stemmer, &stopwords, &req);
    println!("req: {}", req);
    println!("req_n: {:?}", req_n);

    let mut count_word = 0;
    for el in req_n {
        if dict.contains(el) {
            count_word += 1;
        }
    }

    if count_word == 0 {
        println!("ничего не понятно");
    } else {
        let classification = nb.classify(&req_n);

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

        for el in res {
            println!("{:?}", el);
        }
    }
}
*/
