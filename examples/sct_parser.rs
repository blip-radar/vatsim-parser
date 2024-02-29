use std::{env::args_os, fs};

use vatsim_parser::sct::Sct;

fn main() {
    let path = args_os().nth(1).expect("missing argument: path to .sct");
    let sct = Sct::parse(&fs::read(path).unwrap()).expect("unsuccessful parse");

    println!("{}", serde_json::to_string(&sct).unwrap());
}
