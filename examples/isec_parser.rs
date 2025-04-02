use std::{env::args_os, fs, io};

use vatsim_parser::isec::parse_isec_txt;

fn main() {
    tracing_subscriber::fmt().with_writer(io::stderr).init();
    let path = args_os()
        .nth(1)
        .expect("missing argument: path to isec.txt");
    let isec = parse_isec_txt(&fs::read(path).unwrap()).expect("unsuccessful parse");

    println!("{}", serde_json::to_string(&isec).unwrap());
}
