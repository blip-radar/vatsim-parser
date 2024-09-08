use std::{env::args_os, fs, io};

use vatsim_parser::sct::Sct;

fn main() {
    tracing_subscriber::fmt().with_writer(io::stderr).init();
    let path = args_os().nth(1).expect("missing argument: path to .sct");

    match Sct::parse(&fs::read(path).unwrap()) {
        Ok(sct) => println!("{}", serde_json::to_string(&sct).unwrap()),
        Err(e) => eprintln!("{e}"),
    }
}
