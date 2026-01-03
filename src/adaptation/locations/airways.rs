use std::{collections::HashMap, fmt::Display, hash::Hash};

use serde::Serialize;

use crate::adaptation::locations::GraphPosition;

use super::Fix;

// /// conceptionally HashMap<Fix, HashMap<Airway, AirwayNeighbours>>
// #[derive(Clone, Debug, Default, Deref, DerefMut)]
// pub struct FixAirwayMap(pub HashMap<Fix, AirwayNeighboursOfFix>);
//
// #[derive(Clone, Debug, Serialize, PartialEq)]
// pub struct AirwayNeighboursOfFix {
//     pub fix: Fix,
//     pub airway_neighbours: MultiMap<String, AirwayNeighbours>,
// }

#[derive(Copy, Clone, Debug, Serialize, PartialEq)]
pub enum AirwayType {
    High,
    Low,
    Both,
    Unknown,
}

// #[derive(Clone, Debug, Serialize, PartialEq)]
// pub struct AirwayNeighbours {
//     pub airway: String,
//     pub airway_type: AirwayType,
//     pub previous: Option<AirwayFix>,
//     pub next: Option<AirwayFix>,
// }

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct AirwayFix {
    pub fix: Fix,
    pub valid_direction: bool,
    pub minimum_level: Option<u32>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AirwayEdge {
    pub to: FixId,
    pub valid_direction: bool,
    pub minimum_level: Option<u32>,
    pub maximum_level: Option<u32>,
}

impl PartialEq for AirwayEdge {
    fn eq(&self, other: &Self) -> bool {
        self.to == other.to
    }
}

#[derive(Copy, Clone, Debug, Serialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AirwayId(pub usize);

impl Display for AirwayId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Copy, Clone, Debug, Serialize, PartialEq, Eq, Hash)]
pub struct FixId(pub usize);

impl Display for FixId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct AirwayGraph {
    fixes: Vec<GraphFix>,
    fix_id_by_name: HashMap<String, Vec<FixId>>,
    fix_name_by_id: Vec<String>,
    airway_id_by_name: HashMap<String, AirwayId>,
    airway_name_by_id: Vec<String>,
}

impl AirwayGraph {
    pub fn expand_airway_segment(
        &self,
        start: &Fix,
        end: &str,
        airway: &str,
    ) -> Option<Vec<AirwayFix>> {
        let Some(airway) = self.get_airway_id(airway) else {
            tracing::warn!("Airway |{}| not found in data", airway);
            return None;
        };

        let Some(start) = self.find_fix_id(start) else {
            tracing::warn!("Start Point {:?} not found in data", start,);
            return None;
        };

        if !self.is_fix_id_on_airway(&start, airway) {
            tracing::warn!("{} {} {}", airway, self.fix_name_by_id[start.0], end);
            tracing::warn!(
                "Start Point |{}| not on |{}|",
                self.fix_name_by_id[start.0],
                airway
            );
            return None;
        }

        let Some(end) = self.find_fix_on_airway(end, airway) else {
            tracing::warn!("{} {} {}", airway, self.fix_name_by_id[start.0], end);
            tracing::warn!("End Point |{}| not on |{}|", end, airway);
            return None;
        };

        let edges = self.fixes.get(start.0)?.edges.get(&airway)?;

        if edges.len() > 2 {
            return None;
        }

        for edge in edges {
            if let Some(expanded_fixes) = self.traverse_airway(start, edge, end, airway) {
                return Some(
                    expanded_fixes
                        .iter()
                        .map(|(fix, valid_direction, minimum_level)| AirwayFix {
                            fix: Fix {
                                designator: self.fix_name_by_id[fix.0].clone(),
                                coordinate: self.fixes[fix.0].position.0,
                            },
                            valid_direction: *valid_direction,
                            minimum_level: *minimum_level,
                        })
                        .collect(),
                );
            }
        }

        None
    }

    fn traverse_airway(
        &self,
        start_fix: FixId,
        start_edge: &AirwayEdge,
        end: FixId,
        airway: AirwayId,
    ) -> Option<Vec<(FixId, bool, Option<u32>)>> {
        let mut prev = start_fix;
        let mut expanded: Vec<(FixId, bool, Option<u32>)> = vec![];

        let mut edge = start_edge;
        let mut count = 0_usize;

        loop {
            count += 1;
            if count > self.fixes.len() {
                tracing::warn!("Too many segments in expanded route");
                return None;
            }

            let current = edge.to;
            expanded.push((edge.to, edge.valid_direction, edge.minimum_level));

            if current == end {
                return Some(expanded);
            }

            let edges = self.fixes[current.0].edges.get(&airway)?;
            edge = match edges.len() {
                2 => edges.iter().find(|e| e.to != prev)?,
                1 if edges[0].to != prev => &edges[0],
                _ => {
                    return None;
                }
            };

            prev = current;
        }
    }

    pub fn find_fix_id(&self, fix: &Fix) -> Option<FixId> {
        let pos = GraphPosition(fix.coordinate);
        self.get_fix_ids(&fix.designator)?
            .iter()
            .copied()
            .find(|&id| self.fixes[id.0].position == pos)
    }

    pub fn find_fix_on_airway(&self, designator: &str, airway: AirwayId) -> Option<FixId> {
        self.get_fix_ids(designator)?
            .iter()
            .copied()
            .find(|&id| self.fixes[id.0].edges.contains_key(&airway))
    }

    pub fn get_airway_id(&self, name: &str) -> Option<AirwayId> {
        self.airway_id_by_name.get(name).copied()
    }

    pub fn get_airway_name(&self, airway: &AirwayId) -> Option<&String> {
        self.airway_name_by_id.get(airway.0)
    }

    pub fn get_fix_by_id(&self, fix: FixId) -> Option<&GraphFix> {
        self.fixes.get(fix.0)
    }

    pub fn get_fix_ids(&self, name: &str) -> Option<&Vec<FixId>> {
        self.fix_id_by_name.get(name)
    }

    pub fn get_fix_name(&self, fix: FixId) -> Option<&String> {
        self.fix_name_by_id.get(fix.0)
    }

    pub fn is_fix_on_airway(&self, fix: &Fix, airway: &AirwayId) -> bool {
        self.find_fix_id(fix)
            .is_some_and(|id| self.fixes[id.0].edges.contains_key(airway))
    }

    pub fn is_fix_id_on_airway(&self, fix: &FixId, airway: AirwayId) -> bool {
        self.fixes
            .get(fix.0)
            .is_some_and(|fix| fix.edges.contains_key(&airway))
    }

    pub(crate) fn insert_or_update_segment(
        &mut self,
        airway_name: &str,
        from_name: &str,
        from_fix: GraphPosition,
        to_name: &str,
        to_fix: GraphPosition,
        allowed_to: bool,
        allowed_from: Option<bool>,
        minimum_level: Option<u32>,
        maximum_level: Option<u32>,
    ) {
        let self_id = self.get_or_insert_fix_id(from_fix, from_name);
        let to_id = self.get_or_insert_fix_id(to_fix, to_name);
        let awy_id = self.get_or_insert_airway_id(airway_name);

        let to_edge = AirwayEdge {
            to: to_id,
            valid_direction: allowed_to,
            minimum_level,
            maximum_level,
        };

        self.insert_or_update_edge(self_id, awy_id, to_edge);

        let from_edge = AirwayEdge {
            to: self_id,
            valid_direction: allowed_from.unwrap_or_default(),
            minimum_level,
            maximum_level,
        };

        self.insert_or_update_edge(to_id, awy_id, from_edge);
    }

    fn get_or_insert_fix_id(&mut self, fix: GraphPosition, name: &str) -> FixId {
        if let Some(ids) = self.fix_id_by_name.get(name) {
            for &id in ids {
                if self.fixes[id.0].position == fix {
                    return id;
                }
            }
        }
        self.add_fix_raw(fix, name)
    }

    fn get_or_insert_airway_id(&mut self, name: &str) -> AirwayId {
        if let Some(&id) = self.airway_id_by_name.get(name) {
            return id;
        }

        let id = AirwayId(self.airway_name_by_id.len());
        self.airway_name_by_id.push(name.to_owned());
        self.airway_id_by_name.insert(name.to_owned(), id);
        id
    }

    fn add_fix_raw(&mut self, fix: GraphPosition, name: &str) -> FixId {
        let id = FixId(self.fixes.len());
        self.fixes.push(GraphFix {
            position: fix,
            edges: HashMap::new(),
        });

        self.fix_name_by_id.push(name.to_owned());
        self.fix_id_by_name
            .entry(name.to_owned())
            .or_default()
            .push(id);

        id
    }

    fn insert_or_update_edge(&mut self, from: FixId, airway: AirwayId, to_edge: AirwayEdge) {
        let edges = self.fixes[from.0].edges.entry(airway).or_default();
        if let Some(edge) = edges.iter_mut().find(|e| *e == &to_edge) {
            edge.valid_direction |= to_edge.valid_direction;
            edge.minimum_level = match (edge.minimum_level, to_edge.minimum_level) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (x, None) | (None, x) => x,
            };
            edge.maximum_level = match (edge.maximum_level, to_edge.maximum_level) {
                (Some(a), Some(b)) => Some(a.max(b)),
                (x, None) | (None, x) => x,
            };
        } else {
            edges.push(to_edge);
        }
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct GraphFix {
    pub position: GraphPosition,
    pub edges: HashMap<AirwayId, Vec<AirwayEdge>>,
}

impl PartialEq<GraphPosition> for GraphFix {
    fn eq(&self, other: &GraphPosition) -> bool {
        &self.position == other
    }
}

// impl FixAirwayMap {
//     pub fn iter_forwards<'a>(&'a self, start: Fix, airway: &'a str) -> AirwayForwardIterator<'a> {
//         AirwayForwardIterator {
//             airways: self,
//             airway,
//             current_fix: start,
//         }
//     }
//
//     pub fn iter_backwards<'a>(&'a self, start: Fix, airway: &'a str) -> AirwayBackwardIterator<'a> {
//         AirwayBackwardIterator {
//             airways: self,
//             airway,
//             current_fix: start,
//         }
//     }
// }
//
// pub struct AirwayForwardIterator<'airways> {
//     airways: &'airways FixAirwayMap,
//     airway: &'airways str,
//     current_fix: Fix,
// }
//
// impl<'airways> Iterator for AirwayForwardIterator<'airways> {
//     type Item = &'airways AirwayFix;
//
//     fn next(&mut self) -> Option<Self::Item> {
//         let maybe_next = self
//             .airways
//             .get(&self.current_fix)
//             .and_then(|wpt_airways| wpt_airways.airway_neighbours.get_vec(self.airway))
//             .and_then(|neighbours| neighbours.iter().find_map(|n| n.next.as_ref()));
//
//         trace!(
//             "Iterating forward on {}: {} -> {}",
//             self.airway,
//             self.current_fix.designator,
//             maybe_next.map_or("None", |next| &*next.fix.designator)
//         );
//
//         if let Some(next) = maybe_next {
//             self.current_fix.clone_from(&next.fix);
//         }
//
//         maybe_next
//     }
// }
//
// pub struct AirwayBackwardIterator<'airways> {
//     airways: &'airways FixAirwayMap,
//     airway: &'airways str,
//     current_fix: Fix,
// }
//
// impl<'airways> Iterator for AirwayBackwardIterator<'airways> {
//     type Item = &'airways AirwayFix;
//
//     fn next(&mut self) -> Option<Self::Item> {
//         let maybe_previous = self
//             .airways
//             .get(&self.current_fix)
//             .and_then(|wpt_airways| wpt_airways.airway_neighbours.get_vec(self.airway))
//             .and_then(|neighbours| neighbours.iter().find_map(|n| n.previous.as_ref()));
//
//         if let Some(previous) = maybe_previous {
//             self.current_fix.clone_from(&previous.fix);
//         }
//
//         maybe_previous
//     }
// }

// impl Serialize for FixAirwayMap {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         let key = |fix: &Fix| {
//             format!(
//                 "{}:{:.6}:{:.6}",
//                 fix.designator,
//                 fix.coordinate.y(),
//                 fix.coordinate.x()
//             )
//         };
//         serializer.collect_map(self.0.iter().map(|(fix, v)| (key(fix), v)))
//     }
// }
//
// impl Display for FixAirwayMap {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         for (_, airways) in self.0.iter().sorted_by(|(fix, _), (fix2, _)| {
//             fix.designator.cmp(&fix2.designator).then(
//                 fix.coordinate
//                     .x()
//                     .partial_cmp(&fix.coordinate.x())
//                     .map_or(Ordering::Equal, Ordering::reverse),
//             )
//         }) {
//             write!(f, "{airways}")?;
//         }
//
//         Ok(())
//     }
// }
//
// impl Display for AirwayNeighboursOfFix {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         for (airway, multi_neighbours) in self
//             .airway_neighbours
//             .iter_all()
//             .sorted_by_key(|(airway, _)| *airway)
//         {
//             for neighbours in multi_neighbours {
//                 writeln!(
//                     f,
//                     "{fix_designator}\t{fix_lat:.6}\t{fix_lng:.6}\t14\t{airway}\t{airway_type}\t{neighbours}",
//                     fix_designator = self.fix.designator,
//                     fix_lat = self.fix.coordinate.y(),
//                     fix_lng = self.fix.coordinate.x(),
//                     airway_type = neighbours.airway_type,
//                 )?;
//             }
//         }
//
//         Ok(())
//     }
// }

impl Display for AirwayType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            AirwayType::High => "H",
            AirwayType::Low => "L",
            AirwayType::Both => "B",
            AirwayType::Unknown => "",
        })
    }
}

// impl Display for AirwayNeighbours {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(
//             f,
//             "{}\t{}",
//             self.previous
//                 .as_ref()
//                 .map_or("\t\t\t\tN".to_string(), ToString::to_string),
//             self.next
//                 .as_ref()
//                 .map_or("\t\t\t\tN".to_string(), ToString::to_string)
//         )
//     }
// }

impl Display for AirwayFix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\t{:.6}\t{:.6}\t{}\t{}",
            self.fix.designator,
            self.fix.coordinate.y(),
            self.fix.coordinate.x(),
            self.minimum_level
                .map_or_else(String::new, |lvl| format!("{lvl:05}")),
            if self.valid_direction { "Y" } else { "N" }
        )
    }
}
