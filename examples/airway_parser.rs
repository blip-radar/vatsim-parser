use std::{env::args_os, fs};

use vatsim_parser::airway::parse_airway_txt;

fn main() {
    let path = args_os()
        .nth(1)
        .expect("missing argument: path to airway.txt");
    let airway = parse_airway_txt(&fs::read(path).unwrap()).expect("unsuccessful parse");

    println!("{}", serde_json::to_string(&airway).unwrap());
}
