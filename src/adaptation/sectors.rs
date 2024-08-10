use std::collections::{HashMap, HashSet};

use geo::{Coord, Line, Polygon};
use multimap::MultiMap;
use serde::Serialize;
use tracing::warn;

use crate::{
    ese::{self, Ese},
    TwoKeyMultiMap,
};

use super::maps::active::RunwayIdentifier;

#[derive(Clone, Debug, Serialize)]
pub struct Volume {
    pub id: String,
    pub lower_level: u32,
    pub upper_level: u32,
    pub lateral_border: Polygon,
}

#[derive(Clone, Debug, Serialize)]
pub struct Sector {
    pub id: String,
    pub position_priority: Vec<String>,
    pub runway_filter: Vec<Vec<RunwayIdentifier>>,
    pub volumes: Vec<String>,
}

// FIXME better error handling and reporting
fn polygon_from_ese(sector: &ese::Sector) -> Option<Polygon> {
    let lines: Vec<_> = sector
        .border
        .iter()
        .flat_map(|lines| {
            lines.points.lines().map(|line| {
                // i64 to be able to use as key below
                Line::<i64>::new(
                    (
                        (line.start.x * 1_000_000.0) as i64,
                        (line.start.y * 1_000_000.0) as i64,
                    ),
                    (
                        (line.end.x * 1_000_000.0) as i64,
                        (line.end.y * 1_000_000.0) as i64,
                    ),
                )
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

    let polygon = if let Some(start_point) = points.iter().next() {
        let mut stack = vec![*start_point];
        let mut polygon = vec![];
        let mut current = *start_point;

        while !stack.is_empty() {
            if let Some(neighbors) = adj_list.get_mut(&current) {
                if neighbors.is_empty() {
                    polygon.push(current);
                    current = stack.pop().unwrap();
                } else {
                    stack.push(current);
                    let next = neighbors.pop().unwrap();
                    if let Some(rev_neighbors) = adj_list.get_mut(&next) {
                        if let Some(pos) = rev_neighbors.iter().position(|x| *x == current) {
                            rev_neighbors.swap_remove(pos);
                        }
                    }
                    current = next;
                }
            }
        }

        polygon
    } else {
        return None;
    };

    if polygon.len() == lines.len() + 1 {
        Some(Polygon::new(
            polygon
                .iter()
                .map(|c| Coord::from((c.x as f64, c.y as f64)) / 1_000_000.0)
                .collect(),
            vec![],
        ))
    } else {
        None
    }
}

impl Sector {
    pub fn from_ese(ese: &Ese) -> (HashMap<String, Volume>, HashMap<String, Sector>) {
        let (by_priorities_filters, volumes) = ese.sectors.iter().fold(
            (TwoKeyMultiMap(MultiMap::new()), HashMap::new()),
            |(mut sectors, mut volumes), (id, sector)| {
                if let Some(polygon) = polygon_from_ese(sector) {
                    sectors.0.insert(
                        (sector.owner_priority.clone(), sector.runway_filter.clone()),
                        id.clone(),
                    );

                    volumes.insert(
                        id.clone(),
                        Volume {
                            id: id.clone(),
                            lower_level: sector.bottom,
                            upper_level: sector.top,
                            lateral_border: polygon,
                        },
                    );
                } else {
                    warn!("Could not compute valid polygon for {id}");
                }
                (sectors, volumes)
            },
        );
        let sectors = by_priorities_filters.0.into_iter().fold(
            HashMap::new(),
            |mut acc, ((position_priority, runway_filter), volumes)| {
                acc.insert(
                    volumes[0].clone(),
                    Sector {
                        id: volumes[0].clone(),
                        position_priority,
                        runway_filter: vec![runway_filter],
                        volumes,
                    },
                );
                acc
            },
        );
        (volumes, sectors)
    }
}
