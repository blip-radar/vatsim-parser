use std::collections::HashMap;

use geo::Coord;
use serde::Serialize;

/// conceptionally HashMap<Fix, HashMap<Airway, AirwayNeighbours>>
pub type FixAirwayMap = HashMap<String, AirwayNeighboursOfFix>;
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct AirwayNeighboursOfFix {
    pub fix: String,
    pub airway_neighbours: HashMap<String, AirwayNeighbours>,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct AirwayNeighbours {
    pub airway: String,
    pub previous: Option<AirwayFix>,
    pub next: Option<AirwayFix>,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct AirwayFix {
    pub name: String,
    pub coord: Coord,
    pub valid_direction: bool,
    pub minimum_level: Option<u32>,
}
