use std::collections::HashMap;
use std::io;

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use pest::{iterators::Pair, Parser};
use pest_derive::Parser;
use serde::Serialize;
use thiserror::Error;

use crate::{
    adaptation::{colours::Colour, symbols::SymbolRule},
    TwoKeyMap,
};

use super::read_to_string;

#[derive(Parser)]
#[grammar = "pest/base.pest"]
#[grammar = "pest/symbol_rule.pest"]
#[grammar = "pest/symbology.pest"]
pub struct SymbologyParser;

#[derive(Error, Debug)]
pub enum SymbologyError {
    #[error("failed to parse Symbology.txt: {0}")]
    Parse(#[from] pest::error::Error<Rule>),
    #[error("failed to read Symbology.txt file: {0}")]
    FileRead(#[from] io::Error),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Item {
    pub folder: String,
    pub name: String,
    pub colour: Colour,
    pub font_size: f64,
    // TODO
    // line_weight,
    // line_style,
    // text_alignment,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, FromPrimitive, Serialize)]
pub enum SymbolType {
    Airport,
    NDB,
    VOR,
    Fix,
    AircraftStandby,
    AircraftPrimaryOnly,
    AircraftCorrAlphaCharlieSecondaryOnly,
    AircraftCorrModeSierraSecondaryOnly,
    AircraftCorrModeAlphaCharlie,
    AircraftCorrModeSierra,
    AircraftCorrModeAlphaCharlieIdent,
    AircraftCorrModeSierraIdent,
    AircraftFlightPlanTrack,
    AircraftCoasting,
    HistoryDot,
    GroundAircraft,
    AircraftUncorrModeAlphaCharlieSecondaryOnly,
    AircraftUncorrModeSierraSecondaryOnly,
    AircraftUncorrModeAlphaCharlie,
    AircraftUncorrModeSierra,
    AircraftUncorrModeAlphaCharlieIdent,
    AircraftUncorrModeSierraIdent,
    GroundVehicle,
    GroundRotorcraft,
}

#[derive(Debug, Clone, Serialize)]
pub struct Symbology {
    pub items: TwoKeyMap<String, String, Item>,
    pub symbols: HashMap<SymbolType, Vec<SymbolRule>>,
}

pub type SymbologyResult = Result<Symbology, SymbologyError>;

impl Item {
    fn parse(pair: Pair<Rule>) -> Self {
        let mut item = pair.into_inner();
        let folder = item.next().unwrap().as_str().to_string();
        let name = item.next().unwrap().as_str().to_string();
        let colour_str = item.next().unwrap().as_str();
        let colour_num = colour_str.parse::<i32>().unwrap();
        let font_size = item.next().unwrap().as_str().parse().unwrap();

        Self {
            folder,
            name,
            colour: Colour::from_euroscope(colour_num),
            font_size,
        }
    }
}

fn parse_point(pair: Pair<Rule>) -> (f64, f64) {
    let mut point = pair.into_inner();
    let x = point.next().unwrap().as_str().parse().unwrap();
    let y = point.next().unwrap().as_str().parse().unwrap();
    (x, y)
}

// TODO similar to topsky symbol parsing, generalise?
fn parse_symbol_rules(pair: Pair<Rule>) -> Option<(SymbolType, Vec<SymbolRule>)> {
    let mut symbol = pair.into_inner();
    let maybe_symbol_type = SymbolType::from_u32(symbol.next().unwrap().as_str().parse().unwrap());
    let symbol_rules = symbol
        .map(|pair| {
            let ruletype = pair.as_rule();
            let mut symbolrule = pair.into_inner();
            match ruletype {
                Rule::moveto => SymbolRule::Move(parse_point(symbolrule.next().unwrap())),
                Rule::line => SymbolRule::Line(parse_point(symbolrule.next().unwrap())),
                Rule::pixel => SymbolRule::Pixel(parse_point(symbolrule.next().unwrap())),
                Rule::arc => {
                    let pos = parse_point(symbolrule.next().unwrap());
                    let radius = symbolrule.next().unwrap().as_str().parse().unwrap();
                    let start_angle =
                        symbolrule.next().unwrap().as_str().parse::<i64>().unwrap() % 360;
                    let end_angle =
                        symbolrule.next().unwrap().as_str().parse::<i64>().unwrap() % 360;
                    SymbolRule::Arc(pos, radius, start_angle, end_angle)
                }
                Rule::arc_ellipse => {
                    let pos = parse_point(symbolrule.next().unwrap());
                    let radius_x = symbolrule.next().unwrap().as_str().parse().unwrap();
                    let radius_y = symbolrule.next().unwrap().as_str().parse().unwrap();
                    let start_angle =
                        symbolrule.next().unwrap().as_str().parse::<i64>().unwrap() % 360;
                    let end_angle =
                        symbolrule.next().unwrap().as_str().parse::<i64>().unwrap() % 360;
                    SymbolRule::EllipticArc(pos, radius_x, radius_y, start_angle, end_angle)
                }
                Rule::fillarc => {
                    let pos = parse_point(symbolrule.next().unwrap());
                    let radius = symbolrule.next().unwrap().as_str().parse().unwrap();
                    let start_angle =
                        symbolrule.next().unwrap().as_str().parse::<i64>().unwrap() % 360;
                    let end_angle =
                        symbolrule.next().unwrap().as_str().parse::<i64>().unwrap() % 360;
                    SymbolRule::FilledArc(pos, radius, start_angle, end_angle)
                }
                Rule::fillarc_ellipse => {
                    let pos = parse_point(symbolrule.next().unwrap());
                    let radius_x = symbolrule.next().unwrap().as_str().parse().unwrap();
                    let radius_y = symbolrule.next().unwrap().as_str().parse().unwrap();
                    let start_angle =
                        symbolrule.next().unwrap().as_str().parse::<i64>().unwrap() % 360;
                    let end_angle =
                        symbolrule.next().unwrap().as_str().parse::<i64>().unwrap() % 360;
                    SymbolRule::FilledEllipticArc(pos, radius_x, radius_y, start_angle, end_angle)
                }
                Rule::ellipse_circle => {
                    let pos = parse_point(symbolrule.next().unwrap());
                    let radius = symbolrule.next().unwrap().as_str().parse().unwrap();
                    SymbolRule::FilledArc(pos, radius, 0, 0)
                }
                Rule::ellipse => {
                    let pos = parse_point(symbolrule.next().unwrap());
                    let radius_x = symbolrule.next().unwrap().as_str().parse().unwrap();
                    let radius_y = symbolrule.next().unwrap().as_str().parse().unwrap();
                    SymbolRule::FilledEllipticArc(pos, radius_x, radius_y, 0, 0)
                }
                Rule::fillrect => {
                    let (x1, y1) = parse_point(symbolrule.next().unwrap());
                    let (x2, y2) = parse_point(symbolrule.next().unwrap());
                    SymbolRule::Polygon(vec![(x1, y1), (x2, y1), (x2, y2), (x1, y2)])
                }
                Rule::polygon => SymbolRule::Polygon(symbolrule.map(parse_point).collect()),
                rule => unreachable!("{rule:?}"),
            }
        })
        .collect();
    maybe_symbol_type.map(|symbol_type| (symbol_type, symbol_rules))
}

impl Symbology {
    pub fn parse(content: &[u8]) -> SymbologyResult {
        let unparsed_file = read_to_string(content)?;
        let (items, symbols) =
            SymbologyParser::parse(Rule::symbology, &unparsed_file).map(|mut pairs| {
                pairs.next().unwrap().into_inner().fold(
                    (HashMap::new(), HashMap::new()),
                    |(mut items, mut symbols), pair| {
                        match pair.as_rule() {
                            Rule::item => {
                                let item = Item::parse(pair);
                                items.insert((item.folder.clone(), item.name.clone()), item);
                            }
                            Rule::symbol => {
                                if let Some((symbol_type, symbol_rules)) = parse_symbol_rules(pair)
                                {
                                    symbols.insert(symbol_type, symbol_rules);
                                }
                            }
                            Rule::header | Rule::footer | Rule::EOI => (),
                            rule => unreachable!("unhandled {rule:?}"),
                        }
                        (items, symbols)
                    },
                )
            })?;

        Ok(Symbology {
            items: TwoKeyMap(items),
            symbols,
        })
    }
}

#[cfg(test)]
mod test {

    use crate::{
        adaptation::{colours::Colour, symbols::SymbolRule},
        symbology::{Item, SymbolType, Symbology},
    };

    #[test]
    fn test_symbology() {
        let symbology_bytes = br"
SYMBOLOGY
SYMBOLSIZE
Sector:msaw:32768:2.0:0:2:7
Sector:inactive sector background:13158600:3.5:0:0:7
SYMBOL:0
SYMBOLITEM:MOVETO -3 -3
SYMBOLITEM:LINETO 3 -3
SYMBOLITEM:LINETO 3 3
SYMBOLITEM:LINETO -3 3
SYMBOLITEM:LINETO -3 -3
SYMBOLITEM:MOVETO 5 0
SYMBOLITEM:LINETO -6 0
SYMBOLITEM:MOVETO 0 5
SYMBOLITEM:LINETO 0 -6
SYMBOL:1
SYMBOLITEM:MOVETO -4 3
SYMBOLITEM:LINETO 0 -4
SYMBOLITEM:LINETO 4 3
SYMBOLITEM:LINETO -4 3
m_ClipArea:0
END
        ";
        let symbology = Symbology::parse(symbology_bytes);
        assert_eq!(
            symbology
                .as_ref()
                .unwrap()
                .items
                .0
                .get(&("Sector".to_string(), "msaw".to_string())),
            Some(&Item {
                folder: "Sector".to_string(),
                name: "msaw".to_string(),
                colour: Colour::from_rgb(0, 128, 0),
                font_size: 2.0
            })
        );
        assert_eq!(
            symbology.as_ref().unwrap().items.0.get(&(
                "Sector".to_string(),
                "inactive sector background".to_string()
            )),
            Some(&Item {
                folder: "Sector".to_string(),
                name: "inactive sector background".to_string(),
                colour: Colour::from_rgb(200, 200, 200),
                font_size: 3.5
            })
        );
        assert_eq!(
            symbology
                .as_ref()
                .unwrap()
                .symbols
                .get(&SymbolType::Airport),
            Some(&vec![
                SymbolRule::Move((-3.0, -3.0)),
                SymbolRule::Line((3.0, -3.0)),
                SymbolRule::Line((3.0, 3.0)),
                SymbolRule::Line((-3.0, 3.0)),
                SymbolRule::Line((-3.0, -3.0)),
                SymbolRule::Move((5.0, 0.0)),
                SymbolRule::Line((-6.0, 0.0)),
                SymbolRule::Move((0.0, 5.0)),
                SymbolRule::Line((0.0, -6.0)),
            ])
        );
        assert_eq!(
            symbology.as_ref().unwrap().symbols.get(&SymbolType::NDB),
            Some(&vec![
                SymbolRule::Move((-4.0, 3.0)),
                SymbolRule::Line((0.0, -4.0)),
                SymbolRule::Line((4.0, 3.0)),
                SymbolRule::Line((-4.0, 3.0)),
            ])
        );
    }
}
