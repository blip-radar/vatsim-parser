use std::collections::HashMap;

use bevy_reflect::Reflect;
use serde::Serialize;

use crate::topsky::Topsky;

#[derive(Clone, Debug, Reflect, Serialize, PartialEq)]
pub struct Dash {
    pub length: f32,
    pub gap: f32,
}

#[derive(Clone, Debug, Reflect, Serialize, PartialEq)]
pub struct LineStyle {
    pub width: i32,
    pub style: String,
}

impl LineStyle {
    pub const SOLID: &'static str = "SOLID";
    pub const ALTERNATE: &'static str = "ALTERNATE";
    pub const DASH: &'static str = "DASH";
    pub const DOT: &'static str = "DOT";
    pub const DASHDOT: &'static str = "DASHDOT";
    pub const DASHDOTDOT: &'static str = "DASHDOTDOT";
}

impl Default for LineStyle {
    fn default() -> Self {
        Self {
            width: 1,
            style: Self::SOLID.to_string(),
        }
    }
}

pub fn line_styles_from_topsky(topsky: &Option<Topsky>) -> HashMap<String, Option<Vec<Dash>>> {
    topsky
        .as_ref()
        .map(|t| {
            t.line_styles
                .iter()
                .map(|(name, style)| {
                    (
                        name.to_uppercase(),
                        if style.dash_lengths.len() > 1 {
                            Some(
                                style
                                    .dash_lengths
                                    .chunks_exact(2)
                                    .map(|chunk| Dash {
                                        length: chunk[0],
                                        gap: chunk[1],
                                    })
                                    .collect(),
                            )
                        } else {
                            None
                        },
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
        .into_iter()
        .chain([
            (LineStyle::SOLID.to_string(), None),
            (
                LineStyle::DOT.to_string(),
                Some(vec![Dash {
                    length: 1.0,
                    gap: 1.0,
                }]),
            ),
            (
                LineStyle::ALTERNATE.to_string(),
                Some(vec![Dash {
                    length: 1.0,
                    gap: 1.0,
                }]),
            ),
            (
                LineStyle::DASH.to_string(),
                Some(vec![Dash {
                    length: 3.0,
                    gap: 3.0,
                }]),
            ),
            (
                LineStyle::DASHDOT.to_string(),
                Some(vec![
                    Dash {
                        length: 3.0,
                        gap: 3.0,
                    },
                    Dash {
                        length: 1.0,
                        gap: 3.0,
                    },
                ]),
            ),
            (
                LineStyle::DASHDOTDOT.to_string(),
                Some(vec![
                    Dash {
                        length: 3.0,
                        gap: 3.0,
                    },
                    Dash {
                        length: 1.0,
                        gap: 3.0,
                    },
                    Dash {
                        length: 1.0,
                        gap: 3.0,
                    },
                ]),
            ),
        ])
        .collect()
}
