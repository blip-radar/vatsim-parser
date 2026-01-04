use std::{collections::HashMap, fmt::Display, hash::Hash};

use serde::Serialize;

use crate::adaptation::locations::GraphPosition;

use super::Fix;

#[derive(Copy, Clone, Debug, Serialize, PartialEq)]
pub enum AirwayType {
    High,
    Low,
    Both,
    Unknown,
}

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
            tracing::debug!("Airway {} not found in AirwayGraph", airway);
            return None;
        };

        let Some(start) = self.find_fix_id(start).or_else(|| {
            tracing::debug!("Alternative fix lookup for {}", &start.designator);
            self.find_fix_on_airway(&start.designator, airway)
        }) else {
            tracing::debug!("Start Point {:?} not found in AirwayGraph", start);
            return None;
        };

        if !self.is_fix_id_on_airway(start, airway) {
            tracing::debug!(
                "Start Point {} not on airway {}",
                self.fix_name_by_id[start.0],
                self.airway_name_by_id[airway.0],
            );
            return None;
        }

        let Some(end) = self.find_fix_on_airway(end, airway) else {
            tracing::debug!(
                "End Point {} not on {}",
                end,
                self.airway_name_by_id[airway.0]
            );
            return None;
        };

        let edges = self.fixes.get(start.0)?.edges.get(&airway)?;

        if edges.len() > 2 {
            tracing::debug!(
                "More than 2 edges for airway {} at fix {}",
                self.airway_name_by_id[airway.0],
                self.fix_name_by_id[start.0]
            );
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

    fn find_fix_id(&self, fix: &Fix) -> Option<FixId> {
        let pos = GraphPosition(fix.coordinate);
        self.get_fix_ids(&fix.designator)?
            .iter()
            .copied()
            .find(|&id| self.fixes[id.0].position == pos)
    }

    fn find_fix_on_airway(&self, designator: &str, airway: AirwayId) -> Option<FixId> {
        self.get_fix_ids(designator)?
            .iter()
            .copied()
            .find(|&id| self.fixes[id.0].edges.contains_key(&airway))
    }

    fn get_airway_id(&self, name: &str) -> Option<AirwayId> {
        self.airway_id_by_name.get(name).copied()
    }

    fn get_fix_ids(&self, name: &str) -> Option<&Vec<FixId>> {
        self.fix_id_by_name.get(name)
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

    fn is_fix_id_on_airway(&self, fix: FixId, airway: AirwayId) -> bool {
        self.fixes[fix.0].edges.contains_key(&airway)
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
