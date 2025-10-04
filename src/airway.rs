use geo::{point, Point};
use multimap::MultiMap;
use pest::{iterators::Pair, Parser};
use pest_derive::Parser;
use std::collections::HashMap;
use std::io;
use thiserror::Error;

use crate::adaptation::locations::{
    airways::{AirwayFix, AirwayNeighbours, AirwayNeighboursOfFix, AirwayType, FixAirwayMap},
    Fix,
};

use super::read_to_string;

#[derive(Parser)]
#[grammar = "pest/airway.pest"]
pub struct AirwayParser;

#[derive(Error, Debug)]
pub enum AirwayError {
    #[error("failed to parse airway.txt: {0}")]
    Parse(#[from] pest::error::Error<Rule>),
    #[error("failed to read airway.txt: {0}")]
    FileRead(#[from] io::Error),
}

fn parse_coord(pair: Pair<Rule>) -> Point {
    let mut coord = pair.into_inner();
    let lat = coord.next().unwrap().as_str().parse().unwrap();
    let lng = coord.next().unwrap().as_str().parse().unwrap();
    point! { x: lng, y: lat }
}
fn parse_level(pair: &Pair<Rule>) -> Option<u32> {
    match pair.as_rule() {
        Rule::not_established => None,
        Rule::level => Some(pair.as_str().parse().unwrap()),
        rule => unreachable!("{rule:?}"),
    }
}
impl AirwayFix {
    fn parse(pair: Pair<Rule>) -> Option<Self> {
        match pair.as_rule() {
            Rule::no_neighbour => None,
            Rule::neighbour => {
                let mut airway_fix = pair.into_inner();
                let designator = airway_fix.next().unwrap().as_str().to_string();
                let coordinate = parse_coord(airway_fix.next().unwrap());
                let minimum_level = parse_level(&airway_fix.next().unwrap());
                let valid_direction = airway_fix.next().unwrap().as_str() == "Y";
                Some(AirwayFix {
                    fix: Fix {
                        designator,
                        coordinate,
                    },
                    valid_direction,
                    minimum_level,
                })
            }
            rule => unreachable!("{rule:?}"),
        }
    }
}

impl AirwayType {
    fn parse(pair: &Pair<Rule>) -> Self {
        match pair.as_str() {
            "H" => Self::High,
            "L" => Self::Low,
            "B" => Self::Both,
            "" => Self::Unknown,
            parsed => unreachable!("{parsed}"),
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
            .fold(FixAirwayMap(HashMap::new()), |mut acc, pair| {
                if matches!(pair.as_rule(), Rule::airway) {
                    let mut airway_line = pair.into_inner();
                    let fix_name = airway_line.next().unwrap().as_str().to_string();
                    let coordinate = parse_coord(airway_line.next().unwrap());
                    let fix = Fix {
                        designator: fix_name,
                        coordinate,
                    };
                    let airway = airway_line.next().unwrap().as_str().to_string();
                    let airway_type = AirwayType::parse(&airway_line.next().unwrap());

                    let previous = AirwayFix::parse(airway_line.next().unwrap());
                    let next = AirwayFix::parse(airway_line.next().unwrap());

                    acc.entry(fix.clone())
                        .and_modify(|neighbours: &mut AirwayNeighboursOfFix| {
                            neighbours.airway_neighbours.insert(
                                airway.clone(),
                                AirwayNeighbours {
                                    airway: airway.clone(),
                                    airway_type,
                                    previous: previous.clone(),
                                    next: next.clone(),
                                },
                            );
                        })
                        .or_insert(AirwayNeighboursOfFix {
                            fix,
                            airway_neighbours: MultiMap::from_iter([(
                                airway.clone(),
                                AirwayNeighbours {
                                    airway,
                                    airway_type,
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

    use geo::point;
    use multimap::MultiMap;
    use pretty_assertions_sorted::assert_eq_sorted;

    use crate::{
        adaptation::locations::{airways::AirwayType, Fix},
        airway::{AirwayFix, AirwayNeighbours, AirwayNeighboursOfFix},
    };

    use super::parse_airway_txt;

    #[test]
    fn test_airway() {
        let airway_str = "ASPAT	49.196175	10.725828	14	T161	B	REDNI	49.080000	10.890278	05500	N					N
ASPAT	49.196175	10.725828	14	T161	L					N	DEBHI	49.360833	10.466111	NESTB	Y
DEBHI	49.360833	10.466111	14	T161	L	ASPAT	49.196175	10.725828		N	TOSTU	49.713536	9.805942	05000	Y
ERNAS	48.844669	11.219353	14	T161	B	NIMDI	48.802222	11.633611	05000	N	GOLMO	48.962500	11.055278	05500	Y
ERNAS	48.844669	11.219353	14	Y101	B	GIVMI	48.701094	11.364803	04000	N	TALAL	49.108333	11.085278	05000	Y
GIVMI	48.701094	11.364803	14	Y101	B					N	ERNAS	48.844669	11.219353	04000	Y
GOLMO	48.962500	11.055278	14	T161	B	ERNAS	48.844669	11.219353	05500	N	REDNI	49.080000	10.890278	05500	Y
REDNI	49.080000	10.890278	14	T161		GOLMO	48.962500	11.055278	05500	N	ASPAT	49.196175	10.725828	05500	Y
";

        let parsed = parse_airway_txt(airway_str.as_bytes()).unwrap();

        assert_eq_sorted!(airway_str.replace("NESTB", ""), parsed.to_string());

        assert_eq_sorted!(
            parsed.0,
            HashMap::from([
                (
                    Fix {
                        designator: "ASPAT".to_string(),
                        coordinate: point! {
                            x: 10.725_828,
                            y: 49.196_175,
                        },
                    },
                    AirwayNeighboursOfFix {
                        fix: Fix {
                            designator: "ASPAT".to_string(),
                            coordinate: point! {
                                x: 10.725_828,
                                y: 49.196_175,
                            },
                        },
                        airway_neighbours: MultiMap::from_iter([
                            (
                                "T161".to_owned(),
                                AirwayNeighbours {
                                    airway: "T161".to_string(),
                                    airway_type: AirwayType::Both,
                                    previous: Some(AirwayFix {
                                        fix: Fix {
                                            designator: "REDNI".to_string(),
                                            coordinate: point! {
                                                x: 10.890_278,
                                                y: 49.08,
                                            },
                                        },
                                        valid_direction: false,
                                        minimum_level: Some(5500)
                                    }),
                                    next: None,
                                }
                            ),
                            (
                                "T161".to_owned(),
                                AirwayNeighbours {
                                    airway: "T161".to_string(),
                                    airway_type: AirwayType::Low,
                                    previous: None,
                                    next: Some(AirwayFix {
                                        fix: Fix {
                                            designator: "DEBHI".to_string(),
                                            coordinate: point! {
                                                x: 10.466_111,
                                                y: 49.360_833,
                                            },
                                        },
                                        valid_direction: true,
                                        minimum_level: None
                                    })
                                }
                            )
                        ])
                    }
                ),
                (
                    Fix {
                        designator: "DEBHI".to_string(),
                        coordinate: point! {
                            x: 10.466_111,
                            y: 49.360_833,
                        },
                    },
                    AirwayNeighboursOfFix {
                        fix: Fix {
                            designator: "DEBHI".to_string(),
                            coordinate: point! {
                                x: 10.466_111,
                                y: 49.360_833,
                            },
                        },
                        airway_neighbours: MultiMap::from_iter([(
                            "T161".to_owned(),
                            AirwayNeighbours {
                                airway: "T161".to_string(),
                                airway_type: AirwayType::Low,
                                previous: Some(AirwayFix {
                                    fix: Fix {
                                        designator: "ASPAT".to_string(),
                                        coordinate: point! {
                                            x: 10.725_828,
                                            y: 49.196_175
                                        },
                                    },
                                    valid_direction: false,
                                    minimum_level: None
                                }),
                                next: Some(AirwayFix {
                                    fix: Fix {
                                        designator: "TOSTU".to_string(),
                                        coordinate: point! {
                                            x: 9.805_942,
                                            y: 49.713_536
                                        },
                                    },
                                    valid_direction: true,
                                    minimum_level: Some(5000)
                                })
                            }
                        )])
                    }
                ),
                (
                    Fix {
                        designator: "ERNAS".to_string(),
                        coordinate: point! {
                            x: 11.219_353,
                            y: 48.844_669,
                        },
                    },
                    AirwayNeighboursOfFix {
                        fix: Fix {
                            designator: "ERNAS".to_string(),
                            coordinate: point! {
                                x: 11.219_353,
                                y: 48.844_669,
                            },
                        },
                        airway_neighbours: MultiMap::from_iter([
                            (
                                "T161".to_owned(),
                                AirwayNeighbours {
                                    airway: "T161".to_string(),
                                    airway_type: AirwayType::Both,
                                    previous: Some(AirwayFix {
                                        fix: Fix {
                                            designator: "NIMDI".to_string(),
                                            coordinate: point! {
                                                x: 11.633_611,
                                                y: 48.802_222,
                                            },
                                        },
                                        valid_direction: false,
                                        minimum_level: Some(5000)
                                    }),
                                    next: Some(AirwayFix {
                                        fix: Fix {
                                            designator: "GOLMO".to_string(),
                                            coordinate: point! {
                                                x: 11.055_278,
                                                y: 48.9625
                                            },
                                        },
                                        valid_direction: true,
                                        minimum_level: Some(5500)
                                    })
                                }
                            ),
                            (
                                "Y101".to_owned(),
                                AirwayNeighbours {
                                    airway: "Y101".to_string(),
                                    airway_type: AirwayType::Both,
                                    previous: Some(AirwayFix {
                                        fix: Fix {
                                            designator: "GIVMI".to_string(),
                                            coordinate: point! {
                                                x: 11.364_803,
                                                y: 48.701_094,
                                            },
                                        },
                                        valid_direction: false,
                                        minimum_level: Some(4000)
                                    }),
                                    next: Some(AirwayFix {
                                        fix: Fix {
                                            designator: "TALAL".to_string(),
                                            coordinate: point! {
                                                x: 11.085_278,
                                                y: 49.108_333,
                                            },
                                        },
                                        valid_direction: true,
                                        minimum_level: Some(5000)
                                    })
                                }
                            )
                        ])
                    }
                ),
                (
                    Fix {
                        designator: "GIVMI".to_string(),
                        coordinate: point! {
                            x: 11.364_803,
                            y: 48.701_094,
                        },
                    },
                    AirwayNeighboursOfFix {
                        fix: Fix {
                            designator: "GIVMI".to_string(),
                            coordinate: point! {
                                x: 11.364_803,
                                y: 48.701_094,
                            },
                        },
                        airway_neighbours: MultiMap::from_iter([(
                            "Y101".to_owned(),
                            AirwayNeighbours {
                                airway: "Y101".to_string(),
                                airway_type: AirwayType::Both,
                                previous: None,
                                next: Some(AirwayFix {
                                    fix: Fix {
                                        designator: "ERNAS".to_string(),
                                        coordinate: point! {
                                            x: 11.219_353,
                                            y: 48.844_669,
                                        },
                                    },
                                    valid_direction: true,
                                    minimum_level: Some(4000)
                                })
                            }
                        )])
                    }
                ),
                (
                    Fix {
                        designator: "GOLMO".to_string(),
                        coordinate: point! {
                            x: 11.055_278,
                            y: 48.9625,
                        },
                    },
                    AirwayNeighboursOfFix {
                        fix: Fix {
                            designator: "GOLMO".to_string(),
                            coordinate: point! {
                                x: 11.055_278,
                                y: 48.9625,
                            },
                        },
                        airway_neighbours: MultiMap::from_iter([(
                            "T161".to_owned(),
                            AirwayNeighbours {
                                airway: "T161".to_string(),
                                airway_type: AirwayType::Both,
                                previous: Some(AirwayFix {
                                    fix: Fix {
                                        designator: "ERNAS".to_string(),
                                        coordinate: point! {
                                            x: 11.219_353,
                                            y: 48.844_669,
                                        },
                                    },
                                    valid_direction: false,
                                    minimum_level: Some(5500)
                                }),
                                next: Some(AirwayFix {
                                    fix: Fix {
                                        designator: "REDNI".to_string(),
                                        coordinate: point! {
                                            x: 10.890_278,
                                            y: 49.08,
                                        },
                                    },
                                    valid_direction: true,
                                    minimum_level: Some(5500)
                                })
                            }
                        )])
                    }
                ),
                (
                    Fix {
                        designator: "REDNI".to_string(),
                        coordinate: point! {
                            x: 10.890_278,
                            y: 49.08,
                        },
                    },
                    AirwayNeighboursOfFix {
                        fix: Fix {
                            designator: "REDNI".to_string(),
                            coordinate: point! {
                                x: 10.890_278,
                                y: 49.08,
                            },
                        },

                        airway_neighbours: MultiMap::from_iter([(
                            "T161".to_owned(),
                            AirwayNeighbours {
                                airway: "T161".to_string(),
                                airway_type: AirwayType::Unknown,
                                previous: Some(AirwayFix {
                                    fix: Fix {
                                        designator: "GOLMO".to_string(),
                                        coordinate: point! {
                                            x: 11.055_278,
                                            y: 48.9625,
                                        },
                                    },
                                    valid_direction: false,
                                    minimum_level: Some(5500)
                                }),
                                next: Some(AirwayFix {
                                    fix: Fix {
                                        designator: "ASPAT".to_string(),
                                        coordinate: point! {
                                            x: 10.725_828,
                                            y: 49.196_175,
                                        },
                                    },
                                    valid_direction: true,
                                    minimum_level: Some(5500)
                                })
                            }
                        )])
                    }
                )
            ])
        );
    }
}
