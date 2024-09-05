pub mod track;

use std::{collections::HashMap, path::PathBuf};

use serde::Serialize;
use tracing::warn;

use crate::{squawks::SquawksJson, symbology::Symbology, topsky::Topsky};

use self::track::TrackSettings;

use super::{line_styles::LineStyle, Alignment};

#[derive(Clone, Debug, Serialize)]
pub struct LineSettings {
    /// line style of .sct runway centre lines
    pub runway_centreline: LineStyle,
}
impl LineSettings {
    const DEFAULT_RUNWAY_CENTRELINE_WIDTH: i32 = 1;
    const DEFAULT_RUNWAY_CENTRELINE_STYLE: &'static str = LineStyle::SOLID;

    pub fn from_euroscope(symbology: &Symbology) -> Self {
        let runway_centreline = symbology
            .items
            .get(&("Runways".to_string(), "centerline".to_string()));
        Self {
            runway_centreline: runway_centreline.map_or(
                LineStyle {
                    width: Self::DEFAULT_RUNWAY_CENTRELINE_WIDTH,
                    style: Self::DEFAULT_RUNWAY_CENTRELINE_STYLE.to_string(),
                },
                |item| LineStyle {
                    width: item.line_weight,
                    style: item.line_style.clone(),
                },
            ),
        }
    }
}

impl Default for LineSettings {
    fn default() -> Self {
        Self {
            runway_centreline: LineStyle {
                width: Self::DEFAULT_RUNWAY_CENTRELINE_WIDTH,
                style: Self::DEFAULT_RUNWAY_CENTRELINE_STYLE.to_string(),
            },
        }
    }
}

/// settings for .sct labels activated in .asr
#[derive(Clone, Debug, Serialize)]
pub struct LabelSettings {
    pub vor_alignment: Alignment,
    pub vor_font_size: f32,
    pub ndb_alignment: Alignment,
    pub ndb_font_size: f32,
    pub fix_alignment: Alignment,
    pub fix_font_size: f32,
    pub airport_alignment: Alignment,
    pub airport_font_size: f32,
    pub runway_alignment: Alignment,
    pub runway_font_size: f32,
}
impl LabelSettings {
    const DEFAULT_FONT_SIZE: f32 = 3.0;

    pub fn from_euroscope(symbology: &Symbology) -> Self {
        let runway = symbology
            .items
            .get(&("Runways".to_string(), "name".to_string()));
        let fix = symbology
            .items
            .get(&("Fixes".to_string(), "name".to_string()));
        let vor = symbology
            .items
            .get(&("VORs".to_string(), "name".to_string()));
        let ndb = symbology
            .items
            .get(&("NDBs".to_string(), "name".to_string()));
        let airport = symbology
            .items
            .get(&("Airports".to_string(), "name".to_string()));
        Self {
            fix_alignment: fix.map(|item| item.text_alignment).unwrap_or_default(),
            fix_font_size: fix.map_or(Self::DEFAULT_FONT_SIZE, |item| item.font_size_symbol_scale),
            airport_alignment: airport.map(|item| item.text_alignment).unwrap_or_default(),
            airport_font_size: airport
                .map_or(Self::DEFAULT_FONT_SIZE, |item| item.font_size_symbol_scale),
            runway_alignment: runway.map(|item| item.text_alignment).unwrap_or_default(),
            runway_font_size: runway
                .map_or(Self::DEFAULT_FONT_SIZE, |item| item.font_size_symbol_scale),
            vor_alignment: vor.map(|item| item.text_alignment).unwrap_or_default(),
            vor_font_size: vor.map_or(Self::DEFAULT_FONT_SIZE, |item| item.font_size_symbol_scale),
            ndb_alignment: ndb.map(|item| item.text_alignment).unwrap_or_default(),
            ndb_font_size: ndb.map_or(Self::DEFAULT_FONT_SIZE, |item| item.font_size_symbol_scale),
        }
    }
}

impl Default for LabelSettings {
    fn default() -> Self {
        Self {
            fix_alignment: Alignment::default(),
            fix_font_size: Self::DEFAULT_FONT_SIZE,
            airport_alignment: Alignment::default(),
            airport_font_size: Self::DEFAULT_FONT_SIZE,
            runway_alignment: Alignment::default(),
            runway_font_size: Self::DEFAULT_FONT_SIZE,
            vor_alignment: Alignment::default(),
            vor_font_size: Self::DEFAULT_FONT_SIZE,
            ndb_alignment: Alignment::default(),
            ndb_font_size: Self::DEFAULT_FONT_SIZE,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct MapsSettings {
    // TODO font from name is complicated in bevy currently
    /// path to font file
    pub font: Option<PathBuf>,
    pub font_size: f32,
    pub auto_folder: String,
    pub layer: f32,
    pub label_offset: (f64, f64),
    pub line_styles: LineSettings,
    pub labels: LabelSettings,
}
impl MapsSettings {
    const DEFAULT_FONT_SIZE: f32 = 12.0;
    const DEFAULT_LAYER: f32 = 1.0;
    const DEFAULT_AUTO_FOLDER: &'static str = "AUTO";
    const DEFAULT_LABEL_OFFSET: (f64, f64) = (3.0, 3.0);

    pub fn from_euroscope(symbology: &Symbology, topsky: Option<&Topsky>) -> Self {
        Self {
            font_size: topsky.map_or(Self::DEFAULT_FONT_SIZE, |t| {
                t.settings
                    .parse_with_default("Maps_FontSize", Self::DEFAULT_FONT_SIZE)
            }),
            line_styles: LineSettings::from_euroscope(symbology),
            labels: LabelSettings::from_euroscope(symbology),
            ..Default::default()
        }
    }
}

impl Default for MapsSettings {
    fn default() -> Self {
        Self {
            auto_folder: Self::DEFAULT_AUTO_FOLDER.to_string(),
            font_size: Self::DEFAULT_FONT_SIZE,
            font: None,
            layer: Self::DEFAULT_LAYER,
            label_offset: Self::DEFAULT_LABEL_OFFSET,
            line_styles: LineSettings::default(),
            labels: LabelSettings::default(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct SsrSettings {
    pub special_use_codes: HashMap<String, String>,
}
impl SsrSettings {
    pub fn from_squawks_json(squawks: &SquawksJson) -> Self {
        Self {
            special_use_codes: squawks
                .squawks
                .iter()
                .flat_map(|sq| match (&sq.code, &sq.range) {
                    (None, None) | (Some(_), Some(_)) => {
                        warn!("Specify either code or range: {sq:?}");
                        vec![]
                    }
                    (Some(code), None) => u16::from_str_radix(code, 8)
                        .map(|u16_code| vec![(format!("{u16_code:04o}"), sq.message.clone())])
                        .unwrap_or(vec![]),
                    (None, Some(range)) => {
                        if let Some((start, end)) = u16::from_str_radix(&range.start, 8)
                            .ok()
                            .zip(u16::from_str_radix(&range.end, 8).ok())
                        {
                            (start..=end)
                                .map(|i| (format!("{i:04o}"), sq.message.clone()))
                                .collect()
                        } else {
                            vec![]
                        }
                    }
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Settings {
    // text, enabled, divergence, delay(?),  ...?
    // clam: ClamSettings,
    // ram: RamSettings,
    pub maps: MapsSettings,
    pub track: TrackSettings,
    pub coopans: bool,
    pub ssr: SsrSettings,
}

impl Settings {
    pub fn from_euroscope(
        symbology: &Symbology,
        topsky: Option<&Topsky>,
        squawks: Option<&SquawksJson>,
    ) -> Self {
        Self {
            maps: MapsSettings::from_euroscope(symbology, topsky),
            track: topsky.map(TrackSettings::from_topsky).unwrap_or_default(),
            coopans: topsky
                .and_then(|t| t.settings.get("Setup_COOPANS").map(|v| v != "0"))
                .unwrap_or(true),
            ssr: squawks
                .as_ref()
                .map(|sq| SsrSettings::from_squawks_json(sq))
                .unwrap_or_default(),
        }
    }
}
