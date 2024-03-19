use std::{env::args_os, fs};

use vatsim_parser::ese::Ese;

fn main() {
    let path = args_os().nth(1).expect("missing argument: path to .ese");
    let ese = Ese::parse(&fs::read(path).unwrap()).expect("unsuccessful parse");

    println!("{}", serde_json::to_string(&ese).unwrap());
}
