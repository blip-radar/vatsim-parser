use std::{env::args_os, fs, io, path::Path};

use vatsim_parser::{adaptation::Adaptation, prf::Prf};

fn main() {
    tracing_subscriber::fmt().with_writer(io::stderr).init();
    let path = args_os().nth(1).expect("missing argument: path to .prf");
    let contents = fs::read(&path).unwrap();
    let prf = Prf::parse(Path::new(&path), &contents).unwrap();
    match Adaptation::from_prf(&prf) {
        Ok(adaptation) => println!("{}", serde_json::to_string(&adaptation).unwrap()),
        Err(e) => eprintln!("{e}"),
    }
}
