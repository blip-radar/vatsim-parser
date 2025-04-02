use bevy_derive::{Deref, DerefMut};
use geo::{point, Point};
use itertools::Itertools;
use multimap::MultiMap;
use pest::{iterators::Pair, Parser};
use pest_derive::Parser;
use serde::Serialize;
use std::{fmt::Display, io};
use thiserror::Error;

use crate::adaptation::locations::Fix;

use super::read_to_string;

#[derive(Parser)]
#[grammar = "pest/isec.pest"]
pub struct IsecParser;

#[derive(Clone, Debug, Serialize, Deref, DerefMut)]
pub struct IsecMap(MultiMap<String, Fix>);

impl Display for IsecMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (_, fix) in self
            .flat_iter()
            .sorted_by_key(|(designator, _)| *designator)
        {
            writeln!(
                f,
                "{}\t{:>10}\t{:>11}\t15",
                fix.designator,
                format!("{:.6}", fix.coordinate.y()),
                format!("{:.6}", fix.coordinate.x()),
            )?;
        }

        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum IsecError {
    #[error("failed to parse isec.txt: {0}")]
    Parse(#[from] pest::error::Error<Rule>),
    #[error("failed to read isec.txt: {0}")]
    FileRead(#[from] io::Error),
}

fn parse_coord(pair: Pair<Rule>) -> Point {
    let mut coord = pair.into_inner();
    let lat = coord.next().unwrap().as_str().parse().unwrap();
    let lng = coord.next().unwrap().as_str().parse().unwrap();
    point! { x: lng, y: lat }
}

pub fn parse_isec_txt(content: &[u8]) -> Result<IsecMap, IsecError> {
    let unparsed_file = read_to_string(content)?;
    let airways_parse = IsecParser::parse(Rule::wpts, &unparsed_file);

    Ok(airways_parse.map(|mut pairs| {
        pairs
            .next()
            .unwrap()
            .into_inner()
            .fold(IsecMap(MultiMap::new()), |mut acc, pair| {
                if matches!(pair.as_rule(), Rule::wpt) {
                    let mut isec_line = pair.into_inner();
                    let designator = isec_line.next().unwrap().as_str().to_string();
                    let coordinate = parse_coord(isec_line.next().unwrap());
                    let fix = Fix {
                        designator: designator.clone(),
                        coordinate,
                    };

                    acc.insert(designator, fix);
                }

                acc
            })
    })?)
}

#[cfg(test)]
mod test {
    use geo::point;
    use multimap::MultiMap;
    use pretty_assertions_sorted::assert_eq_sorted;

    use crate::{adaptation::locations::Fix, isec::parse_isec_txt};

    #[test]
    fn test_isec() {
        let isec_str = "8900E	 89.000000	   0.000000	15
8900E	 89.000000	   0.000000	15
ASPAT	 49.196175	  10.725828	15
DEBHI	 49.360833	  10.466111	15
ERNAS	 48.844669	  11.219353	15
GIVMI	 48.701094	  11.364803	15
";

        let parsed = parse_isec_txt(isec_str.as_bytes());

        assert!(parsed.is_ok(), "{}", parsed.unwrap_err());
        assert_eq_sorted!(isec_str, parsed.as_ref().unwrap().to_string());

        assert_eq_sorted!(
            parsed.unwrap().0,
            MultiMap::from_iter([
                (
                    "8900E".to_string(),
                    Fix {
                        designator: "8900E".to_string(),
                        coordinate: point! {
                            x: 0.,
                            y: 89.,
                        },
                    },
                ),
                (
                    "8900E".to_string(),
                    Fix {
                        designator: "8900E".to_string(),
                        coordinate: point! {
                            x: 0.,
                            y: 89.,
                        },
                    },
                ),
                (
                    "ASPAT".to_string(),
                    Fix {
                        designator: "ASPAT".to_string(),
                        coordinate: point! {
                            x: 10.725_828,
                            y: 49.196_175,
                        },
                    },
                ),
                (
                    "DEBHI".to_string(),
                    Fix {
                        designator: "DEBHI".to_string(),
                        coordinate: point! {
                            x: 10.466_111,
                            y: 49.360_833,
                        },
                    },
                ),
                (
                    "ERNAS".to_string(),
                    Fix {
                        designator: "ERNAS".to_string(),
                        coordinate: point! {
                            x: 11.219_353,
                            y: 48.844_669,
                        },
                    },
                ),
                (
                    "GIVMI".to_string(),
                    Fix {
                        designator: "GIVMI".to_string(),
                        coordinate: point! {
                            x: 11.364_803,
                            y: 48.701_094,
                        },
                    },
                )
            ])
        );
    }
}
