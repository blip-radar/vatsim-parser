use std::fs;

use vatsim_parser::symbology::Symbology;

fn main() {
    let symbology = Symbology::parse(
        &fs::read("../vatsim-germany-edmm/EDMM/Settings/iCAS2/Symbology.txt").unwrap(),
    )
    .expect("unsuccessful parse");

    println!("{}", serde_json::to_string(&symbology).unwrap());
}
