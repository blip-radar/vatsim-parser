fn main() {}
// use std::{env::args_os, fs, io};
//
// use vatsim_parser::airway::parse_airway_txt;
//
// fn main() {
//     tracing_subscriber::fmt().with_writer(io::stderr).init();
//     let path = args_os()
//         .nth(1)
//         .expect("missing argument: path to airway.txt");
//     let airway = parse_airway_txt(&fs::read(path).unwrap()).expect("unsuccessful parse");
//
//     println!("{}", serde_json::to_string(&airway).unwrap());
// }
