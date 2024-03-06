use std::collections::HashMap;

use bevy_reflect::Reflect;
use pest::{
    iterators::{Pair, Pairs},
    Parser,
};
use serde::Serialize;

use crate::read_to_string;

use super::{parse_point, Rule, TopskyError, TopskyParser};

#[derive(Clone, Debug, PartialEq, Reflect, Serialize)]
pub enum SymbolRule {
    Move((f64, f64)),
    Line((f64, f64)),
    Pixel((f64, f64)),
    Arc((f64, f64), f64, i64, i64),
    EllipticArc((f64, f64), f64, f64, i64, i64),
    FilledArc((f64, f64), f64, i64, i64),
    FilledEllipticArc((f64, f64), f64, f64, i64, i64),
    Polygon(Vec<(f64, f64)>),
}

#[derive(Clone, Debug, PartialEq, Reflect, Serialize)]
pub struct SymbolDef {
    pub name: String,
    pub rules: Vec<SymbolRule>,
}

fn parse_symbol_rules(pairs: Pairs<Rule>) -> Vec<SymbolRule> {
    pairs
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
                Rule::polygon => SymbolRule::Polygon(symbolrule.map(parse_point).collect()),
                _ => {
                    unreachable!()
                }
            }
        })
        .collect()
}

pub(super) fn parse_symbol(pair: Pair<Rule>) -> Option<SymbolDef> {
    match pair.as_rule() {
        Rule::symbol | Rule::symboldef => {
            let mut symbol = pair.into_inner();
            let name = symbol.next().unwrap().as_str().to_string();
            Some(SymbolDef {
                name,
                rules: parse_symbol_rules(symbol),
            })
        }
        Rule::EOI => None,
        _ => unreachable!(),
    }
}

pub(super) fn parse_topsky_symbols(
    contents: &[u8],
) -> Result<HashMap<String, SymbolDef>, TopskyError> {
    let symbols =
        TopskyParser::parse(Rule::symbols, &read_to_string(contents)?).map(|mut pairs| {
            pairs
                .next()
                .unwrap()
                .into_inner()
                .filter_map(parse_symbol)
                .map(|symbol| (symbol.name.clone(), symbol))
                .collect::<HashMap<_, _>>()
        })?;

    Ok(symbols)
}

#[cfg(test)]
mod test {
    use crate::topsky::symbol::{SymbolDef, SymbolRule};

    use super::parse_topsky_symbols;

    #[test]
    fn test_symbols() {
        let symbols_str = br#"
SYMBOL:AIRPORT
MOVETO:-3.2:-3
LINETO:3:-3
LINETO:3:3
LINETO:-3:3
LINETO:-3:-3
MOVETO:5:0
LINETO:-6:0
MOVETO:0:5
LINETO:0:-6

SYMBOL:NDB
SETPIXEL:0:0
ARC:0:0:1:0:360
ARC:0:0:3:0:360
ARC:0:0:5:0:360

SYMBOL:HISTORY
FILLARC:0:0:1:0:360

SYMBOL:APPFix
ARC:0:0:2:6:0:360
ARC:0:0:6:2:0:360

SYMBOL:NODAPS_DIV
POLYGON:-4:0:0:-4:4:0:0:4
ARC:0:0:8:0:0"#;
        let symbols = parse_topsky_symbols(symbols_str).unwrap();

        assert_eq!(
            symbols.get("AIRPORT").unwrap(),
            &SymbolDef {
                name: "AIRPORT".to_string(),
                rules: vec![
                    SymbolRule::Move((-3.2, -3.0)),
                    SymbolRule::Line((3.0, -3.0)),
                    SymbolRule::Line((3.0, 3.0)),
                    SymbolRule::Line((-3.0, 3.0)),
                    SymbolRule::Line((-3.0, -3.0)),
                    SymbolRule::Move((5.0, 0.0)),
                    SymbolRule::Line((-6.0, 0.0)),
                    SymbolRule::Move((0.0, 5.0)),
                    SymbolRule::Line((0.0, -6.0)),
                ]
            }
        );

        assert_eq!(
            symbols.get("NDB").unwrap(),
            &SymbolDef {
                name: "NDB".to_string(),
                rules: vec![
                    SymbolRule::Pixel((0.0, 0.0)),
                    SymbolRule::Arc((0.0, 0.0), 1.0, 0, 0),
                    SymbolRule::Arc((0.0, 0.0), 3.0, 0, 0),
                    SymbolRule::Arc((0.0, 0.0), 5.0, 0, 0),
                ]
            }
        );

        assert_eq!(
            symbols.get("HISTORY").unwrap(),
            &SymbolDef {
                name: "HISTORY".to_string(),
                rules: vec![SymbolRule::FilledArc((0.0, 0.0), 1.0, 0, 0),]
            }
        );

        assert_eq!(
            symbols.get("APPFix").unwrap(),
            &SymbolDef {
                name: "APPFix".to_string(),
                rules: vec![
                    SymbolRule::EllipticArc((0.0, 0.0), 2.0, 6.0, 0, 0),
                    SymbolRule::EllipticArc((0.0, 0.0), 6.0, 2.0, 0, 0),
                ]
            }
        );

        assert_eq!(
            symbols.get("NODAPS_DIV").unwrap(),
            &SymbolDef {
                name: "NODAPS_DIV".to_string(),
                rules: vec![
                    SymbolRule::Polygon(vec![(-4.0, 0.0), (0.0, -4.0), (4.0, 0.0), (0.0, 4.0)]),
                    SymbolRule::Arc((0.0, 0.0), 8.0, 0, 0),
                ]
            }
        );
    }
}
