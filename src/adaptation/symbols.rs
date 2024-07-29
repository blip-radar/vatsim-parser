use std::collections::HashMap;

use bevy_reflect::Reflect;
use once_cell::sync::Lazy;
use serde::Serialize;

use crate::{
    symbology::{SymbolType, Symbology},
    topsky::Topsky,
};

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

type Symbol = Vec<SymbolRule>;
#[derive(Clone, Debug, Serialize)]
pub struct Symbols {
    pub history_dot: Symbol,
    pub fix: Symbol,
    pub ndb: Symbol,
    pub vor: Symbol,
    pub airport: Symbol,
    pub primary: Symbol,
    pub coasted: Symbol,
    pub uncontrolled: Symbol,
    pub controlled: Symbol,
    other: HashMap<String, Vec<SymbolRule>>,
}

// FIXME check all these, probably have some ES-specific workaround offsets
static DEFAULT_HISTORY_DOT: Lazy<Symbol> = Lazy::new(|| {
    vec![SymbolRule::Polygon(vec![
        (-1.0, -1.0),
        (1.0, -1.0),
        (1.0, 1.0),
        (-1.0, 1.0),
    ])]
});
static DEFAULT_FIX: Lazy<Symbol> = Lazy::new(|| {
    vec![
        SymbolRule::Move((-5.0, 5.0)),
        SymbolRule::Line((0.0, -5.0)),
        SymbolRule::Line((5.0, 5.0)),
        SymbolRule::Line((-5.0, 5.0)),
    ]
});
static DEFAULT_NDB: Lazy<Symbol> = Lazy::new(|| {
    vec![
        SymbolRule::Pixel((0.0, 0.0)),
        SymbolRule::Arc((0.0, -0.0), 1.0, 0, 360),
        SymbolRule::Arc((0.0, -0.0), 3.0, 0, 360),
        SymbolRule::Arc((0.0, -0.0), 5.0, 0, 360),
    ]
});
static DEFAULT_VOR: Lazy<Symbol> = Lazy::new(|| {
    vec![
        SymbolRule::Move((-6.0, 0.0)),
        SymbolRule::Line((-2.0, -5.0)),
        SymbolRule::Line((2.0, -5.0)),
        SymbolRule::Line((6.0, 0.0)),
        SymbolRule::Line((2.0, 5.0)),
        SymbolRule::Line((-2.0, 5.0)),
        SymbolRule::Line((-6.0, 0.0)),
        SymbolRule::Move((-6.0, -5.0)),
        SymbolRule::Line((-6.0, 5.0)),
        SymbolRule::Line((6.0, 5.0)),
        SymbolRule::Line((6.0, -5.0)),
        SymbolRule::Line((-6.0, -5.0)),
    ]
});
static DEFAULT_AIRPORT: Lazy<Symbol> = Lazy::new(|| {
    vec![
        SymbolRule::Move((-3.0, -3.0)),
        SymbolRule::Line((3.0, -3.0)),
        SymbolRule::Line((3.0, 3.0)),
        SymbolRule::Line((-3.0, 3.0)),
        SymbolRule::Line((-3.0, -3.0)),
        SymbolRule::Move((5.0, 0.0)),
        SymbolRule::Line((-6.0, 0.0)),
        SymbolRule::Move((0.0, 5.0)),
        SymbolRule::Line((0.0, -6.0)),
    ]
});
static DEFAULT_PRIMARY: Lazy<Symbol> = Lazy::new(|| {
    vec![
        SymbolRule::Move((-3.0, -3.0)),
        SymbolRule::Line((4.0, 4.0)),
        SymbolRule::Move((-3.0, 3.0)),
        SymbolRule::Line((4.0, -4.0)),
    ]
});
static DEFAULT_CONTROLLED: Lazy<Symbol> = Lazy::new(|| {
    vec![
        SymbolRule::Move((-5.0, 0.0)),
        SymbolRule::Line((0.0, -5.0)),
        SymbolRule::Line((5.0, 0.0)),
        SymbolRule::Line((0.0, 5.0)),
        SymbolRule::Line((-5.0, 0.0)),
    ]
});
static DEFAULT_UNCONTROLLED: Lazy<Symbol> = Lazy::new(|| {
    vec![
        SymbolRule::Move((-4.0, 3.0)),
        SymbolRule::Line((0.0, -4.0)),
        SymbolRule::Line((3.0, 3.0)),
        SymbolRule::Line((-4.0, 3.0)),
    ]
});
static DEFAULT_COASTED: Lazy<Symbol> = Lazy::new(|| {
    vec![
        SymbolRule::Move((-4.0, -4.0)),
        SymbolRule::Line((-4.0, 4.0)),
        SymbolRule::Line((4.0, 4.0)),
        SymbolRule::Line((4.0, -4.0)),
    ]
});

impl Symbols {
    pub fn get(&self, name: &str) -> Option<&[SymbolRule]> {
        self.other.get(name).map(Vec::as_slice)
    }

    pub fn from_euroscope(symbology: &Symbology, topsky: &Option<Topsky>) -> Self {
        let topsky_symbols = topsky
            .as_ref()
            .map(|t| {
                t.symbols
                    .iter()
                    .map(|(key, symbol)| (key.clone(), symbol.rules.clone()))
                    .collect()
            })
            .unwrap_or_default();
        Self {
            history_dot: topsky
                .as_ref()
                .and_then(|t| t.symbols.get("HISTORY"))
                .map(|s| s.rules.clone())
                .or_else(|| symbology.symbols.get(&SymbolType::HistoryDot).cloned())
                .unwrap_or(DEFAULT_HISTORY_DOT.to_vec()),
            fix: topsky
                .as_ref()
                .and_then(|t| t.symbols.get("FIX"))
                .map(|s| s.rules.clone())
                .or_else(|| symbology.symbols.get(&SymbolType::Fix).cloned())
                .unwrap_or(DEFAULT_FIX.to_vec()),
            ndb: topsky
                .as_ref()
                .and_then(|t| t.symbols.get("NDB"))
                .map(|s| s.rules.clone())
                .or_else(|| symbology.symbols.get(&SymbolType::NDB).cloned())
                .unwrap_or(DEFAULT_NDB.to_vec()),
            vor: topsky
                .as_ref()
                .and_then(|t| t.symbols.get("VOR"))
                .map(|s| s.rules.clone())
                .or_else(|| symbology.symbols.get(&SymbolType::VOR).cloned())
                .unwrap_or(DEFAULT_VOR.to_vec()),
            airport: topsky
                .as_ref()
                .and_then(|t| t.symbols.get("AIRPORT"))
                .map(|s| s.rules.clone())
                .or_else(|| symbology.symbols.get(&SymbolType::Airport).cloned())
                .unwrap_or(DEFAULT_AIRPORT.to_vec()),
            primary: topsky
                .as_ref()
                .and_then(|t| t.symbols.get("PRIMARY"))
                .map(|s| s.rules.clone())
                .or_else(|| {
                    symbology
                        .symbols
                        .get(&SymbolType::AircraftPrimaryOnly)
                        .cloned()
                })
                .unwrap_or(DEFAULT_PRIMARY.to_vec()),
            coasted: topsky
                .as_ref()
                .and_then(|t| t.symbols.get("COASTED"))
                .map(|s| s.rules.clone())
                .or_else(|| {
                    symbology
                        .symbols
                        .get(&SymbolType::AircraftCoasting)
                        .cloned()
                })
                .unwrap_or(DEFAULT_COASTED.to_vec()),
            uncontrolled: topsky
                .as_ref()
                .and_then(|t| t.symbols.get("UNCONTROLLED"))
                .map(|s| s.rules.clone())
                .or_else(|| {
                    symbology
                        .symbols
                        .get(&SymbolType::AircraftUncorrModeAlphaCharlie)
                        .cloned()
                })
                .unwrap_or(DEFAULT_UNCONTROLLED.to_vec()),
            controlled: topsky
                .as_ref()
                .and_then(|t| t.symbols.get("DAPS"))
                .map(|s| s.rules.clone())
                .or_else(|| {
                    symbology
                        .symbols
                        .get(&SymbolType::AircraftCorrModeSierra)
                        .cloned()
                })
                .unwrap_or(DEFAULT_CONTROLLED.to_vec()),
            other: topsky_symbols,
        }
    }
}

impl Default for Symbols {
    fn default() -> Self {
        Self {
            history_dot: DEFAULT_HISTORY_DOT.to_vec(),
            fix: DEFAULT_FIX.to_vec(),
            ndb: DEFAULT_NDB.to_vec(),
            vor: DEFAULT_VOR.to_vec(),
            airport: DEFAULT_AIRPORT.to_vec(),
            primary: DEFAULT_PRIMARY.to_vec(),
            coasted: DEFAULT_COASTED.to_vec(),
            uncontrolled: DEFAULT_UNCONTROLLED.to_vec(),
            controlled: DEFAULT_CONTROLLED.to_vec(),
            other: HashMap::new(),
        }
    }
}
