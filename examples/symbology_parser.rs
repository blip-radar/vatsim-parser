use std::{env::args_os, fs};

use vatsim_parser::symbology::Symbology;

fn main() {
    let path = args_os()
        .nth(1)
        .expect("missing argument: path to symbology file");

    match Symbology::parse(&fs::read(path).unwrap()) {
        Ok(sym) => println!("{}", serde_json::to_string(&sym).unwrap()),
        Err(e) => eprintln!("{e}"),
    }
}
