use geo::{point, Point};
use pest::{iterators::Pair, Parser};
use pest_derive::Parser;
use std::io;
use thiserror::Error;

use crate::adaptation::locations::{
    airways::{AirwayFix, AirwayGraph, AirwayType},
    Fix, GraphPosition,
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
            rule => {
                unreachable!("{rule:?}")
            }
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

pub type AirwayGraphResult = Result<AirwayGraph, AirwayError>;

pub fn parse_airway_txt(content: &[u8]) -> AirwayGraphResult {
    tracing::debug!("PARSING 2");
    let unparsed_file = read_to_string(content)?;
    let airways_parse = AirwayParser::parse(Rule::airways, &unparsed_file);

    Ok(airways_parse.map(|mut pairs| {
        pairs
            .next()
            .unwrap()
            .into_inner()
            .fold(AirwayGraph::default(), |mut acc, pair| {
                if matches!(pair.as_rule(), Rule::airway) {
                    let mut airway_line = pair.into_inner();
                    let fix_name = airway_line.next().unwrap().as_str().to_string();
                    let coordinate = parse_coord(airway_line.next().unwrap());

                    let fix = GraphPosition(coordinate);
                    let airway = airway_line.next().unwrap().as_str();
                    // TODO: parse airway type
                    let _airway_type = AirwayType::parse(&airway_line.next().unwrap());

                    if let Some(previous) = AirwayFix::parse(airway_line.next().unwrap()) {
                        acc.insert_or_update_segment(
                            airway,
                            &fix_name,
                            fix.clone(),
                            &previous.fix.designator,
                            GraphPosition(previous.fix.coordinate),
                            previous.valid_direction,
                            None,
                            previous.minimum_level,
                            None,
                        );
                    }
                    if let Some(next) = AirwayFix::parse(airway_line.next().unwrap()) {
                        acc.insert_or_update_segment(
                            airway,
                            &fix_name,
                            fix,
                            &next.fix.designator,
                            GraphPosition(next.fix.coordinate),
                            next.valid_direction,
                            None,
                            next.minimum_level,
                            None,
                        );
                    }
                }

                acc
            })
    })?)
}
