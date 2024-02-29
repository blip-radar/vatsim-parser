use std::{env::args_os, fs, path::Path};

use vatsim_parser::prf::Prf;

fn main() {
    let path = args_os().nth(1).expect("missing argument: path to .prf");
    let contents = fs::read(&path).unwrap();
    let prf = Prf::parse(Path::new(&path), &contents).unwrap();

    println!("{}", serde_json::to_string(&prf).unwrap());
}
