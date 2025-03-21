use std::collections::HashMap;

use bevy_derive::{Deref, DerefMut};
use geo::Coord;
use serde::{Serialize, Serializer};

use super::Fix;

/// conceptionally HashMap<Fix, HashMap<Airway, AirwayNeighbours>>
#[derive(Clone, Debug, Default, Deref, DerefMut)]
pub struct FixAirwayMap(pub HashMap<Fix, AirwayNeighboursOfFix>);

impl Serialize for FixAirwayMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let key = |fix: &Fix| {
            format!(
                "{}:{:.6}:{:.6}",
                fix.designator, fix.coordinate.y, fix.coordinate.x
            )
        };
        serializer.collect_map(self.0.iter().map(|(fix, v)| (key(fix), v)))
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct AirwayNeighboursOfFix {
    pub fix: Fix,
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
