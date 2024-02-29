use std::env::args_os;

use vatsim_parser::topsky::Topsky;

fn main() {
    let path = args_os()
        .nth(1)
        .expect("missing argument: path to topsky folder");
    let topsky = Topsky::parse(path.into()).unwrap();

    println!("{}", serde_json::to_string(&topsky).unwrap());
}
