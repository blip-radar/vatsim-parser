use std::collections::HashMap;

use bevy_reflect::Reflect;
use serde::Serialize;

use crate::{
    symbology::Symbology,
    topsky::{Topsky, DEFAULT_COLOURS},
};

use super::settings::Settings;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Reflect)]
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

    pub fn from_euroscope(colour_num: i32) -> Self {
        Self::from_rgb(
            (colour_num % 256) as u8,
            (colour_num / 256 % 256) as u8,
            (colour_num / 256 / 256) as u8,
        )
    }

    fn from_symbology(symbology: &Symbology, key: (String, String), default: Colour) -> Self {
        symbology
            .items
            .0
            .get(&key)
            .map(|item| item.colour)
            .unwrap_or(default)
    }

    fn from_topsky_default(settings: &Settings, key: &str) -> Option<Self> {
        DEFAULT_COLOURS
            .get(key)
            .and_then(|tuples| if settings.coopans { tuples.1 } else { tuples.0 })
            .map(|(r, g, b)| Colour::from_rgb(r, g, b))
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
                ("Sector".to_string(), "active sector background".to_string()),
                Self::DEFAULT_ACTIVE_BACKGROUND,
            ),
            inactive_background: Colour::from_symbology(
                symbology,
                (
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

#[derive(Clone, Debug, Serialize)]
pub struct TrackColours {
    pub assumed: Colour,
    pub flight_leg: Colour,
    pub predicted_alert: Colour,
    pub current_alert: Colour,
}
impl TrackColours {
    const DEFAULT_ASSUMED: Colour = Colour::from_rgb(0, 0, 0);
    const DEFAULT_FLIGHT_LEG: Colour = Colour::from_rgb(0, 0, 0);
    const DEFAULT_PREDICTED_ALERT: Colour = Colour::from_rgb(255, 140, 0);
    const DEFAULT_CURRENT_ALERT: Colour = Colour::from_rgb(255, 25, 26);

    pub fn from_euroscope(topsky_colours: &HashMap<String, Colour>, settings: &Settings) -> Self {
        Self {
            assumed: topsky_colours
                .get("Assumed")
                .copied()
                .or(Colour::from_topsky_default(settings, "Assumed"))
                .unwrap_or(Self::DEFAULT_ASSUMED),
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
        }
    }
}

impl Default for TrackColours {
    fn default() -> Self {
        Self {
            assumed: Self::DEFAULT_ASSUMED,
            flight_leg: Self::DEFAULT_FLIGHT_LEG,
            predicted_alert: Self::DEFAULT_PREDICTED_ALERT,
            current_alert: Self::DEFAULT_CURRENT_ALERT,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct UIColours {
    pub foreground: Colour,
    pub background: Colour,
    // TODO check use for better name
    pub armed: Colour,
}
impl UIColours {
    const DEFAULT_ARMED: Colour = Colour::from_rgb(120, 120, 120);
    const DEFAULT_BACKGROUND: Colour = Colour::from_rgb(192, 192, 192);
    const DEFAULT_FOREGROUND: Colour = Colour::from_rgb(0, 0, 0);

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
        }
    }
}

impl Default for UIColours {
    fn default() -> Self {
        Self {
            armed: Self::DEFAULT_ARMED,
            background: Self::DEFAULT_BACKGROUND,
            foreground: Self::DEFAULT_FOREGROUND,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Colours {
    pub track: TrackColours,
    pub sector: SectorColours,
    pub ui: UIColours,
    other: HashMap<String, Colour>,
}

impl Colours {
    pub fn get(&self, name: &str, settings: &Settings) -> Option<Colour> {
        self.other
            .get(name)
            .copied()
            .or_else(|| Colour::from_topsky_default(settings, name))
    }

    pub fn from_euroscope(
        symbology: &Symbology,
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
            sector: SectorColours::from_euroscope(symbology),
            track: TrackColours::from_euroscope(&topsky_colours, settings),
            ui: UIColours::from_euroscope(&topsky_colours, settings),
            other: topsky_colours,
        }
    }
}
