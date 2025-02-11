use std::{collections::HashMap, fmt::Display, str::FromStr};

use bevy_derive::{Deref, DerefMut};
use serde::Serialize;

pub use crate::icao_airlines::Airline;

#[derive(Copy, Clone, Debug, Serialize, PartialEq, Default)]
pub enum Wtc {
    LIGHT,
    MEDIUM,
    HEAVY,
    SUPER,
    #[default]
    UNKNOWN,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseWtcError;

impl FromStr for Wtc {
    type Err = ParseWtcError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() == 1 {
            match s.as_bytes()[0] {
                b'L' => Ok(Self::LIGHT),
                b'M' => Ok(Self::MEDIUM),
                b'H' => Ok(Self::HEAVY),
                b'J' => Ok(Self::SUPER),
                b'-' => Ok(Self::UNKNOWN),
                _ => Err(ParseWtcError),
            }
        } else {
            Err(ParseWtcError)
        }
    }
}

impl Display for Wtc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::LIGHT => "L",
            Self::MEDIUM => "M",
            Self::HEAVY => "H",
            Self::SUPER => "J",
            Self::UNKNOWN => "?",
        })
    }
}

#[derive(Copy, Clone, Debug, Serialize, PartialEq)]
pub enum AircraftType {
    LANDPLANE,
    SEAPLANE,
    AMPHIBIAN,
    GYROCOPTER,
    HELICOPTER,
    TILTROTOR,
    UNKNOWN,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseAircraftTypeError;

impl FromStr for AircraftType {
    type Err = ParseAircraftTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() == 1 {
            match s.as_bytes()[0] {
                b'L' => Ok(Self::LANDPLANE),
                b'S' => Ok(Self::SEAPLANE),
                b'A' => Ok(Self::AMPHIBIAN),
                b'G' => Ok(Self::GYROCOPTER),
                b'H' => Ok(Self::HELICOPTER),
                b'T' => Ok(Self::TILTROTOR),
                b'-' => Ok(Self::UNKNOWN),
                _ => Err(ParseAircraftTypeError),
            }
        } else {
            Err(ParseAircraftTypeError)
        }
    }
}

#[derive(Copy, Clone, Debug, Serialize, PartialEq)]
pub enum EngineType {
    JET,
    TURBOPROP,
    PISTON,
    ELECTRIC,
    ROCKET,
    UNKNOWN,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseEngineTypeError;

impl FromStr for EngineType {
    type Err = ParseEngineTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() == 1 {
            match s.as_bytes()[0] {
                b'J' => Ok(Self::JET),
                b'T' => Ok(Self::TURBOPROP),
                b'P' => Ok(Self::PISTON),
                b'E' => Ok(Self::ELECTRIC),
                b'R' => Ok(Self::ROCKET),
                b'-' => Ok(Self::UNKNOWN),
                _ => Err(ParseEngineTypeError),
            }
        } else {
            Err(ParseEngineTypeError)
        }
    }
}

#[derive(Clone, Debug, Default, Deref, DerefMut, Serialize)]
pub struct AircraftMap(pub HashMap<String, Aircraft>);

impl AircraftMap {
    pub fn wtc(&self, ac_type: Option<&str>) -> Wtc {
        ac_type
            .and_then(|ac_type| self.get(ac_type))
            .map_or(Wtc::UNKNOWN, |ac| ac.wtc)
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Aircraft {
    pub designator: String,
    pub wtc: Wtc,
    pub aircrafttype: AircraftType,
    pub num_engines: u8,
    pub enginetype: EngineType,
    pub manufacturer: String,
    pub name: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Airport {
    pub designator: String,
    pub name: String,
    pub country: String,
}
