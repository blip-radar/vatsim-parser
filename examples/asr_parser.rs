use std::{env::args_os, fs};

use vatsim_parser::asr::Asr;

fn main() {
    let path = args_os().nth(1).expect("missing argument: path to .asr");
    let asr = Asr::parse(&fs::read(path).unwrap()).expect("unsuccessful parse");

    println!("{}", serde_json::to_string(&asr).unwrap());
}
