use std::{collections::HashMap, fmt::Display, hash::Hash, io};

use bevy_derive::{Deref, DerefMut};
use bevy_reflect::Reflect;
use geo::{Coord, Point};
use multimap::MultiMap;
use serde::{Deserialize, Serialize, Serializer};
use tracing::warn;

pub mod adaptation;
pub mod airway;
pub mod asr;
pub mod ese;
pub mod icao_aircraft;
pub mod icao_airlines;
pub mod icao_airports;
pub mod isec;
pub mod prf;
pub mod sct;
pub mod squawks;
pub mod symbology;
pub mod topsky;

fn read_to_string(contents: &[u8]) -> Result<String, io::Error> {
    String::from_utf8(contents.to_vec()).or_else(|_| {
        let (string, _, errors) = encoding_rs::WINDOWS_1252.decode(contents);
        if errors {
            warn!("errors while decoding win-1252");
        }
        Ok(string.to_string())
    })
}

// deg: i16, because, i.e. UK uses invalid coordinates for dummy data: S999.00.00.000 E999.00.00.000
#[derive(Copy, Clone, Debug, Default)]
pub enum Sign {
    #[default]
    Pos,
    Neg,
}

impl Sign {
    fn sign(self) -> i8 {
        match self {
            Self::Pos => 1,
            Self::Neg => -1,
        }
    }
}

impl From<&str> for Sign {
    fn from(hemi: &str) -> Self {
        match hemi {
            "N" | "E" | "n" | "e" => Sign::Pos,
            "S" | "W" | "s" | "w" => Sign::Neg,
            _ => unreachable!("{hemi} is not a hemisphere"),
        }
    }
}

type DegMinSec = (Sign, u16, u8, f64);

fn decimal_to_dms(decimal: f64, is_latitude: bool) -> (u8, u8, f64, char) {
    let is_negative = decimal.is_sign_negative();
    let abs = decimal.abs();

    let degrees = abs.floor() as u8;
    let minutes_f64 = abs.fract() * 60.0;
    let minutes = minutes_f64.floor() as u8;
    let seconds = minutes_f64.fract() * 60.0;

    let direction = match (is_latitude, is_negative) {
        (true, false) => 'N',
        (true, true) => 'S',
        (false, false) => 'E',
        (false, true) => 'W',
    };

    (degrees, minutes, seconds, direction)
}

pub trait DegMinSecExt {
    fn from_deg_min_sec(lat: DegMinSec, lng: DegMinSec) -> Self;
    fn lat_deg_min_sec_fmt(&self) -> String;
    fn lng_deg_min_sec_fmt(&self) -> String;
    fn deg_min_sec_fmt(&self) -> String {
        format!(
            "{} {}",
            self.lat_deg_min_sec_fmt(),
            self.lng_deg_min_sec_fmt()
        )
    }
}

impl DegMinSecExt for Coord {
    fn from_deg_min_sec(lat: DegMinSec, lng: DegMinSec) -> Self {
        let lat_deg = f64::from(lat.1);
        let lat_min = f64::from(lat.2);
        let lng_deg = f64::from(lng.1);
        let lng_min = f64::from(lng.2);
        Self {
            y: (lat_deg + lat_min / 60.0 + lat.3 / 3600.0) * f64::from(lat.0.sign()),
            x: (lng_deg + lng_min / 60.0 + lng.3 / 3600.0) * f64::from(lng.0.sign()),
        }
    }

    fn lat_deg_min_sec_fmt(&self) -> String {
        let lat = decimal_to_dms(self.y, true);
        let deg = lat.0;
        let carry_rounded_sec = (lat.2 - 60.).abs() < 0.000_001;
        let min = lat.1 + u8::from(carry_rounded_sec);
        let sec = if carry_rounded_sec { 0.0 } else { lat.2 };
        format!("{}{deg:03}.{min:02}.{sec:06.3}", lat.3)
    }

    fn lng_deg_min_sec_fmt(&self) -> String {
        let lon = decimal_to_dms(self.x, false);
        let deg = lon.0;
        let carry_rounded_sec = (lon.2 - 60.).abs() < 0.000_001;
        let min = lon.1 + u8::from(carry_rounded_sec);
        let sec = if carry_rounded_sec { 0.0 } else { lon.2 };
        format!("{}{deg:03}.{min:02}.{sec:06.3}", lon.3)
    }
}
impl DegMinSecExt for Point {
    fn from_deg_min_sec(lat: DegMinSec, lng: DegMinSec) -> Self {
        Coord::from_deg_min_sec(lat, lng).into()
    }

    fn lat_deg_min_sec_fmt(&self) -> String {
        self.0.lat_deg_min_sec_fmt()
    }

    fn lng_deg_min_sec_fmt(&self) -> String {
        self.0.lng_deg_min_sec_fmt()
    }
}

#[derive(Clone, Debug, Reflect, Serialize, Deserialize, PartialEq)]
#[reflect(Debug)]
pub enum Location {
    Fix(String),
    Coordinate(#[reflect(ignore)] Point),
}

#[derive(Clone, Debug, PartialEq, Eq, Deref, DerefMut)]
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

impl<K1, K2, V> Default for TwoKeyMap<K1, K2, V>
where
    K1: Eq + Hash + Display,
    K2: Eq + Hash + Display,
{
    fn default() -> Self {
        Self(HashMap::default())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deref, DerefMut)]
pub struct TwoKeyMultiMap<K1: Eq + Hash, K2: Eq + Hash, V>(pub MultiMap<(K1, K2), V>);

impl<K1, K2, V> Serialize for TwoKeyMultiMap<K1, K2, V>
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

impl<K1, K2, V> Default for TwoKeyMultiMap<K1, K2, V>
where
    K1: Eq + Hash + Display,
    K2: Eq + Hash + Display,
{
    fn default() -> Self {
        Self(MultiMap::default())
    }
}

#[cfg(test)]
mod test {
    use geo::Coord;

    use crate::{DegMinSecExt as _, Sign};

    #[test]
    fn test_dms_roundtrip() {
        let lat = (Sign::Pos, 48, 40, 0.);
        let lng = (Sign::Pos, 10, 58, 0.5);
        let coord = Coord::from_deg_min_sec(lat, lng);
        let expected = 48.666_666_666_666_666;
        assert!(
            (coord.y - expected).abs() < f64::EPSILON,
            "left: {:?} not equal right: {:?}",
            coord.y,
            expected
        );
        let expected = 10.966_805_555_555_556;
        assert!(
            (coord.x - expected).abs() < f64::EPSILON,
            "left: {:?} not equal right: {:?}",
            coord.x,
            expected
        );
        assert_eq!(coord.deg_min_sec_fmt(), "N048.40.00.000 E010.58.00.500");
    }

    #[test]
    fn test_negative_zero_degree() {
        let lat = (0, 30, 0.0); // S00°30'00"
        let lng = (Sign::Neg, 0, 0, 0.0);

        let coord = Coord::from_deg_min_sec((Sign::Neg, lat.0, lat.1, lat.2), lng);

        assert!(coord.y < 0.0);
    }

    #[test]
    fn test_south_zero_degree() {
        let coord = Coord { x: 0.0, y: -0.5 };
        assert_eq!(coord.lat_deg_min_sec_fmt(), "S000.30.00.000");
    }
}
