use std::{cmp::Ordering, collections::HashMap, fmt::Display};

use bevy_derive::{Deref, DerefMut};
use geo::Coord;
use itertools::Itertools;
use multimap::MultiMap;
use serde::{Serialize, Serializer};

use super::Fix;

/// conceptionally HashMap<Fix, HashMap<Airway, AirwayNeighbours>>
#[derive(Clone, Debug, Default, Deref, DerefMut)]
pub struct FixAirwayMap(pub HashMap<Fix, AirwayNeighboursOfFix>);

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct AirwayNeighboursOfFix {
    pub fix: Fix,
    pub airway_neighbours: MultiMap<String, AirwayNeighbours>,
}

#[derive(Copy, Clone, Debug, Serialize, PartialEq)]
pub enum AirwayType {
    High,
    Low,
    Both,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct AirwayNeighbours {
    pub airway: String,
    pub airway_type: AirwayType,
    pub previous: Option<AirwayFix>,
    pub next: Option<AirwayFix>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct AirwayFix {
    pub designator: String,
    pub coordinate: Coord,
    pub valid_direction: bool,
    pub minimum_level: Option<u32>,
}

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

impl Display for FixAirwayMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (_, airways) in self.0.iter().sorted_by(|(fix, _), (fix2, _)| {
            fix.designator.cmp(&fix2.designator).then(
                fix.coordinate
                    .x
                    .partial_cmp(&fix.coordinate.x)
                    .map_or(Ordering::Equal, Ordering::reverse),
            )
        }) {
            write!(f, "{airways}")?;
        }

        Ok(())
    }
}

impl Display for AirwayNeighboursOfFix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (airway, multi_neighbours) in self
            .airway_neighbours
            .iter_all()
            .sorted_by_key(|(airway, _)| *airway)
        {
            for neighbours in multi_neighbours {
                writeln!(
                    f,
                    "{fix_designator}\t{fix_lat:.6}\t{fix_lng:.6}\t14\t{airway}\t{airway_type}\t{neighbours}",
                    fix_designator = self.fix.designator,
                    fix_lat = self.fix.coordinate.y,
                    fix_lng = self.fix.coordinate.x,
                    airway_type = neighbours.airway_type,
                )?;
            }
        }

        Ok(())
    }
}

impl Display for AirwayType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            AirwayType::High => "H",
            AirwayType::Low => "L",
            AirwayType::Both => "B",
        })
    }
}

impl Display for AirwayNeighbours {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\t{}",
            self.previous
                .as_ref()
                .map_or("\t\t\t\tN".to_string(), ToString::to_string),
            self.next
                .as_ref()
                .map_or("\t\t\t\tN".to_string(), ToString::to_string)
        )
    }
}

impl Display for AirwayFix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\t{:.6}\t{:.6}\t{}\t{}",
            self.designator,
            self.coordinate.y,
            self.coordinate.x,
            self.minimum_level
                .map_or("NESTB".to_string(), |lvl| format!("{lvl:05}")),
            if self.valid_direction { "Y" } else { "N" }
        )
    }
}
