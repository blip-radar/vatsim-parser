use std::{env::args_os, fs, io};

use vatsim_parser::asr::Asr;

fn main() {
    tracing_subscriber::fmt().with_writer(io::stderr).init();
    let path = args_os().nth(1).expect("missing argument: path to .asr");
    match Asr::parse(&fs::read(path).unwrap()) {
        Ok(asr) => println!("{}", serde_json::to_string(&asr).unwrap()),
        Err(e) => eprintln!("{e}"),
    }
}
