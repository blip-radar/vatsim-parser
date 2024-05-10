use std::{env::args_os, fs, path::Path};

use vatsim_parser::{adaptation::Adaptation, prf::Prf};

fn main() {
    let path = args_os().nth(1).expect("missing argument: path to .prf");
    let contents = fs::read(&path).unwrap();
    let prf = Prf::parse(Path::new(&path), &contents).unwrap();
    let adaptation = Adaptation::from_prf(prf).unwrap();

    println!("{}", serde_json::to_string(&adaptation).unwrap());
}
