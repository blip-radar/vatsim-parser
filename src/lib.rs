use std::{collections::HashMap, fmt::Display, hash::Hash, io};

use bevy_reflect::Reflect;
use geo_types::Coord;
use serde::{Serialize, Serializer};

pub mod airway;
pub mod asr;
pub mod ese;
pub mod prf;
pub mod sct;
pub mod symbology;
pub mod topsky;

fn read_to_string(contents: &[u8]) -> Result<String, io::Error> {
    String::from_utf8(contents.to_vec()).or_else(|_| {
        let (string, _, errors) = encoding_rs::WINDOWS_1252.decode(contents);
        if errors {
            eprintln!("errors while decoding win-1252");
        }
        Ok(string.to_string())
    })
}

type DegMinSec = (f64, f64, f64);

trait FromDegMinSec {
    fn from_deg_min_sec(lat: DegMinSec, lng: DegMinSec) -> Self;
}

impl FromDegMinSec for Coord {
    fn from_deg_min_sec(lat: DegMinSec, lng: DegMinSec) -> Self {
        Self {
            y: lat.0 + lat.0.signum() * lat.1 / 60.0 + lat.0.signum() * lat.2 / 3600.0,
            x: lng.0 + lng.0.signum() * lng.1 / 60.0 + lng.0.signum() * lng.2 / 3600.0,
        }
    }
}

#[derive(Clone, Debug, Reflect, Serialize, PartialEq)]
#[reflect(Debug)]
pub enum Location {
    Fix(String),
    Coordinate(#[reflect(ignore)] Coord),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Reflect)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub fn from_euroscope(color_num: i32) -> Self {
        Self::from_rgb(
            (color_num % 256) as u8,
            (color_num / 256 % 256) as u8,
            (color_num / 256 / 256) as u8,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TwoKeyMap<K1: Eq + Hash, K2: Eq + Hash, V>(pub HashMap<(K1, K2), V>);

impl<K1, K2, V> Serialize for TwoKeyMap<K1, K2, V>
where
    K1: Eq + Hash + Display,
    K2: Eq + Hash + Display,
    V: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let key = |k1, k2| format!("{k1}:{k2}");
        serializer.collect_map(self.0.iter().map(|(k, v)| (key(&k.0, &k.1), v)))
    }
}
