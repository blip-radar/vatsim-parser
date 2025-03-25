use std::{collections::HashMap, sync::OnceLock};

use bevy_reflect::Reflect;
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
    other: HashMap<String, Symbol>,
}

// FIXME check all these, probably have some ES-specific workaround offsets

fn default_history_dot() -> &'static Symbol {
    static DEFAULT_HISTORY_DOT: OnceLock<Symbol> = OnceLock::new();
    DEFAULT_HISTORY_DOT.get_or_init(|| {
        vec![SymbolRule::Polygon(vec![
            (-1.0, -1.0),
            (1.0, -1.0),
            (1.0, 1.0),
            (-1.0, 1.0),
        ])]
    })
}
fn default_fix() -> &'static Symbol {
    static DEFAULT_FIX: OnceLock<Symbol> = OnceLock::new();
    DEFAULT_FIX.get_or_init(|| {
        vec![
            SymbolRule::Move((-5.0, 5.0)),
            SymbolRule::Line((0.0, -5.0)),
            SymbolRule::Line((5.0, 5.0)),
            SymbolRule::Line((-5.0, 5.0)),
        ]
    })
}
fn default_ndb() -> &'static Symbol {
    static DEFAULT_NDB: OnceLock<Symbol> = OnceLock::new();
    DEFAULT_NDB.get_or_init(|| {
        vec![
            SymbolRule::Pixel((0.0, 0.0)),
            SymbolRule::Arc((0.0, -0.0), 1.0, 0, 360),
            SymbolRule::Arc((0.0, -0.0), 3.0, 0, 360),
            SymbolRule::Arc((0.0, -0.0), 5.0, 0, 360),
        ]
    })
}
fn default_vor() -> &'static Symbol {
    static DEFAULT_VOR: OnceLock<Symbol> = OnceLock::new();
    DEFAULT_VOR.get_or_init(|| {
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
    })
}
fn default_airport() -> &'static Symbol {
    static DEFAULT_AIRPORT: OnceLock<Symbol> = OnceLock::new();
    DEFAULT_AIRPORT.get_or_init(|| {
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
    })
}
fn default_primary() -> &'static Symbol {
    static DEFAULT_PRIMARY: OnceLock<Symbol> = OnceLock::new();
    DEFAULT_PRIMARY.get_or_init(|| {
        vec![
            SymbolRule::Move((-3.0, -3.0)),
            SymbolRule::Line((4.0, 4.0)),
            SymbolRule::Move((-3.0, 3.0)),
            SymbolRule::Line((4.0, -4.0)),
        ]
    })
}
fn default_controlled() -> &'static Symbol {
    static DEFAULT_CONTROLLED: OnceLock<Symbol> = OnceLock::new();
    DEFAULT_CONTROLLED.get_or_init(|| {
        vec![
            SymbolRule::Move((-5.0, 0.0)),
            SymbolRule::Line((0.0, -5.0)),
            SymbolRule::Line((5.0, 0.0)),
            SymbolRule::Line((0.0, 5.0)),
            SymbolRule::Line((-5.0, 0.0)),
        ]
    })
}
fn default_uncontrolled() -> &'static Symbol {
    static DEFAULT_UNCONTROLLED: OnceLock<Symbol> = OnceLock::new();
    DEFAULT_UNCONTROLLED.get_or_init(|| {
        vec![
            SymbolRule::Move((-4.0, 3.0)),
            SymbolRule::Line((0.0, -4.0)),
            SymbolRule::Line((3.0, 3.0)),
            SymbolRule::Line((-4.0, 3.0)),
        ]
    })
}
fn default_coasted() -> &'static Symbol {
    static DEFAULT_COASTED: OnceLock<Symbol> = OnceLock::new();
    DEFAULT_COASTED.get_or_init(|| {
        vec![
            SymbolRule::Move((-4.0, -4.0)),
            SymbolRule::Line((-4.0, 4.0)),
            SymbolRule::Line((4.0, 4.0)),
            SymbolRule::Line((4.0, -4.0)),
        ]
    })
}

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
                .unwrap_or(default_history_dot().clone()),
            fix: topsky
                .as_ref()
                .and_then(|t| t.symbols.get("FIX"))
                .map(|s| s.rules.clone())
                .or_else(|| symbology.symbols.get(&SymbolType::Fix).cloned())
                .unwrap_or(default_fix().clone()),
            ndb: topsky
                .as_ref()
                .and_then(|t| t.symbols.get("NDB"))
                .map(|s| s.rules.clone())
                .or_else(|| symbology.symbols.get(&SymbolType::NDB).cloned())
                .unwrap_or(default_ndb().clone()),
            vor: topsky
                .as_ref()
                .and_then(|t| t.symbols.get("VOR"))
                .map(|s| s.rules.clone())
                .or_else(|| symbology.symbols.get(&SymbolType::VOR).cloned())
                .unwrap_or(default_ndb().clone()),
            airport: topsky
                .as_ref()
                .and_then(|t| t.symbols.get("AIRPORT"))
                .map(|s| s.rules.clone())
                .or_else(|| symbology.symbols.get(&SymbolType::Airport).cloned())
                .unwrap_or(default_airport().clone()),
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
                .unwrap_or(default_primary().clone()),
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
                .unwrap_or(default_coasted().clone()),
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
                .unwrap_or(default_uncontrolled().clone()),
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
                .unwrap_or(default_controlled().clone()),
            other: topsky_symbols,
        }
    }
}

impl Default for Symbols {
    fn default() -> Self {
        Self {
            history_dot: default_history_dot().clone(),
            fix: default_fix().clone(),
            ndb: default_ndb().clone(),
            vor: default_vor().clone(),
            airport: default_airport().clone(),
            primary: default_primary().clone(),
            coasted: default_coasted().clone(),
            uncontrolled: default_uncontrolled().clone(),
            controlled: default_controlled().clone(),
            other: HashMap::new(),
        }
    }
}
