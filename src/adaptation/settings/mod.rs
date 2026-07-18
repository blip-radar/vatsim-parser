pub mod track;

use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};
use tracing::warn;
use uom::si::time::minute;
use uom::si::{f32::Time, time::second};

use crate::{prf::Prf, squawks::SquawksJson, symbology::Symbology, topsky::Topsky};

use self::track::TrackSettings;

use super::{line_styles::LineStyle, Alignment};

const EUROSCOPE_FONT_SIZE_FACTOR: f32 = 3.5;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LineSettings {
    /// line style of .sct runway centre lines
    pub runway_centreline: LineStyle,
    /// line style of .sct Geo lines
    pub geo: LineStyle,
    /// line style of .sct SID lines
    pub sid: LineStyle,
    /// line style of .sct STAR lines
    pub star: LineStyle,
    /// line style of .sct High Airway lines
    pub high_airways: LineStyle,
    /// line style of .sct Low Airway lines
    pub low_airways: LineStyle,
    /// line style of .sct ARTCC low boundary lines
    pub artcc_low: LineStyle,
    /// line style of .sct ARTCC boundary lines
    pub artcc: LineStyle,
    /// line style of .sct ARTCC high boundary lines
    pub artcc_high: LineStyle,
}
impl LineSettings {
    const DEFAULT_RUNWAY_CENTRELINE_WIDTH: i32 = 1;
    const DEFAULT_RUNWAY_CENTRELINE_STYLE: &'static str = LineStyle::SOLID;
    const DEFAULT_GEO_WIDTH: i32 = 1;
    const DEFAULT_GEO_STYLE: &'static str = LineStyle::SOLID;
    const DEFAULT_SID_WIDTH: i32 = 1;
    const DEFAULT_SID_STYLE: &'static str = LineStyle::SOLID;
    const DEFAULT_STAR_WIDTH: i32 = 1;
    const DEFAULT_STAR_STYLE: &'static str = LineStyle::SOLID;
    const DEFAULT_ARTCC_LOW_WIDTH: i32 = 1;
    const DEFAULT_ARTCC_LOW_STYLE: &'static str = LineStyle::SOLID;
    const DEFAULT_ARTCC_WIDTH: i32 = 1;
    const DEFAULT_ARTCC_STYLE: &'static str = LineStyle::SOLID;
    const DEFAULT_ARTCC_HIGH_WIDTH: i32 = 1;
    const DEFAULT_ARTCC_HIGH_STYLE: &'static str = LineStyle::SOLID;
    const DEFAULT_HIGH_AIRWAY_WIDTH: i32 = 1;
    const DEFAULT_HIGH_AIRWAY_STYLE: &'static str = LineStyle::SOLID;
    const DEFAULT_LOW_AIRWAY_WIDTH: i32 = 1;
    const DEFAULT_LOW_AIRWAY_STYLE: &'static str = LineStyle::SOLID;

    pub fn from_euroscope(symbology: &Symbology) -> Self {
        let runway_centreline = symbology
            .items
            .get(&("Runways".to_string(), "centerline".to_string()));
        let geo = symbology
            .items
            .get(&("Geo".to_string(), "line".to_string()));
        let sid = symbology
            .items
            .get(&("Sids".to_string(), "line".to_string()));
        let star = symbology
            .items
            .get(&("Stars".to_string(), "line".to_string()));
        let high_airways = symbology
            .items
            .get(&("High airways".to_string(), "line".to_string()));
        let low_airways = symbology
            .items
            .get(&("Low airways".to_string(), "line".to_string()));
        let artcc_high = symbology
            .items
            .get(&("ARTCC high boundary".to_string(), "line".to_string()));
        let artcc = symbology
            .items
            .get(&("ARTCC boundary".to_string(), "line".to_string()));
        let artcc_low = symbology
            .items
            .get(&("ARTCC low boundary".to_string(), "line".to_string()));
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
            geo: geo.map_or(
                LineStyle {
                    width: Self::DEFAULT_GEO_WIDTH,
                    style: Self::DEFAULT_GEO_STYLE.to_string(),
                },
                |item| LineStyle {
                    width: item.line_weight,
                    style: item.line_style.clone(),
                },
            ),
            sid: sid.map_or(
                LineStyle {
                    width: Self::DEFAULT_SID_WIDTH,
                    style: Self::DEFAULT_SID_STYLE.to_string(),
                },
                |item| LineStyle {
                    width: item.line_weight,
                    style: item.line_style.clone(),
                },
            ),
            star: star.map_or(
                LineStyle {
                    width: Self::DEFAULT_STAR_WIDTH,
                    style: Self::DEFAULT_STAR_STYLE.to_string(),
                },
                |item| LineStyle {
                    width: item.line_weight,
                    style: item.line_style.clone(),
                },
            ),
            high_airways: high_airways.map_or(
                LineStyle {
                    width: Self::DEFAULT_HIGH_AIRWAY_WIDTH,
                    style: Self::DEFAULT_HIGH_AIRWAY_STYLE.to_string(),
                },
                |item| LineStyle {
                    width: item.line_weight,
                    style: item.line_style.clone(),
                },
            ),
            low_airways: low_airways.map_or(
                LineStyle {
                    width: Self::DEFAULT_LOW_AIRWAY_WIDTH,
                    style: Self::DEFAULT_LOW_AIRWAY_STYLE.to_string(),
                },
                |item| LineStyle {
                    width: item.line_weight,
                    style: item.line_style.clone(),
                },
            ),
            artcc_low: artcc_low.map_or(
                LineStyle {
                    width: Self::DEFAULT_ARTCC_LOW_WIDTH,
                    style: Self::DEFAULT_ARTCC_LOW_STYLE.to_string(),
                },
                |item| LineStyle {
                    width: item.line_weight,
                    style: item.line_style.clone(),
                },
            ),
            artcc: artcc.map_or(
                LineStyle {
                    width: Self::DEFAULT_ARTCC_WIDTH,
                    style: Self::DEFAULT_ARTCC_STYLE.to_string(),
                },
                |item| LineStyle {
                    width: item.line_weight,
                    style: item.line_style.clone(),
                },
            ),
            artcc_high: artcc_high.map_or(
                LineStyle {
                    width: Self::DEFAULT_ARTCC_HIGH_WIDTH,
                    style: Self::DEFAULT_ARTCC_HIGH_STYLE.to_string(),
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
            geo: LineStyle {
                width: Self::DEFAULT_GEO_WIDTH,
                style: Self::DEFAULT_GEO_STYLE.to_string(),
            },
            sid: LineStyle {
                width: Self::DEFAULT_SID_WIDTH,
                style: Self::DEFAULT_SID_STYLE.to_string(),
            },
            star: LineStyle {
                width: Self::DEFAULT_STAR_WIDTH,
                style: Self::DEFAULT_STAR_STYLE.to_string(),
            },
            high_airways: LineStyle {
                width: Self::DEFAULT_HIGH_AIRWAY_WIDTH,
                style: Self::DEFAULT_HIGH_AIRWAY_STYLE.to_string(),
            },
            low_airways: LineStyle {
                width: Self::DEFAULT_LOW_AIRWAY_WIDTH,
                style: Self::DEFAULT_LOW_AIRWAY_STYLE.to_string(),
            },
            artcc_low: LineStyle {
                width: Self::DEFAULT_ARTCC_LOW_WIDTH,
                style: Self::DEFAULT_ARTCC_LOW_STYLE.to_string(),
            },
            artcc: LineStyle {
                width: Self::DEFAULT_ARTCC_WIDTH,
                style: Self::DEFAULT_ARTCC_STYLE.to_string(),
            },
            artcc_high: LineStyle {
                width: Self::DEFAULT_ARTCC_HIGH_WIDTH,
                style: Self::DEFAULT_ARTCC_HIGH_STYLE.to_string(),
            },
        }
    }
}

/// settings for .sct labels activated in .asr
#[derive(Clone, Debug, Serialize, Deserialize)]
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
            fix_font_size: fix.map_or(Self::DEFAULT_FONT_SIZE, |item| item.font_size_symbol_scale)
                * EUROSCOPE_FONT_SIZE_FACTOR,
            airport_alignment: airport.map(|item| item.text_alignment).unwrap_or_default(),
            airport_font_size: airport
                .map_or(Self::DEFAULT_FONT_SIZE, |item| item.font_size_symbol_scale)
                * EUROSCOPE_FONT_SIZE_FACTOR,
            runway_alignment: runway.map(|item| item.text_alignment).unwrap_or_default(),
            runway_font_size: runway
                .map_or(Self::DEFAULT_FONT_SIZE, |item| item.font_size_symbol_scale)
                * EUROSCOPE_FONT_SIZE_FACTOR,
            vor_alignment: vor.map(|item| item.text_alignment).unwrap_or_default(),
            vor_font_size: vor.map_or(Self::DEFAULT_FONT_SIZE, |item| item.font_size_symbol_scale)
                * EUROSCOPE_FONT_SIZE_FACTOR,
            ndb_alignment: ndb.map(|item| item.text_alignment).unwrap_or_default(),
            ndb_font_size: ndb.map_or(Self::DEFAULT_FONT_SIZE, |item| item.font_size_symbol_scale)
                * EUROSCOPE_FONT_SIZE_FACTOR,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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

/// Override of ABI/ACT/REV send-window timing.
///
/// Falls through to the next-less-specific level (constraint > FIR > global) when
/// `None`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CoordinationTiming {
    pub act_time: Option<Time>,
    pub abi_time: Option<Time>,
    pub rev_time: Option<Time>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedCoordinationTiming {
    pub act_time: Time,
    pub abi_time: Time,
    pub rev_time: Time,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoordinationSettings {
    pub act_time: Time,
    pub abi_time: Time,
    pub rev_time: Time,
    /// LAM acknowledgement timeout
    pub lam_timeout: Time,
    /// Per-partner-FIR overrides, keyed by the partner FIR.
    #[serde(default)]
    pub fir_overrides: HashMap<String, CoordinationTiming>,
}

impl CoordinationSettings {
    const DEFAULT_ACT_TIME_MIN: f32 = 15.0;
    const DEFAULT_ABI_TIME_MIN: f32 = 30.0;
    const DEFAULT_REV_TIME_MIN: f32 = 5.0;
    const DEFAULT_LAM_TIMEOUT_SEC: f32 = 5.0;

    /// Resolves the effective ABI/ACT/REV timing for a given partner FIR and/or a
    /// specific matched `Constraint`'s own override.
    #[must_use]
    pub fn resolve(
        &self,
        fir: Option<&str>,
        constraint_timing: Option<&CoordinationTiming>,
    ) -> ResolvedCoordinationTiming {
        let fir_override = fir.and_then(|f| self.fir_overrides.get(f));

        let pick = |select: fn(&CoordinationTiming) -> Option<Time>, default: Time| {
            constraint_timing
                .and_then(select)
                .or_else(|| fir_override.and_then(select))
                .unwrap_or(default)
        };

        ResolvedCoordinationTiming {
            act_time: pick(|t| t.act_time, self.act_time),
            abi_time: pick(|t| t.abi_time, self.abi_time),
            rev_time: pick(|t| t.rev_time, self.rev_time),
        }
    }
}

impl Default for CoordinationSettings {
    fn default() -> Self {
        Self {
            act_time: Time::new::<minute>(Self::DEFAULT_ACT_TIME_MIN),
            abi_time: Time::new::<minute>(Self::DEFAULT_ABI_TIME_MIN),
            rev_time: Time::new::<minute>(Self::DEFAULT_REV_TIME_MIN),
            lam_timeout: Time::new::<second>(Self::DEFAULT_LAM_TIMEOUT_SEC),
            fir_overrides: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Settings {
    // text, enabled, divergence, delay(?),  ...?
    // clam: ClamSettings,
    // ram: RamSettings,
    pub maps: MapsSettings,
    pub track: TrackSettings,
    pub coopans: bool,
    pub ssr: SsrSettings,
    pub asr_files: Vec<PathBuf>,
    pub coordination: CoordinationSettings,
}

impl Settings {
    pub fn from_euroscope(
        symbology: &Symbology,
        topsky: Option<&Topsky>,
        squawks: Option<&SquawksJson>,
        prf: &Prf,
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
            asr_files: (1..10).filter_map(|i| prf.recent_path(i)).collect(),
            coordination: CoordinationSettings::default(),
        }
    }
}

#[cfg(test)]
mod coordination_settings_tests {
    use test_log::test;
    use uom::si::f32::Time;
    use uom::si::time::{minute, second};

    use super::{CoordinationSettings, CoordinationTiming};

    fn settings() -> CoordinationSettings {
        CoordinationSettings::default()
    }

    #[test]
    fn no_overrides_resolves_to_global_defaults() {
        let s = settings();
        let resolved = s.resolve(None, None);
        assert_eq!(resolved.act_time, s.act_time);
        assert_eq!(resolved.abi_time, s.abi_time);
        assert_eq!(resolved.rev_time, s.rev_time);
    }

    #[test]
    fn fir_override_wins_over_global_per_field() {
        let mut s = settings();
        s.fir_overrides.insert(
            "EDMM".to_string(),
            CoordinationTiming {
                act_time: Some(Time::new::<minute>(20.0)),
                abi_time: None,
                rev_time: None,
            },
        );

        let resolved = s.resolve(Some("EDMM"), None);
        assert_eq!(resolved.act_time, Time::new::<minute>(20.0));
        assert_eq!(resolved.abi_time, s.abi_time);
        assert_eq!(resolved.rev_time, s.rev_time);
    }

    #[test]
    fn constraint_timing_wins_over_fir_and_global() {
        let mut s = settings();
        s.fir_overrides.insert(
            "EDMM".to_string(),
            CoordinationTiming {
                act_time: Some(Time::new::<minute>(20.0)),
                abi_time: Some(Time::new::<minute>(40.0)),
                rev_time: None,
            },
        );
        let constraint_timing = CoordinationTiming {
            act_time: Some(Time::new::<minute>(10.0)),
            abi_time: None,
            rev_time: None,
        };

        let resolved = s.resolve(Some("EDMM"), Some(&constraint_timing));
        assert_eq!(
            resolved.act_time,
            Time::new::<minute>(10.0),
            "constraint override wins over FIR override"
        );
        assert_eq!(
            resolved.abi_time,
            Time::new::<minute>(40.0),
            "unset constraint field falls through to FIR override"
        );
        assert_eq!(
            resolved.rev_time, s.rev_time,
            "unset in both constraint and FIR falls through to global"
        );
    }

    #[test]
    fn unresolved_fir_and_constraint_falls_back_to_global() {
        let s = settings();
        let resolved = s.resolve(None, None);
        assert_eq!(resolved.act_time, s.act_time);
        assert_eq!(resolved.abi_time, s.abi_time);
        assert_eq!(resolved.rev_time, s.rev_time);
    }

    #[test]
    fn serde_round_trip_preserves_fir_overrides() {
        let mut s = settings();
        s.fir_overrides.insert(
            "EDMM".to_string(),
            CoordinationTiming {
                act_time: Some(Time::new::<second>(1200.0)),
                abi_time: None,
                rev_time: None,
            },
        );

        let json = serde_json::to_string(&s).unwrap();
        let round_tripped: CoordinationSettings = serde_json::from_str(&json).unwrap();

        assert_eq!(
            round_tripped.fir_overrides.get("EDMM").unwrap().act_time,
            Some(Time::new::<second>(1200.0))
        );
    }
}
