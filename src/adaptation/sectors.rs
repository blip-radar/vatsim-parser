use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
};

use bevy_derive::Deref;
use geo::{Coord, Line, LineString, Polygon, Winding};
use multimap::MultiMap;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{
    adaptation::Quantize as _,
    ese::{self, Ese},
    TwoKeyMultiMap,
};

use super::maps::active::RunwayIdentifier;

#[derive(Clone, Debug, Default, Serialize, Deserialize, Deref)]
pub struct Sectors(pub HashMap<String, Sector>);

impl<'a> IntoIterator for &'a Sectors {
    type Item = (&'a String, &'a Sector);
    type IntoIter = std::collections::hash_map::Iter<'a, String, Sector>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Volume {
    pub id: String,
    pub lower_level: u32,
    pub upper_level: u32,
    pub lateral_border: Polygon,
    private: PhantomData<()>,
}
impl Volume {
    pub fn new(
        id: String,
        lower_level: u32,
        upper_level: u32,
        mut lateral_border: LineString,
    ) -> Self {
        lateral_border.make_ccw_winding();
        Self {
            id,
            lower_level,
            upper_level,
            lateral_border: Polygon::new(lateral_border, vec![]),
            private: PhantomData,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sector {
    pub id: String,
    pub position_priority: Vec<String>,
    pub runway_filter: Vec<Vec<RunwayIdentifier>>,
    pub volumes: HashSet<String>,
    pub departure_aerodromes: HashSet<String>,
    pub arrival_aerodromes: HashSet<String>,
}

// FIXME better error handling and reporting
fn polygon_from_ese(sector: &ese::Sector) -> Option<LineString> {
    let lines: Vec<_> = sector
        .border
        .iter()
        .flat_map(|lines| {
            lines.points.lines().map(|line| {
                // i64 to be able to use as key below
                Line::<i64>::new(line.start.quantize(), line.end.quantize())
            })
        })
        .collect();
    let (mut adj_list, edge_count, points) = lines.iter().fold(
        (
            HashMap::<Coord<i64>, Vec<Coord<i64>>>::new(),
            HashMap::<Coord<i64>, usize>::new(),
            HashSet::<Coord<i64>>::new(),
        ),
        |(mut adj_list, mut edge_count, mut points), line| {
            adj_list.entry(line.start).or_default().push(line.end);
            adj_list.entry(line.end).or_default().push(line.start);
            *edge_count.entry(line.start).or_insert(0) += 1;
            *edge_count.entry(line.end).or_insert(0) += 1;
            points.insert(line.start);
            points.insert(line.end);
            (adj_list, edge_count, points)
        },
    );

    // Check if all vertices have even degree
    if !edge_count.values().all(|&count| count % 2 == 0) {
        return None;
    }

    let line_ring = if let Some(start_point) = points.iter().next() {
        let mut stack = vec![*start_point];
        let mut line_ring = vec![];
        let mut current = *start_point;

        while !stack.is_empty() {
            if let Some(neighbours) = adj_list.get_mut(&current) {
                if neighbours.is_empty() {
                    line_ring.push(current);
                    current = stack.pop().unwrap();
                } else {
                    stack.push(current);
                    let next = neighbours.pop().unwrap();
                    if let Some(rev_neighbours) = adj_list.get_mut(&next) {
                        if let Some(pos) = rev_neighbours.iter().position(|x| *x == current) {
                            rev_neighbours.swap_remove(pos);
                        }
                    }
                    current = next;
                }
            }
        }

        line_ring
    } else {
        return None;
    }
    .iter()
    .map(|c| Coord::from((c.x as f64, c.y as f64)) / 1_000_000.0)
    .collect::<LineString>();

    if line_ring.0.len() == lines.len() + 1 {
        Some(line_ring)
    } else {
        None
    }
}

impl Sectors {
    pub fn from_ese(ese: &Ese) -> (HashMap<String, Volume>, Sectors) {
        let (by_priorities_filters, volumes) = ese.sectors.iter().fold(
            (TwoKeyMultiMap(MultiMap::new()), HashMap::new()),
            |(mut sectors, mut volumes), (id, sector)| {
                if let Some(polygon) = polygon_from_ese(sector) {
                    sectors.0.insert(
                        (sector.owner_priority.clone(), sector.runway_filter.clone()),
                        (id.clone(), sector.clone()),
                    );

                    volumes.insert(
                        id.clone(),
                        Volume::new(id.clone(), sector.bottom, sector.top, polygon),
                    );
                } else {
                    warn!("Could not compute valid polygon for {id}");
                }
                (sectors, volumes)
            },
        );
        let sectors = by_priorities_filters.0.into_iter().fold(
            HashMap::new(),
            |mut acc, ((position_priority, runway_filter), volumes_and_sector)| {
                let (volumes, sectors): (Vec<String>, Vec<ese::Sector>) =
                    volumes_and_sector.into_iter().unzip();
                acc.insert(
                    // FIXME better sector name than that of the first volume
                    volumes[0].clone(),
                    Sector {
                        id: volumes[0].clone(),
                        position_priority,
                        runway_filter: vec![runway_filter],
                        volumes: volumes.into_iter().collect(),
                        departure_aerodromes: sectors
                            .iter()
                            .flat_map(|s| &s.departure_airports)
                            .cloned()
                            .collect(),
                        arrival_aerodromes: sectors
                            .iter()
                            .flat_map(|s| &s.arrival_airports)
                            .cloned()
                            .collect(),
                    },
                );
                acc
            },
        );
        (volumes, Sectors(sectors))
    }

    pub fn find_id_by_volume(&self, vol: &str) -> Option<&String> {
        self.iter()
            .find_map(|(id, sector)| sector.volumes.contains(vol).then_some(id))
    }
}
