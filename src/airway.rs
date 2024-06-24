use geo_types::Coord;
use pest::{iterators::Pair, Parser};
use pest_derive::Parser;
use std::collections::HashMap;
use std::io;
use thiserror::Error;

use crate::adaptation::locations::airways::{
    AirwayFix, AirwayNeighbours, AirwayNeighboursOfFix, FixAirwayMap,
};

use super::read_to_string;

#[derive(Parser)]
#[grammar = "airway.pest"]
pub struct AirwayParser;

#[derive(Error, Debug)]
pub enum AirwayError {
    #[error("failed to parse airway.txt: {0:?}")]
    Parse(#[from] pest::error::Error<Rule>),
    #[error("failed to read airway.txt: {0:?}")]
    FileRead(#[from] io::Error),
}

fn parse_coord(pair: Pair<Rule>) -> Coord {
    let mut coord = pair.into_inner();
    let lat = coord.next().unwrap().as_str().parse().unwrap();
    let lng = coord.next().unwrap().as_str().parse().unwrap();
    Coord { x: lng, y: lat }
}
impl AirwayFix {
    fn parse(pair: Pair<Rule>) -> Option<Self> {
        match pair.as_rule() {
            Rule::no_neighbour => None,
            Rule::neighbour => {
                let mut airway_fix = pair.into_inner();
                let name = airway_fix.next().unwrap().as_str().to_string();
                let coord = parse_coord(airway_fix.next().unwrap());
                let minimum_level = airway_fix.next().unwrap().as_str().parse().unwrap();
                let valid_direction = airway_fix.next().unwrap().as_str() == "Y";
                Some(AirwayFix {
                    name,
                    coord,
                    minimum_level,
                    valid_direction,
                })
            }
            rule => unreachable!("{rule:?}"),
        }
    }
}

pub type FixAirwayResult = Result<FixAirwayMap, AirwayError>;

pub fn parse_airway_txt(content: &[u8]) -> FixAirwayResult {
    let unparsed_file = read_to_string(content)?;
    let airways_parse = AirwayParser::parse(Rule::airways, &unparsed_file);

    Ok(airways_parse.map(|mut pairs| {
        pairs
            .next()
            .unwrap()
            .into_inner()
            .fold(HashMap::new(), |mut acc, pair| {
                if matches!(pair.as_rule(), Rule::airway) {
                    let mut airway_line = pair.into_inner();
                    let fix = airway_line.next().unwrap().as_str().to_string();
                    let _coord = airway_line.next().unwrap().as_str().to_string();
                    let airway = airway_line.next().unwrap().as_str().to_string();

                    let previous = AirwayFix::parse(airway_line.next().unwrap());
                    let next = AirwayFix::parse(airway_line.next().unwrap());

                    acc.entry(fix.clone())
                        .and_modify(|neighbours: &mut AirwayNeighboursOfFix| {
                            neighbours
                                .airway_neighbours
                                .entry(airway.clone())
                                .and_modify(|existing| {
                                    if existing.previous.is_some() && previous.is_some()
                                        || existing.next.is_some() && next.is_some()
                                    {
                                        eprintln!(
                                            "Duplicate for airway {} and fix {}",
                                            airway, fix
                                        );
                                    }
                                    existing.previous =
                                        existing.previous.clone().or(previous.clone());
                                    existing.next = existing.next.clone().or(next.clone());
                                })
                                .or_insert(AirwayNeighbours {
                                    airway: airway.clone(),
                                    previous: previous.clone(),
                                    next: next.clone(),
                                });
                        })
                        .or_insert(AirwayNeighboursOfFix {
                            fix,
                            airway_neighbours: HashMap::from([(
                                airway.clone(),
                                AirwayNeighbours {
                                    airway,
                                    previous,
                                    next,
                                },
                            )]),
                        });
                }

                acc
            })
    })?)
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use geo_types::Coord;
    use pretty_assertions_sorted::assert_eq_sorted;

    use crate::airway::{AirwayFix, AirwayNeighbours, AirwayNeighboursOfFix};

    use super::parse_airway_txt;

    #[test]
    fn test_airway() {
        let airway_bytes = b"
ASPAT	49.196175	10.725828	14	T161	B	REDNI	49.080000	10.890278	05500	N					N
ASPAT	49.196175	10.725828	14	T161	L					N	DEBHI	49.360833	10.466111	05500	Y
DEBHI	49.360833	10.466111	14	T161	L	ASPAT	49.196175	10.725828	05500	N	TOSTU	49.713536	9.805942	05000	Y
ERNAS	48.844669	11.219353	14	T161	B	NIMDI	48.802222	11.633611	05000	N	GOLMO	48.962500	11.055278	05500	Y
ERNAS	48.844669	11.219353	14	Y101	B	GIVMI	48.701094	11.364803	04000	N	TALAL	49.108333	11.085278	05000	Y
GIVMI	48.701094	11.364803	14	Y101	B					N	ERNAS	48.844669	11.219353	04000	Y
GOLMO	48.962500	11.055278	14	T161	B	ERNAS	48.844669	11.219353	05500	N	REDNI	49.080000	10.890278	05500	Y
REDNI	49.080000	10.890278	14	T161	B	GOLMO	48.962500	11.055278	05500	N	ASPAT	49.196175	10.725828	05500	Y
";

        let parsed = parse_airway_txt(airway_bytes).unwrap();

        assert_eq_sorted!(
            parsed,
            HashMap::from([
                (
                    "ASPAT".to_string(),
                    AirwayNeighboursOfFix {
                        fix: "ASPAT".to_string(),
                        airway_neighbours: HashMap::from([(
                            "T161".to_owned(),
                            AirwayNeighbours {
                                airway: "T161".to_string(),
                                previous: Some(AirwayFix {
                                    name: "REDNI".to_string(),
                                    coord: Coord {
                                        x: 10.890278,
                                        y: 49.08
                                    },
                                    valid_direction: false,
                                    minimum_level: 5500
                                }),
                                next: Some(AirwayFix {
                                    name: "DEBHI".to_string(),
                                    coord: Coord {
                                        x: 10.466111,
                                        y: 49.360833,
                                    },
                                    valid_direction: true,
                                    minimum_level: 5500
                                })
                            }
                        )])
                    }
                ),
                (
                    "DEBHI".to_string(),
                    AirwayNeighboursOfFix {
                        fix: "DEBHI".to_string(),
                        airway_neighbours: HashMap::from([(
                            "T161".to_owned(),
                            AirwayNeighbours {
                                airway: "T161".to_string(),
                                previous: Some(AirwayFix {
                                    name: "ASPAT".to_string(),
                                    coord: Coord {
                                        x: 10.725828,
                                        y: 49.196175
                                    },
                                    valid_direction: false,
                                    minimum_level: 5500
                                }),
                                next: Some(AirwayFix {
                                    name: "TOSTU".to_string(),
                                    coord: Coord {
                                        x: 9.805942,
                                        y: 49.713536
                                    },
                                    valid_direction: true,
                                    minimum_level: 5000
                                })
                            }
                        )])
                    }
                ),
                (
                    "ERNAS".to_string(),
                    AirwayNeighboursOfFix {
                        fix: "ERNAS".to_string(),
                        airway_neighbours: HashMap::from([
                            (
                                "T161".to_owned(),
                                AirwayNeighbours {
                                    airway: "T161".to_string(),
                                    previous: Some(AirwayFix {
                                        name: "NIMDI".to_string(),
                                        coord: Coord {
                                            x: 11.633611,
                                            y: 48.802222
                                        },
                                        valid_direction: false,
                                        minimum_level: 5000
                                    }),
                                    next: Some(AirwayFix {
                                        name: "GOLMO".to_string(),
                                        coord: Coord {
                                            x: 11.055278,
                                            y: 48.9625
                                        },
                                        valid_direction: true,
                                        minimum_level: 5500
                                    })
                                }
                            ),
                            (
                                "Y101".to_owned(),
                                AirwayNeighbours {
                                    airway: "Y101".to_string(),
                                    previous: Some(AirwayFix {
                                        name: "GIVMI".to_string(),
                                        coord: Coord {
                                            x: 11.364803,
                                            y: 48.701094
                                        },
                                        valid_direction: false,
                                        minimum_level: 4000
                                    }),
                                    next: Some(AirwayFix {
                                        name: "TALAL".to_string(),
                                        coord: Coord {
                                            x: 11.085278,
                                            y: 49.108333
                                        },
                                        valid_direction: true,
                                        minimum_level: 5000
                                    })
                                }
                            )
                        ])
                    }
                ),
                (
                    "GIVMI".to_string(),
                    AirwayNeighboursOfFix {
                        fix: "GIVMI".to_string(),
                        airway_neighbours: HashMap::from([(
                            "Y101".to_owned(),
                            AirwayNeighbours {
                                airway: "Y101".to_string(),
                                previous: None,
                                next: Some(AirwayFix {
                                    name: "ERNAS".to_string(),
                                    coord: Coord {
                                        x: 11.219353,
                                        y: 48.844669
                                    },
                                    valid_direction: true,
                                    minimum_level: 4000
                                })
                            }
                        )])
                    }
                ),
                (
                    "GOLMO".to_string(),
                    AirwayNeighboursOfFix {
                        fix: "GOLMO".to_string(),
                        airway_neighbours: HashMap::from([(
                            "T161".to_owned(),
                            AirwayNeighbours {
                                airway: "T161".to_string(),
                                previous: Some(AirwayFix {
                                    name: "ERNAS".to_string(),
                                    coord: Coord {
                                        x: 11.219353,
                                        y: 48.844669
                                    },
                                    valid_direction: false,
                                    minimum_level: 5500
                                }),
                                next: Some(AirwayFix {
                                    name: "REDNI".to_string(),
                                    coord: Coord {
                                        x: 10.890278,
                                        y: 49.08
                                    },
                                    valid_direction: true,
                                    minimum_level: 5500
                                })
                            }
                        )])
                    }
                ),
                (
                    "REDNI".to_string(),
                    AirwayNeighboursOfFix {
                        fix: "REDNI".to_string(),
                        airway_neighbours: HashMap::from([(
                            "T161".to_owned(),
                            AirwayNeighbours {
                                airway: "T161".to_string(),
                                previous: Some(AirwayFix {
                                    name: "GOLMO".to_string(),
                                    coord: Coord {
                                        x: 11.055278,
                                        y: 48.9625
                                    },
                                    valid_direction: false,
                                    minimum_level: 5500
                                }),
                                next: Some(AirwayFix {
                                    name: "ASPAT".to_string(),
                                    coord: Coord {
                                        x: 10.725828,
                                        y: 49.196175
                                    },
                                    valid_direction: true,
                                    minimum_level: 5500
                                })
                            }
                        )])
                    }
                )
            ])
        )
    }
}
