use std::fs;

use vatsim_parser::asr::Asr;

fn main() {
    let asr = Asr::parse(&fs::read("../vatsim-germany-edmm/EDMM/ASR/iCAS2/EDDM_APP.asr").unwrap())
        .expect("unsuccessful parse");

    println!("{}", serde_json::to_string(&asr).unwrap());
}
