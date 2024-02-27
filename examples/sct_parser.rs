use std::fs;

use vatsim_parser::sct::Sct;

fn main() {
    let sct = Sct::parse(&fs::read("../vatsim-germany-edmm/EDMM-AeroNav.sct").unwrap())
        .expect("unsuccessful parse");

    println!("{}", serde_json::to_string(&sct).unwrap());
}
