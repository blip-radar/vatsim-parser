use std::collections::HashMap;
use std::io;

use pest::{iterators::Pair, Parser};
use pest_derive::Parser;
use serde::Serialize;
use thiserror::Error;

use super::{read_to_string, Coordinate, DegMinSec};

#[derive(Parser)]
#[grammar = "ese.pest"]
pub struct EseParser;

#[derive(Error, Debug)]
pub enum EseError {
    #[error("failed to parse .ese file: {0:?}")]
    Parse(#[from] pest::error::Error<Rule>),
    #[error("failed to read .ese file: {0:?}")]
    FileRead(#[from] io::Error),
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Position {
    pub name: String,
    pub callsign: String,
    pub frequency: String,
    pub identifier: String,
    pub prefix: String,
    pub middle: String,
    pub suffix: String,
    pub squawk_range: (u16, u16),
    pub vis_points: Vec<Coordinate>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Ese {
    pub positions: HashMap<String, Position>,
}

pub type EseResult = Result<Ese, EseError>;

#[derive(Debug)]
enum Section {
    Positions(HashMap<String, Position>),
    Unsupported,
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum SectionName {
    Position,
    Unsupported,
}

fn parse_coordinate_part(pair: Pair<Rule>) -> DegMinSec {
    let mut coordinate_part = pair.into_inner();
    let hemi = coordinate_part.next().unwrap().as_str();
    let degrees = coordinate_part
        .next()
        .unwrap()
        .as_str()
        .parse::<f64>()
        .unwrap()
        * match hemi {
            "N" | "E" => 1.0,
            "S" | "W" => -1.0,
            _ => unreachable!(),
        };
    let min = coordinate_part.next().unwrap().as_str().parse().unwrap();
    let sec = coordinate_part.next().unwrap().as_str().parse().unwrap();

    (degrees, min, sec)
}

fn parse_coordinate(pair: Pair<Rule>) -> Coordinate {
    let mut coordinate = pair.into_inner();
    let lat = parse_coordinate_part(coordinate.next().unwrap());
    let lng = parse_coordinate_part(coordinate.next().unwrap());
    Coordinate::from_deg_min_sec(lat, lng)
}

fn parse_position(pair: Pair<Rule>) -> (String, Position) {
    let mut position = pair.into_inner();
    let name = position.next().unwrap().as_str().to_string();
    let callsign = position.next().unwrap().as_str().to_string();
    let frequency = position.next().unwrap().as_str().to_string();
    let identifier = position.next().unwrap().as_str().to_string();
    let middle = position.next().unwrap().as_str().to_string();
    let prefix = position.next().unwrap().as_str().to_string();
    let suffix = position.next().unwrap().as_str().to_string();
    // skip unused fields
    let mut position = position.skip(2);
    let squawk_begin = position.next().unwrap().as_str().parse().unwrap();
    let squawk_end = position.next().unwrap().as_str().parse().unwrap();
    let vis_points = position.map(parse_coordinate).collect();

    (
        identifier.clone(),
        Position {
            name,
            callsign,
            frequency,
            identifier,
            prefix,
            middle,
            suffix,
            squawk_range: (squawk_begin, squawk_end),
            vis_points,
        },
    )
}

fn parse_section(pair: Pair<Rule>) -> (SectionName, Section) {
    match pair.as_rule() {
        Rule::position_section => (
            SectionName::Position,
            Section::Positions(pair.into_inner().map(parse_position).collect()),
        ),
        _ => (SectionName::Unsupported, Section::Unsupported),
    }
}

impl Ese {
    pub fn parse(content: &[u8]) -> EseResult {
        let unparsed_file = read_to_string(content)?;
        let mut sections = EseParser::parse(Rule::ese, &unparsed_file).map(|mut pairs| {
            pairs
                .next()
                .unwrap()
                .into_inner()
                .map(parse_section)
                .collect::<HashMap<_, _>>()
        })?;
        let positions = match sections.remove_entry(&SectionName::Position) {
            Some((_, Section::Positions(positions))) => positions,
            _ => HashMap::new(),
        };

        Ok(Ese { positions })
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::{
        ese::{Ese, Position},
        Coordinate,
    };

    #[test]
    fn test_ese() {
        let ese_bytes = b"
[POSITIONS]
EDDM_ATIS:Muenchen ATIS:123.130:MX::EDDM:ATIS:::0000:0000
EDMM_TEG_CTR:Muenchen Radar:133.680:TEG:TEG:EDMM:CTR:::2354:2367:N048.10.49.419:E011.48.59.530
EDXX_FIS_CTR:Langen Information:128.950:GIXX:FIS:EDXX:CTR:::2001:2577:N049.26.51.334:E010.13.06.336:N052.28.08.891:E010.52.12.796

[SIDSSTARS]
STAR:EDDN:28:UPALA1V:UPALA DN463 DN462 DN461 DN452 DN453 DN454 DN455 DN456 DN457 DN458 DN459 DN439 DN438 DN437 OSNUB NGD32
STAR:EDMO:22:MAHxRNP:MAH MO220 MO221 EDIMO
SID:EDDM:26R:GIVMI1N:DM060 DM063 GIVMI
        ";
        let ese = Ese::parse(ese_bytes);
        assert_eq!(
            ese.as_ref().unwrap().positions,
            HashMap::from([
                (
                    "MX".to_string(),
                    Position {
                        name: "EDDM_ATIS".to_string(),
                        callsign: "Muenchen ATIS".to_string(),
                        frequency: "123.130".to_string(),
                        identifier: "MX".to_string(),
                        prefix: "EDDM".to_string(),
                        middle: "".to_string(),
                        suffix: "ATIS".to_string(),
                        squawk_range: (0, 0),
                        vis_points: vec![],
                    }
                ),
                (
                    "TEG".to_string(),
                    Position {
                        name: "EDMM_TEG_CTR".to_string(),
                        callsign: "Muenchen Radar".to_string(),
                        frequency: "133.680".to_string(),
                        identifier: "TEG".to_string(),
                        prefix: "EDMM".to_string(),
                        middle: "TEG".to_string(),
                        suffix: "CTR".to_string(),
                        squawk_range: (2354, 2367),
                        vis_points: vec![Coordinate {
                            lat: 48.180394166666666,
                            lng: 11.816536111111112
                        }],
                    }
                ),
                (
                    "GIXX".to_string(),
                    Position {
                        name: "EDXX_FIS_CTR".to_string(),
                        callsign: "Langen Information".to_string(),
                        frequency: "128.950".to_string(),
                        identifier: "GIXX".to_string(),
                        prefix: "EDXX".to_string(),
                        middle: "FIS".to_string(),
                        suffix: "CTR".to_string(),
                        squawk_range: (2001, 2577),
                        vis_points: vec![
                            Coordinate {
                                lat: 49.44759277777777,
                                lng: 10.218426666666668
                            },
                            Coordinate {
                                lat: 52.46913638888889,
                                lng: 10.870221111111112
                            }
                        ],
                    }
                )
            ])
        );
    }
}
