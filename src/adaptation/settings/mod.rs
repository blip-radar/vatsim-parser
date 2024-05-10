pub mod track;

use std::path::PathBuf;

use serde::Serialize;

use crate::topsky::Topsky;

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
pub struct Settings {
    // text, enabled, divergence, delay(?),  ...?
    // clam: ClamSettings,
    // ram: RamSettings,
    pub maps: MapsSettings,
    pub track: TrackSettings,
    pub coopans: bool,
}

impl Settings {
    pub fn from_topsky(topsky: &Topsky) -> Self {
        Self {
            maps: MapsSettings::from_topsky(topsky),
            track: TrackSettings::from_topsky(topsky),
            coopans: topsky
                .settings
                .0
                .get("Setup_COOPANS")
                .map(|v| v != "0")
                .unwrap_or(true),
        }
    }
}
