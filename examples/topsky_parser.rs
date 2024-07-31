use std::{env::args_os, io};

use vatsim_parser::topsky::Topsky;

fn main() {
    tracing_subscriber::fmt().with_writer(io::stderr).init();
    let path = args_os()
        .nth(1)
        .expect("missing argument: path to topsky folder");

    match Topsky::parse(path.into()) {
        Ok(topsky) => println!("{}", serde_json::to_string(&topsky).unwrap()),
        Err(e) => eprintln!("{e}"),
    }
}
