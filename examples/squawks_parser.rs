use std::{env::args_os, fs};

use vatsim_parser::squawks::SquawksJson;

fn main() {
    let path = args_os()
        .nth(1)
        .expect("missing argument: path to symbology file");
    let squawks: SquawksJson = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();

    println!("{}", serde_json::to_string(&squawks).unwrap());
}
