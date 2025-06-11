use std::{collections::HashMap, num::TryFromIntError, sync::OnceLock};

use bevy_reflect::Reflect;
use regex::Regex;
use serde::{de::Visitor, Deserialize, Serialize};

use crate::{
    sct::Sct,
    symbology::Symbology,
    topsky::{Topsky, DEFAULT_COLOURS},
};

use super::settings::Settings;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Reflect)]
pub struct Colour {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Colour {
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub fn from_euroscope(colour_num: i32) -> Result<Self, TryFromIntError> {
        Ok(Self::from_rgb(
            u8::try_from(colour_num % 256)?,
            u8::try_from(colour_num / 256 % 256)?,
            u8::try_from(colour_num / 256 / 256)?,
        ))
    }

    pub fn to_euroscope(&self) -> i32 {
        i32::from(self.r) + i32::from(self.g) * 256 + i32::from(self.b) * 256 * 256
    }

    fn from_symbology(symbology: &Symbology, key: &(String, String), default: Colour) -> Self {
        symbology
            .items
            .0
            .get(key)
            .map_or(default, |item| item.colour)
    }

    fn from_topsky_default(settings: &Settings, key: &str) -> Option<Self> {
        DEFAULT_COLOURS
            .get(key)
            .and_then(|tuples| if settings.coopans { tuples.1 } else { tuples.0 })
            .map(|(r, g, b)| Colour::from_rgb(r, g, b))
    }
}

struct ColourVisitor;

fn hash_rgb_regex() -> &'static Regex {
    static HASH_RGB_RE: OnceLock<Regex> = OnceLock::new();
    HASH_RGB_RE
        .get_or_init(|| Regex::new(r"^#([0-9a-fA-F]{2})([0-9a-fA-F]{2})([0-9a-fA-F]{2})$").unwrap())
}

impl Visitor<'_> for ColourVisitor {
    type Value = Colour;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        // TODO should be able to deserialise other formats, too
        formatter.write_str("A colour in the format #rrggbb")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if let Some(((r, g), b)) = hash_rgb_regex().captures(v).and_then(|captures| {
            let r = u8::from_str_radix(&captures[1], 16);
            let g = u8::from_str_radix(&captures[2], 16);
            let b = u8::from_str_radix(&captures[3], 16);
            r.ok().zip(g.ok()).zip(b.ok())
        }) {
            return Ok(Colour::from_rgb(r, g, b));
        }

        Err(serde::de::Error::invalid_type(
            serde::de::Unexpected::Str(v),
            &self,
        ))
    }
}

impl<'de> Deserialize<'de> for Colour {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(ColourVisitor)
    }
}

/// Colours used in the ASR map
#[derive(Clone, Debug, Serialize)]
pub struct MapColours {
    pub fix_symbol: Colour,
    pub fix_name: Colour,
    pub airport_symbol: Colour,
    pub airport_name: Colour,
    pub ndb_symbol: Colour,
    pub ndb_name: Colour,
    pub ndb_frequency: Colour,
    pub vor_symbol: Colour,
    pub vor_name: Colour,
    pub vor_frequency: Colour,
    pub low_airway_line: Colour,
    pub low_airway_name: Colour,
    pub high_airway_line: Colour,
    pub high_airway_name: Colour,
    pub sid: Colour,
    pub star: Colour,
    pub artcc_boundary: Colour,
    pub artcc_low_boundary: Colour,
    pub artcc_high_boundary: Colour,
    pub geo: Colour,
    pub runway_centreline: Colour,
    pub runway_extended_centreline: Colour,
    pub runway_name: Colour,
    pub free_text: Colour,
}
impl MapColours {
    const DEFAULT_FIX_SYMBOL: Colour = Colour::from_rgb(0, 0, 0);
    const DEFAULT_FIX_NAME: Colour = Colour::from_rgb(0, 0, 0);
    const DEFAULT_AIRPORT_SYMBOL: Colour = Colour::from_rgb(0, 0, 0);
    const DEFAULT_AIRPORT_NAME: Colour = Colour::from_rgb(0, 0, 0);
    const DEFAULT_NDB_SYMBOL: Colour = Colour::from_rgb(0, 0, 0);
    const DEFAULT_NDB_NAME: Colour = Colour::from_rgb(0, 0, 0);
    const DEFAULT_NDB_FREQUENCY: Colour = Colour::from_rgb(0, 0, 0);
    const DEFAULT_VOR_SYMBOL: Colour = Colour::from_rgb(0, 0, 0);
    const DEFAULT_VOR_NAME: Colour = Colour::from_rgb(0, 0, 0);
    const DEFAULT_VOR_FREQUENCY: Colour = Colour::from_rgb(0, 0, 0);
    const DEFAULT_LOW_AIRWAY_LINE: Colour = Colour::from_rgb(0, 0, 0);
    const DEFAULT_LOW_AIRWAY_NAME: Colour = Colour::from_rgb(0, 0, 0);
    const DEFAULT_HIGH_AIRWAY_LINE: Colour = Colour::from_rgb(0, 0, 0);
    const DEFAULT_HIGH_AIRWAY_NAME: Colour = Colour::from_rgb(0, 0, 0);
    const DEFAULT_SID: Colour = Colour::from_rgb(0, 0, 0);
    const DEFAULT_STAR: Colour = Colour::from_rgb(0, 0, 0);
    const DEFAULT_ARTCC_BOUNDARY: Colour = Colour::from_rgb(0, 0, 0);
    const DEFAULT_ARTCC_LOW_BOUNDARY: Colour = Colour::from_rgb(0, 0, 0);
    const DEFAULT_ARTCC_HIGH_BOUNDARY: Colour = Colour::from_rgb(0, 0, 0);
    const DEFAULT_GEO: Colour = Colour::from_rgb(0, 0, 0);
    const DEFAULT_RUNWAY_CENTRELINE: Colour = Colour::from_rgb(0, 0, 0);
    const DEFAULT_RUNWAY_EXTENDED_CENTRELINE: Colour = Colour::from_rgb(0, 0, 0);
    const DEFAULT_RUNWAY_NAME: Colour = Colour::from_rgb(0, 0, 0);
    const DEFAULT_FREE_TEXT: Colour = Colour::from_rgb(0, 0, 0);

    pub fn from_euroscope(symbology: &Symbology) -> Self {
        Self {
            fix_symbol: Colour::from_symbology(
                symbology,
                &("Fixes".to_string(), "symbol".to_string()),
                Self::DEFAULT_FIX_SYMBOL,
            ),
            fix_name: Colour::from_symbology(
                symbology,
                &("Fixes".to_string(), "name".to_string()),
                Self::DEFAULT_FIX_NAME,
            ),
            airport_symbol: Colour::from_symbology(
                symbology,
                &("Airports".to_string(), "symbol".to_string()),
                Self::DEFAULT_AIRPORT_SYMBOL,
            ),
            airport_name: Colour::from_symbology(
                symbology,
                &("Airports".to_string(), "name".to_string()),
                Self::DEFAULT_AIRPORT_NAME,
            ),
            ndb_symbol: Colour::from_symbology(
                symbology,
                &("NDBs".to_string(), "symbol".to_string()),
                Self::DEFAULT_NDB_SYMBOL,
            ),
            ndb_name: Colour::from_symbology(
                symbology,
                &("NDBs".to_string(), "name".to_string()),
                Self::DEFAULT_NDB_NAME,
            ),
            ndb_frequency: Colour::from_symbology(
                symbology,
                &("NDBs".to_string(), "frequency".to_string()),
                Self::DEFAULT_NDB_FREQUENCY,
            ),
            vor_symbol: Colour::from_symbology(
                symbology,
                &("VORs".to_string(), "symbol".to_string()),
                Self::DEFAULT_VOR_SYMBOL,
            ),
            vor_name: Colour::from_symbology(
                symbology,
                &("VORs".to_string(), "name".to_string()),
                Self::DEFAULT_VOR_NAME,
            ),
            vor_frequency: Colour::from_symbology(
                symbology,
                &("VORs".to_string(), "frequency".to_string()),
                Self::DEFAULT_VOR_FREQUENCY,
            ),
            low_airway_line: Colour::from_symbology(
                symbology,
                &("Low airways".to_string(), "line".to_string()),
                Self::DEFAULT_LOW_AIRWAY_LINE,
            ),
            low_airway_name: Colour::from_symbology(
                symbology,
                &("Low airways".to_string(), "name".to_string()),
                Self::DEFAULT_LOW_AIRWAY_NAME,
            ),
            high_airway_line: Colour::from_symbology(
                symbology,
                &("High airways".to_string(), "line".to_string()),
                Self::DEFAULT_HIGH_AIRWAY_LINE,
            ),
            high_airway_name: Colour::from_symbology(
                symbology,
                &("High airways".to_string(), "line".to_string()),
                Self::DEFAULT_HIGH_AIRWAY_NAME,
            ),
            sid: Colour::from_symbology(
                symbology,
                &("Sids".to_string(), "line".to_string()),
                Self::DEFAULT_SID,
            ),
            star: Colour::from_symbology(
                symbology,
                &("Stars".to_string(), "line".to_string()),
                Self::DEFAULT_STAR,
            ),
            artcc_boundary: Colour::from_symbology(
                symbology,
                &("ARTCC boundary".to_string(), "line".to_string()),
                Self::DEFAULT_ARTCC_BOUNDARY,
            ),
            artcc_low_boundary: Colour::from_symbology(
                symbology,
                &("ARTCC low boundary".to_string(), "line".to_string()),
                Self::DEFAULT_ARTCC_LOW_BOUNDARY,
            ),
            artcc_high_boundary: Colour::from_symbology(
                symbology,
                &("ARTCC high boundary".to_string(), "line".to_string()),
                Self::DEFAULT_ARTCC_HIGH_BOUNDARY,
            ),
            geo: Colour::from_symbology(
                symbology,
                &("Geo".to_string(), "line".to_string()),
                Self::DEFAULT_GEO,
            ),
            runway_centreline: Colour::from_symbology(
                symbology,
                &("Runways".to_string(), "centerline".to_string()),
                Self::DEFAULT_RUNWAY_CENTRELINE,
            ),
            runway_extended_centreline: Colour::from_symbology(
                symbology,
                &("Runways".to_string(), "extended centerline".to_string()),
                Self::DEFAULT_RUNWAY_EXTENDED_CENTRELINE,
            ),
            runway_name: Colour::from_symbology(
                symbology,
                &("Runways".to_string(), "name".to_string()),
                Self::DEFAULT_RUNWAY_NAME,
            ),
            free_text: Colour::from_symbology(
                symbology,
                &("Other".to_string(), "freetext".to_string()),
                Self::DEFAULT_FREE_TEXT,
            ),
        }
    }
}
impl Default for MapColours {
    fn default() -> Self {
        Self {
            fix_symbol: Self::DEFAULT_FIX_SYMBOL,
            fix_name: Self::DEFAULT_FIX_NAME,
            airport_symbol: Self::DEFAULT_AIRPORT_SYMBOL,
            airport_name: Self::DEFAULT_AIRPORT_NAME,
            ndb_symbol: Self::DEFAULT_NDB_SYMBOL,
            ndb_name: Self::DEFAULT_NDB_NAME,
            ndb_frequency: Self::DEFAULT_NDB_FREQUENCY,
            vor_symbol: Self::DEFAULT_VOR_SYMBOL,
            vor_name: Self::DEFAULT_VOR_NAME,
            vor_frequency: Self::DEFAULT_VOR_FREQUENCY,
            low_airway_line: Self::DEFAULT_LOW_AIRWAY_LINE,
            low_airway_name: Self::DEFAULT_LOW_AIRWAY_NAME,
            high_airway_line: Self::DEFAULT_HIGH_AIRWAY_LINE,
            high_airway_name: Self::DEFAULT_HIGH_AIRWAY_NAME,
            sid: Self::DEFAULT_SID,
            star: Self::DEFAULT_STAR,
            artcc_boundary: Self::DEFAULT_ARTCC_BOUNDARY,
            artcc_low_boundary: Self::DEFAULT_ARTCC_LOW_BOUNDARY,
            artcc_high_boundary: Self::DEFAULT_ARTCC_HIGH_BOUNDARY,
            geo: Self::DEFAULT_GEO,
            runway_centreline: Self::DEFAULT_RUNWAY_CENTRELINE,
            runway_extended_centreline: Self::DEFAULT_RUNWAY_EXTENDED_CENTRELINE,
            runway_name: Self::DEFAULT_RUNWAY_NAME,
            free_text: Self::DEFAULT_FREE_TEXT,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SectorColours {
    pub active_background: Colour,
    pub inactive_background: Colour,
}
impl SectorColours {
    const DEFAULT_ACTIVE_BACKGROUND: Colour = Colour::from_rgb(200, 200, 200);
    const DEFAULT_INACTIVE_BACKGROUND: Colour = Colour::from_rgb(200, 200, 200);

    pub fn from_euroscope(symbology: &Symbology) -> Self {
        Self {
            active_background: Colour::from_symbology(
                symbology,
                &("Sector".to_string(), "active sector background".to_string()),
                Self::DEFAULT_ACTIVE_BACKGROUND,
            ),
            inactive_background: Colour::from_symbology(
                symbology,
                &(
                    "Sector".to_string(),
                    "inactive sector background".to_string(),
                ),
                Self::DEFAULT_INACTIVE_BACKGROUND,
            ),
        }
    }
}
impl Default for SectorColours {
    fn default() -> Self {
        Self {
            active_background: Self::DEFAULT_ACTIVE_BACKGROUND,
            inactive_background: Self::DEFAULT_INACTIVE_BACKGROUND,
        }
    }
}

// TODO Euroscope colour fallback?
#[derive(Clone, Debug, Serialize)]
pub struct TrackColours {
    pub assumed: Colour,
    pub advanced: Colour,
    pub concerned: Colour,
    pub unconcerned: Colour,
    pub flight_leg: Colour,
    pub predicted_alert: Colour,
    pub current_alert: Colour,
    pub reaction: Colour,
    pub cleared: Colour,
    // FIXME
    pub vfr: Colour,
}
impl TrackColours {
    const DEFAULT_ASSUMED: Colour = Colour::from_rgb(0, 0, 0);
    const DEFAULT_ADVANCED: Colour = Colour::from_rgb(0, 0, 255);
    const DEFAULT_CONCERNED: Colour = Colour::from_rgb(0, 0, 255);
    const DEFAULT_UNCONCERNED: Colour = Colour::from_rgb(255, 255, 255);
    const DEFAULT_VFR: Colour = Colour::from_rgb(180, 150, 95);
    const DEFAULT_FLIGHT_LEG: Colour = Colour::from_rgb(0, 0, 0);
    const DEFAULT_PREDICTED_ALERT: Colour = Colour::from_rgb(255, 255, 0);
    const DEFAULT_CURRENT_ALERT: Colour = Colour::from_rgb(255, 0, 0);
    const DEFAULT_REACTION: Colour = Colour::from_rgb(230, 185, 184);
    const DEFAULT_CLEARED: Colour = Colour::from_rgb(34, 139, 34);

    pub fn from_euroscope(topsky_colours: &HashMap<String, Colour>, settings: &Settings) -> Self {
        Self {
            assumed: topsky_colours
                .get("Assumed")
                .copied()
                .or(Colour::from_topsky_default(settings, "Assumed"))
                .unwrap_or(Self::DEFAULT_ASSUMED),
            advanced: topsky_colours
                .get("Concerned")
                .copied()
                .or(Colour::from_topsky_default(settings, "Concerned"))
                .unwrap_or(Self::DEFAULT_ADVANCED),
            concerned: topsky_colours
                .get("Redundant")
                .copied()
                .or(Colour::from_topsky_default(settings, "Redundant"))
                .unwrap_or(Self::DEFAULT_CONCERNED),
            unconcerned: topsky_colours
                .get("Unconcerned")
                .copied()
                .or(Colour::from_topsky_default(settings, "Unconcerned"))
                .unwrap_or(Self::DEFAULT_UNCONCERNED),
            vfr: topsky_colours
                .get("VFR")
                .copied()
                .or(Colour::from_topsky_default(settings, "VFR"))
                .unwrap_or(Self::DEFAULT_VFR),
            flight_leg: topsky_colours
                .get("Flight_Leg")
                .copied()
                .or(Colour::from_topsky_default(settings, "Flight_Leg"))
                .unwrap_or(Self::DEFAULT_FLIGHT_LEG),
            predicted_alert: topsky_colours
                .get("Warning")
                .copied()
                .or(Colour::from_topsky_default(settings, "Warning"))
                .unwrap_or(Self::DEFAULT_PREDICTED_ALERT),
            current_alert: topsky_colours
                .get("Urgency")
                .copied()
                .or(Colour::from_topsky_default(settings, "Urgency"))
                .unwrap_or(Self::DEFAULT_CURRENT_ALERT),
            reaction: topsky_colours
                .get("Proposition_In")
                .copied()
                .or(Colour::from_topsky_default(settings, "Proposition_In"))
                .unwrap_or(Self::DEFAULT_REACTION),
            cleared: topsky_colours
                .get("Rwy_Locked")
                .copied()
                .or(Colour::from_topsky_default(settings, "Rwy_Locked"))
                .unwrap_or(Self::DEFAULT_CLEARED),
        }
    }
}

impl Default for TrackColours {
    fn default() -> Self {
        Self {
            assumed: Self::DEFAULT_ASSUMED,
            advanced: Self::DEFAULT_ADVANCED,
            concerned: Self::DEFAULT_CONCERNED,
            unconcerned: Self::DEFAULT_UNCONCERNED,
            vfr: Self::DEFAULT_VFR,
            flight_leg: Self::DEFAULT_FLIGHT_LEG,
            predicted_alert: Self::DEFAULT_PREDICTED_ALERT,
            current_alert: Self::DEFAULT_CURRENT_ALERT,
            reaction: Self::DEFAULT_REACTION,
            cleared: Self::DEFAULT_CLEARED,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct UIColours {
    pub foreground: Colour,
    pub background: Colour,
    // TODO check use for better name (disabled/checkbox background)
    pub armed: Colour,
    pub window_title_background_selected: Colour,
    pub text_input: Colour,
    pub warning: Colour,
    pub alert: Colour,
}
impl UIColours {
    const DEFAULT_ARMED: Colour = Colour::from_rgb(160, 160, 160);
    const DEFAULT_BACKGROUND: Colour = Colour::from_rgb(192, 192, 192);
    const DEFAULT_FOREGROUND: Colour = Colour::from_rgb(0, 0, 0);
    const DEFAULT_WINDOW_TITLE_BACKGROUND_SELECTED: Colour = Colour::from_rgb(192, 192, 192);
    const DEFAULT_TEXT_INPUT: Colour = Colour::from_rgb(170, 224, 224);
    const DEFAULT_WARNING: Colour = Colour::from_rgb(255, 255, 0);
    const DEFAULT_ALERT: Colour = Colour::from_rgb(255, 0, 0);

    pub fn from_euroscope(topsky_colours: &HashMap<String, Colour>, settings: &Settings) -> Self {
        Self {
            armed: topsky_colours
                .get("Arm")
                .copied()
                .or(Colour::from_topsky_default(settings, "Arm"))
                .unwrap_or(Self::DEFAULT_ARMED),
            background: topsky_colours
                .get("Background")
                .copied()
                .or(Colour::from_topsky_default(settings, "Background"))
                .unwrap_or(Self::DEFAULT_BACKGROUND),
            foreground: topsky_colours
                .get("Foreground")
                .copied()
                .or(Colour::from_topsky_default(settings, "Foreground"))
                .unwrap_or(Self::DEFAULT_FOREGROUND),
            window_title_background_selected: topsky_colours
                .get("Background")
                .copied()
                .or(Colour::from_topsky_default(settings, "Background"))
                .unwrap_or(Self::DEFAULT_WINDOW_TITLE_BACKGROUND_SELECTED),
            warning: topsky_colours
                .get("Warning")
                .copied()
                .or(Colour::from_topsky_default(settings, "Warning"))
                .unwrap_or(Self::DEFAULT_WARNING),
            alert: topsky_colours
                .get("Urgency")
                .copied()
                .or(Colour::from_topsky_default(settings, "Urgency"))
                .unwrap_or(Self::DEFAULT_ALERT),
            text_input: topsky_colours
                .get("Field_Highlight")
                .copied()
                .or(Colour::from_topsky_default(settings, "Field_Highlight"))
                .unwrap_or(Self::DEFAULT_TEXT_INPUT),
        }
    }
}

impl Default for UIColours {
    fn default() -> Self {
        Self {
            armed: Self::DEFAULT_ARMED,
            background: Self::DEFAULT_BACKGROUND,
            foreground: Self::DEFAULT_FOREGROUND,
            window_title_background_selected: Self::DEFAULT_WINDOW_TITLE_BACKGROUND_SELECTED,
            text_input: Self::DEFAULT_TEXT_INPUT,
            warning: Self::DEFAULT_WARNING,
            alert: Self::DEFAULT_ALERT,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Colours {
    pub track: TrackColours,
    pub sector: SectorColours,
    pub map: MapColours,
    pub ui: UIColours,
    other: HashMap<String, Colour>,
}

impl Colours {
    // topsky and sct colours referenced by name in maps
    pub fn get(&self, name: &str, settings: &Settings) -> Option<Colour> {
        self.other
            .get(name)
            .copied()
            .or_else(|| Colour::from_topsky_default(settings, name))
    }

    pub fn from_euroscope(
        symbology: &Symbology,
        sct: &Sct,
        topsky: &Option<Topsky>,
        settings: &Settings,
    ) -> Self {
        let topsky_colours = topsky
            .as_ref()
            .map(|t| {
                t.colours
                    .iter()
                    .map(|(key, colour)| (key.clone(), colour.colour))
                    .collect()
            })
            .unwrap_or_default();
        Self {
            map: MapColours::from_euroscope(symbology),
            sector: SectorColours::from_euroscope(symbology),
            track: TrackColours::from_euroscope(&topsky_colours, settings),
            ui: UIColours::from_euroscope(&topsky_colours, settings),
            other: sct
                .colours
                .clone()
                .into_iter()
                .chain(topsky_colours)
                .collect(),
        }
    }
}
