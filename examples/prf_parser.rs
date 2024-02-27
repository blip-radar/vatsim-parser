use std::{fs, path::PathBuf};

use vatsim_parser::prf::Prf;

fn main() {
    let path = PathBuf::from("../vatsim-germany-edmm/iCAS2.prf");
    let contents = fs::read(&path).unwrap();
    let prf = Prf::parse(&path, &contents).unwrap();

    println!("{}", serde_json::to_string(&prf).unwrap());
}
