use vatsim_parser::topsky::Topsky;

fn main() {
    let topsky =
        Topsky::parse("../vatsim-germany-edmm/EDMM/Plugins/Topsky/TWR_PHX_DAY".into()).unwrap();

    println!("{}", serde_json::to_string(&topsky).unwrap());
}
