use std::fmt::Display;

use serde::Serialize;

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct Airline {
    pub designator: String,
    pub airline: String,
    pub callsign: String,
    pub country: String,
}

#[derive(Copy, Clone, Debug, Serialize, PartialEq)]
pub enum Wtc {
    LIGHT,
    MEDIUM,
    HEAVY,
    SUPER,
    UNKNOWN,
}

impl Wtc {
    pub fn parse(wtc_string: &str) -> Self {
        assert!(wtc_string.len() == 1);
        match wtc_string.as_bytes()[0] {
            b'L' => Self::LIGHT,
            b'M' => Self::MEDIUM,
            b'H' => Self::HEAVY,
            b'J' => Self::SUPER,
            _ => Self::UNKNOWN,
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

impl AircraftType {
    pub fn parse(wtc_string: &str) -> Self {
        assert!(wtc_string.len() == 1);
        match wtc_string.as_bytes()[0] {
            b'L' => Self::LANDPLANE,
            b'S' => Self::SEAPLANE,
            b'A' => Self::AMPHIBIAN,
            b'G' => Self::GYROCOPTER,
            b'H' => Self::HELICOPTER,
            b'T' => Self::TILTROTOR,
            _ => Self::UNKNOWN,
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

impl EngineType {
    pub fn parse(enginetype_string: &str) -> Self {
        match enginetype_string.as_bytes()[0] {
            b'J' => Self::JET,
            b'T' => Self::TURBOPROP,
            b'P' => Self::PISTON,
            b'E' => Self::ELECTRIC,
            b'R' => Self::ROCKET,
            _ => Self::UNKNOWN,
        }
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
