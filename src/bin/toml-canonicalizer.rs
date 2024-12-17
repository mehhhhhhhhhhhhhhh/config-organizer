use clap::Parser;
use std::fs::File;
use std::io::stdin;
use std::io::Write;
use std::path::PathBuf;
use toml;
use json_canon;

#[derive(Parser)]
struct Args {
    input_file_path: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();
    let mut crap = String::new();
    let mut reader: Box<dyn std::io::Read> = match args.input_file_path {
        None => Box::new(stdin()),
        Some(path) => Box::new(File::open(path).unwrap()),
    };
    reader.read_to_string(&mut crap).unwrap();
    let stuff: toml::Table = toml::from_str(&crap).unwrap();
    let mut stdout = std::io::stdout().lock();
    json_canon::to_writer(&mut stdout, &stuff).unwrap();
    writeln!(&mut stdout, "").unwrap();
}
