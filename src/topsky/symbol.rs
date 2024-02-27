use bevy_reflect::Reflect;
use pest::iterators::{Pair, Pairs};
use serde::Serialize;

use super::{parse_point, Rule};

#[derive(Clone, Debug, Reflect, Serialize)]
pub enum SymbolRule {
    Move((f64, f64)),
    Line((f64, f64)),
    Pixel((f64, f64)),
    Arc((f64, f64), f32, f32, f32),
    EllipticArc((f64, f64), i64, i64, i64, i64),
    FilledArc((f64, f64), i64, i64, i64),
    FilledEllipticArc((f64, f64), i64, i64, i64, i64),
    Polygon(Vec<(f64, f64)>),
}

#[derive(Clone, Debug, Reflect, Serialize)]
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
                    let start_angle = symbolrule.next().unwrap().as_str().parse().unwrap();
                    let end_angle = symbolrule.next().unwrap().as_str().parse().unwrap();
                    SymbolRule::Arc(pos, radius, start_angle, end_angle)
                }
                Rule::fillarc => {
                    let pos = parse_point(symbolrule.next().unwrap());
                    let radius = symbolrule.next().unwrap().as_str().parse().unwrap();
                    let start_angle = symbolrule.next().unwrap().as_str().parse().unwrap();
                    let end_angle = symbolrule.next().unwrap().as_str().parse().unwrap();
                    SymbolRule::FilledArc(pos, radius, start_angle, end_angle)
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
