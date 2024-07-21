pub mod track;

use std::{collections::HashMap, path::PathBuf};

use serde::Serialize;

use crate::{squawks::SquawksJson, topsky::Topsky};

use self::track::TrackSettings;

#[derive(Clone, Debug, Serialize)]
pub struct MapsSettings {
    // TODO font from name is complicated in bevy currently
    /// path to font file
    pub font: Option<PathBuf>,
    pub font_size: f32,
    pub auto_folder: String,
    pub layer: f32,
    pub label_offset: (f64, f64),
}
impl MapsSettings {
    const DEFAULT_FONT_SIZE: f32 = 12.0;
    const DEFAULT_LAYER: f32 = 1.0;
    const DEFAULT_AUTO_FOLDER: &'static str = "AUTO";
    const DEFAULT_LABEL_OFFSET: (f64, f64) = (3.0, 3.0);

    pub fn from_topsky(topsky: &Topsky) -> Self {
        Self {
            font_size: topsky
                .settings
                .parse_with_default("Maps_FontSize", Self::DEFAULT_FONT_SIZE),
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
                        eprintln!("Specify either code or range: {sq:?}");
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
    pub fn from_euroscope(topsky: &Option<Topsky>, squawks: &Option<SquawksJson>) -> Self {
        Self {
            maps: topsky
                .as_ref()
                .map(MapsSettings::from_topsky)
                .unwrap_or_default(),
            track: topsky
                .as_ref()
                .map(TrackSettings::from_topsky)
                .unwrap_or_default(),
            coopans: topsky
                .as_ref()
                .and_then(|t| t.settings.0.get("Setup_COOPANS").map(|v| v != "0"))
                .unwrap_or(true),
            ssr: squawks
                .as_ref()
                .map(SsrSettings::from_squawks_json)
                .unwrap_or_default(),
        }
    }
}
