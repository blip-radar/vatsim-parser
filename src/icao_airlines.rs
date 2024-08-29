use from_pest::FromPest;
use std::collections::HashMap;
use std::io;
use thiserror::Error;

use super::read_to_string;

mod airlines {
    use pest_derive::Parser;

    #[derive(Parser)]
    #[grammar = "pest/base.pest"]
    #[grammar = "pest/icao_airlines.pest"]
    pub struct Parser;
}

pub mod ast {
    use pest::Span;
    use serde::Serialize;

    use super::airlines::Rule;

    fn span_into_string(span: Span) -> String {
        span.as_str().to_string()
    }

    #[derive(Debug, FromPest, Serialize, Default, Clone)]
    #[pest_ast(rule(Rule::airlines))]
    pub struct Airlines {
        pub definitions: Vec<Airline>,
        _eoi: Eoi,
    }

    #[derive(Debug, FromPest, Serialize, Default, Clone)]
    #[pest_ast(rule(Rule::EOI))]
    struct Eoi;

    #[derive(Debug, FromPest, Serialize, Default, Clone, PartialEq)]
    #[pest_ast(rule(Rule::definition))]
    pub struct Airline {
        #[pest_ast(inner(rule(Rule::designator), with(span_into_string)))]
        pub designator: String,
        #[pest_ast(inner(rule(Rule::name), with(span_into_string)))]
        pub name: String,
        #[pest_ast(inner(rule(Rule::callsign), with(span_into_string)))]
        pub callsign: String,
        #[pest_ast(inner(rule(Rule::country), with(span_into_string)))]
        pub country: String,
    }
}

#[derive(Error, Debug)]
pub enum AirlinesError {
    #[error("failed to parse ICAO_Airlines.txt: {0}")]
    Parse(#[from] pest::error::Error<airlines::Rule>),
    #[error("failed to read ICAO_Airlines.txt: {0}")]
    FileRead(#[from] io::Error),
}

pub type AirlinesResult = Result<HashMap<String, ast::Airline>, AirlinesError>;

pub fn parse_airlines(content: &[u8]) -> AirlinesResult {
    use pest::Parser;

    let unparsed_file = read_to_string(content)?;
    let mut parse_tree = airlines::Parser::parse(airlines::Rule::airlines, &unparsed_file)?;
    let syntax_tree: ast::Airlines = ast::Airlines::from_pest(&mut parse_tree).expect("infallible");
    Ok(syntax_tree
        .definitions
        .into_iter()
        .fold(HashMap::new(), |mut acc, airline| {
            acc.entry(airline.designator.clone()).or_insert(airline);
            acc
        }))
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use pretty_assertions_sorted::assert_eq_sorted;

    use super::ast::Airline;

    use super::parse_airlines;

    #[test]
    fn test_airlines() {
        let airway_bytes = b"
;========================COMMENT=======================================================================================;
BWW	VATGER RG MUENCHEN	BAVARIAN WEISSWURST	GERMANY
CAP	UNITED STATES AIR FORCE AUXILIARY /  CIVIL AIR PATROL (RICHMOND, VA)	CAP	UNITED STATES
; inline comment
CNW	NORTH-WESTERN CARGO INTERNATIONAL AIRLINES CO.,LTD 	TANG	CHINA
";

        let parsed = parse_airlines(airway_bytes).unwrap();

        assert_eq_sorted!(
            parsed,
            HashMap::from([
                (
                    "BWW".to_string(),
                    Airline {
                        designator: "BWW".to_string(),
                        name: "VATGER RG MUENCHEN".to_string(),
                        callsign: "BAVARIAN WEISSWURST".to_string(),
                        country: "GERMANY".to_string()
                    }
                ),
                (
                    "CAP".to_string(),
                    Airline {
                        designator: "CAP".to_string(),
                        name:
                            "UNITED STATES AIR FORCE AUXILIARY /  CIVIL AIR PATROL (RICHMOND, VA)"
                                .to_string(),
                        callsign: "CAP".to_string(),
                        country: "UNITED STATES".to_string()
                    }
                ),
                (
                    "CNW".to_string(),
                    Airline {
                        designator: "CNW".to_string(),
                        name: "NORTH-WESTERN CARGO INTERNATIONAL AIRLINES CO.,LTD".to_string(),
                        callsign: "TANG".to_string(),
                        country: "CHINA".to_string()
                    }
                ),
            ])
        );
    }
}
