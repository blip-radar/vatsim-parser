use std::{env::args_os, fs};

use vatsim_parser::symbology::Symbology;

fn main() {
    let path = args_os()
        .nth(1)
        .expect("missing argument: path to symbology file");
    let symbology = Symbology::parse(&fs::read(path).unwrap()).expect("unsuccessful parse");

    println!("{}", serde_json::to_string(&symbology).unwrap());
}
