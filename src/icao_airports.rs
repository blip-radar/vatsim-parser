use pest::Parser;
use pest_derive::Parser;
use std::collections::HashMap;
use std::io;
use thiserror::Error;

use crate::adaptation::icao::Airport;

use super::read_to_string;

#[derive(Parser)]
#[grammar = "pest/base.pest"]
#[grammar = "pest/icao_airports.pest"]
pub struct AirportsParser;

#[derive(Error, Debug)]
pub enum AirportsError {
    #[error("failed to parse ICAO_Airports.txt: {0}")]
    Parse(#[from] pest::error::Error<Rule>),
    #[error("failed to read ICAO_Airports.txt: {0}")]
    FileRead(#[from] io::Error),
}

pub type AirportsResult = Result<HashMap<String, Airport>, AirportsError>;

pub fn parse_airports(content: &[u8]) -> AirportsResult {
    let unparsed_file = read_to_string(content)?;
    let aircraft_parse = AirportsParser::parse(Rule::aircraft, &unparsed_file);

    Ok(aircraft_parse.map(|mut pairs| {
        pairs
            .next()
            .unwrap()
            .into_inner()
            .fold(HashMap::new(), |mut acc, pair| {
                if matches!(pair.as_rule(), Rule::definition) {
                    let mut line = pair.into_inner();
                    let designator = line.next().unwrap().as_str().to_string();
                    let name = line.next().unwrap().as_str().to_string();
                    let country = line.next().unwrap().as_str().to_string();

                    acc.entry(designator.clone()).or_insert(Airport {
                        designator,
                        name,
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

    use crate::adaptation::icao::Airport;

    use super::parse_airports;

    #[test]
    fn test_airports() {
        let airports_bytes = b"
;========================COMMENT=======================================================================================;
EBOS	OOSTENDE BRUGGE/OOSTENDE	Belgium
EDDM	MUNICH	Germany
EDGL	BERUFSGENOSSENSCHAFTLICHE UNFALLKLINIK LUDWIGSHAFEN	Germany";

        let parsed = parse_airports(airports_bytes).unwrap();

        assert_eq_sorted!(
            parsed,
            HashMap::from([
                (
                    "EBOS".to_string(),
                    Airport {
                        designator: "EBOS".to_string(),
                        name: "OOSTENDE BRUGGE/OOSTENDE".to_string(),
                        country: "Belgium".to_string(),
                    }
                ),
                (
                    "EDDM".to_string(),
                    Airport {
                        designator: "EDDM".to_string(),
                        name: "MUNICH".to_string(),
                        country: "Germany".to_string(),
                    }
                ),
                (
                    "EDGL".to_string(),
                    Airport {
                        designator: "EDGL".to_string(),
                        name: "BERUFSGENOSSENSCHAFTLICHE UNFALLKLINIK LUDWIGSHAFEN".to_string(),
                        country: "Germany".to_string(),
                    }
                ),
            ])
        );
    }
}
