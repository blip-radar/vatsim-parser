use std::{env::args_os, fs, io};

use vatsim_parser::symbology::Symbology;

fn main() {
    tracing_subscriber::fmt().with_writer(io::stderr).init();
    let path = args_os()
        .nth(1)
        .expect("missing argument: path to symbology file");

    match Symbology::parse(&fs::read(path).unwrap()) {
        Ok(sym) => println!("{}", serde_json::to_string(&sym).unwrap()),
        Err(e) => eprintln!("{e}"),
    }
}
