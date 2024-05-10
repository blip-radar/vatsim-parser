use serde::Serialize;

use crate::topsky::Topsky;

const DEFAULT_VECTOR_LENGTH: f32 = 1.0;
const DEFAULT_HISTORY_DOTS_COUNT: u32 = 6;

// TODO some systems support duration instead of count
#[derive(Clone, Debug, Serialize)]
pub struct HistoryDotsSettings {
    pub enabled: bool,
    pub count: u32,
    pub options: Vec<u32>,
}

impl HistoryDotsSettings {
    pub fn from_topsky(topsky: &Topsky) -> Self {
        let topsky_count = topsky
            .settings
            .parse_with_default("Track_HistoryDots", DEFAULT_HISTORY_DOTS_COUNT);
        Self {
            count: if topsky_count != 0 {
                topsky_count
            } else {
                DEFAULT_HISTORY_DOTS_COUNT
            },
            enabled: topsky_count != 0,
            ..Default::default()
        }
    }
}
impl Default for HistoryDotsSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            count: DEFAULT_HISTORY_DOTS_COUNT,
            options: vec![0, 1, 2, 3, 4, 5, 6, 7, 8],
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct VectorSettings {
    pub enabled: bool,
    pub length: f32,
    pub options: Vec<f32>,
}

impl VectorSettings {
    pub fn from_topsky(topsky: &Topsky) -> Self {
        let topsky_length = topsky
            .settings
            .parse_with_default("Track_PredictionLine", DEFAULT_VECTOR_LENGTH);
        Self {
            length: if topsky_length != 0.0 {
                topsky_length
            } else {
                DEFAULT_VECTOR_LENGTH
            },
            enabled: topsky_length != 0.0,
            ..Default::default()
        }
    }
}
impl Default for VectorSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            length: DEFAULT_VECTOR_LENGTH,
            options: vec![
                0.5, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 15.0, 20.0,
            ],
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct TrackSettings {
    pub vector: VectorSettings,
    pub history_dots: HistoryDotsSettings,
}
impl TrackSettings {
    pub fn from_topsky(topsky: &Topsky) -> Self {
        Self {
            history_dots: HistoryDotsSettings::from_topsky(topsky),
            vector: VectorSettings::from_topsky(topsky),
        }
    }
}
