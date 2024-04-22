use clap::Parser;
use std::fs::File;
use std::io::stdin;
use std::io::Write;
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
    let mut stdout = std::io::stdout().lock();
    json_canon::to_writer(&mut stdout, &stuff).unwrap();
    writeln!(&mut stdout, "").unwrap();
}
