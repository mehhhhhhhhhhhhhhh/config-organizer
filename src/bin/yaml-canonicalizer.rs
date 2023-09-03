use clap::Parser;
use std::fs::File;
use std::io::stdin;
use std::path::PathBuf;
use serde_yaml::Value;
use json_canon;

#[derive(Parser)]
struct Args {
    input_file_path: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();
    let stuff: Value = args.input_file_path
        .map(|path|{ serde_yaml::from_reader(File::open(path).unwrap()).unwrap() })
        .unwrap_or_else(||{ serde_yaml::from_reader(stdin()).unwrap() });
    println!("{}", json_canon::to_string(&stuff).unwrap());
}
