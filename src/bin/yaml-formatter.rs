use clap::Parser;
use std::fs::File;
use std::io::stdin;
use std::path::PathBuf;
use serde_yaml::Value;
use serde_yaml::Value::*;

#[derive(Parser)]
struct Args {
    input_file_path: Option<PathBuf>,
}

fn sort_mapping_keys(mapping: &mut serde_yaml::Mapping) {
    let mut keys : Vec<Value> = mapping.keys().map(|k| k.clone()).collect();
    keys.sort_by(|a,b| a.partial_cmp(b).unwrap());
    for k in keys {
        let mut val = mapping.remove(&k).expect("key should still exist");
        sort_keys(&mut val);
        mapping.insert(k, val);
    }
}

fn sort_keys(thing: &mut Value) {
    match thing {
        Mapping(ref mut mapping) => sort_mapping_keys(mapping),
        Sequence(vals) => vals.iter_mut().for_each(|v| sort_keys(v)),
        Tagged(tv) => sort_keys(&mut tv.value),
        _ => (),
    }
}

fn main() {
    let args = Args::parse();
    let mut stuff: Value = args.input_file_path
        .map(|path|{ serde_yaml::from_reader(File::open(path).unwrap()).unwrap() })
        .unwrap_or_else(||{ serde_yaml::from_reader(stdin()).unwrap() });
    sort_keys(&mut stuff);
    serde_yaml::to_writer(std::io::stdout(), &stuff).unwrap()
}
