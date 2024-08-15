use pest::Parser;
use pest_derive::Parser;
use std::collections::HashMap;
use std::io;
use thiserror::Error;

use crate::adaptation::icao::Airline;

use super::read_to_string;

#[derive(Parser)]
#[grammar = "pest/base.pest"]
#[grammar = "pest/icao_airlines.pest"]
pub struct AirlineParser;

#[derive(Error, Debug)]
pub enum AirlinesError {
    #[error("failed to parse ICAO_Airlines.txt: {0}")]
    Parse(#[from] pest::error::Error<Rule>),
    #[error("failed to read ICAO_Airlines.txt: {0}")]
    FileRead(#[from] io::Error),
}

pub type AirlinesResult = Result<HashMap<String, Airline>, AirlinesError>;

pub fn parse_airlines(content: &[u8]) -> AirlinesResult {
    let unparsed_file = read_to_string(content)?;
    let airlines_parse = AirlineParser::parse(Rule::airlines, &unparsed_file);

    Ok(airlines_parse.map(|mut pairs| {
        pairs
            .next()
            .unwrap()
            .into_inner()
            .fold(HashMap::new(), |mut acc, pair| {
                if matches!(pair.as_rule(), Rule::definition) {
                    let mut airline_line = pair.into_inner();
                    let designator = airline_line.next().unwrap().as_str().to_string();
                    let airline = airline_line.next().unwrap().as_str().to_string();
                    let callsign = airline_line.next().unwrap().as_str().to_string();
                    let country = airline_line.next().unwrap().as_str().to_string();

                    acc.entry(designator.clone()).or_insert(Airline {
                        designator,
                        airline,
                        callsign,
                        country,
                    });
                }

                acc
            })
    })?)
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use pretty_assertions_sorted::assert_eq_sorted;

    use crate::adaptation::icao::Airline;

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
                        airline: "VATGER RG MUENCHEN".to_string(),
                        callsign: "BAVARIAN WEISSWURST".to_string(),
                        country: "GERMANY".to_string()
                    }
                ),
                (
                    "CAP".to_string(),
                    Airline {
                        designator: "CAP".to_string(),
                        airline:
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
                        airline: "NORTH-WESTERN CARGO INTERNATIONAL AIRLINES CO.,LTD".to_string(),
                        callsign: "TANG".to_string(),
                        country: "CHINA".to_string()
                    }
                ),
            ])
        );
    }
}
