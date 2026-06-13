use geo::Point;
use pest::Parser;
use pest_derive::Parser;
use std::collections::HashMap;
use std::io;
use thiserror::Error;

use super::read_to_string;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct NavdataAirport {
    pub designator: String,
    pub name: String,
    pub location: Point,
}

#[derive(Parser)]
#[grammar = "pest/base.pest"]
#[grammar = "pest/navdata_airports.pest"]
pub struct NavdataAirportsParser;

#[derive(Error, Debug)]
pub enum NavdataAirportsError {
    #[error("failed to parse navdata airports: {0}")]
    Parse(#[from] pest::error::Error<Rule>),
    #[error("failed to read navdata airports: {0}")]
    FileRead(#[from] io::Error),
}

pub type NavdataAirportsResult = Result<HashMap<String, NavdataAirport>, NavdataAirportsError>;

pub fn parse_navdata_airports(content: &[u8]) -> NavdataAirportsResult {
    let unparsed_file = read_to_string(content)?;
    let airports_parse = NavdataAirportsParser::parse(Rule::airports, &unparsed_file);

    Ok(airports_parse.map(|mut pairs| {
        pairs
            .next()
            .unwrap()
            .into_inner()
            .fold(HashMap::new(), |mut acc, pair| {
                if matches!(pair.as_rule(), Rule::definition) {
                    let mut line = pair.into_inner();
                    let designator = line.next().unwrap().as_str().to_string();
                    let lat: f64 = line.next().unwrap().as_str().parse().unwrap();
                    let lon: f64 = line.next().unwrap().as_str().parse().unwrap();
                    let name = line.next().unwrap().as_str().to_string();

                    acc.entry(designator.clone()).or_insert(NavdataAirport {
                        designator,
                        name,
                        location: Point::new(lon, lat),
                    });
                }

                acc
            })
    })?)
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use geo::Point;
    use pretty_assertions_sorted::assert_eq_sorted;

    use super::{parse_navdata_airports, NavdataAirport};

    #[test]
    fn test_navdata_airports() {
        let content = b"EDDB    52.3622470    13.5006720    BERLIN BRANDENBURG
EDDC    51.1343440    13.7680000    DRESDEN
EDDE    50.9798110    10.9581060    ERFURT-WEIMAR
EDDF    50.0333060    8.5704560    FRANKFURT/MAIN
EDDG    52.1346420    7.6848310    MUENSTER/OSNABRUECK";

        let parsed = parse_navdata_airports(content).unwrap();

        assert_eq_sorted!(
            parsed,
            HashMap::from([
                (
                    "EDDB".to_string(),
                    NavdataAirport {
                        designator: "EDDB".to_string(),
                        name: "BERLIN BRANDENBURG".to_string(),
                        location: Point::new(13.500_672, 52.362_247),
                    }
                ),
                (
                    "EDDC".to_string(),
                    NavdataAirport {
                        designator: "EDDC".to_string(),
                        name: "DRESDEN".to_string(),
                        location: Point::new(13.768, 51.134_344),
                    }
                ),
                (
                    "EDDE".to_string(),
                    NavdataAirport {
                        designator: "EDDE".to_string(),
                        name: "ERFURT-WEIMAR".to_string(),
                        location: Point::new(10.958_106, 50.979_811),
                    }
                ),
                (
                    "EDDF".to_string(),
                    NavdataAirport {
                        designator: "EDDF".to_string(),
                        name: "FRANKFURT/MAIN".to_string(),
                        location: Point::new(8.570_456, 50.033_306),
                    }
                ),
                (
                    "EDDG".to_string(),
                    NavdataAirport {
                        designator: "EDDG".to_string(),
                        name: "MUENSTER/OSNABRUECK".to_string(),
                        location: Point::new(7.684_831, 52.134_642),
                    }
                ),
            ])
        );
    }
}
