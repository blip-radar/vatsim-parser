use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    hash::Hash,
    sync::Arc,
};

use geo::Point;
use serde::{Deserialize, Serialize};

use crate::adaptation::locations::Locations;

use super::{Fix, GraphPosition};

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum AirwayType {
    High,
    Low,
    Both,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct AirwayFix {
    pub fix: Fix,
    pub valid_direction: bool,
    pub minimum_level: Option<u32>,
}

#[derive(Copy, Clone, Debug)]
struct AirwayEdge {
    to: FixId,
    valid_direction: bool,
    minimum_level: Option<u32>,
    maximum_level: Option<u32>,
    airway_type: AirwayType,
}

impl PartialEq for AirwayEdge {
    fn eq(&self, other: &Self) -> bool {
        self.to == other.to
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct AirwayId(usize);

impl Display for AirwayId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct FixId(usize);

impl Display for FixId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub type SharedStr = Arc<str>;

/// Human/jsonnet-overridable view of an [`AirwayGraph`]: each airway is an ordered
/// chain of waypoints, keyed by airway designator.
pub type AirwayGraphView = HashMap<String, Vec<AirwayWaypointView>>;

/// One waypoint in an airway's ordered chain. `valid_direction`, `minimum_level`,
/// `maximum_level` and `airway_type` describe the segment leading INTO this waypoint
/// from the previous one in the chain; they are meaningless (default) on the first
/// waypoint of a chain, which has no incoming segment.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct AirwayWaypointView {
    pub fix: String,
    pub coordinate: Point,
    #[serde(default)]
    pub valid_direction: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum_level: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum_level: Option<u32>,
    #[serde(default)]
    pub airway_type: AirwayType,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(into = "AirwayGraphView", from = "AirwayGraphView")]
pub struct AirwayGraph {
    fixes: Vec<GraphFix>,
    fix_id_by_name: HashMap<SharedStr, Vec<FixId>>,
    fix_name_by_id: Vec<SharedStr>,
    airway_id_by_name: HashMap<SharedStr, AirwayId>,
    airway_name_by_id: Vec<SharedStr>,
}

impl AirwayGraph {
    pub fn expand_airway_segment(
        &self,
        start: &Fix,
        end: &str,
        airway: &str,
        locations: &Locations,
    ) -> Option<Vec<(AirwayFix, bool)>> {
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

        if !self.is_fix_id_on_airway(*start, airway) {
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

        let edges = self.fixes[start.0].edges.get(&airway)?;

        if edges.len() > 2 {
            tracing::debug!(
                "More than 2 edges for airway {} at fix {}",
                self.airway_name_by_id[airway.0],
                self.fix_name_by_id[start.0]
            );
            return None;
        }

        for edge in edges {
            let Some(expanded_fixes) = self.traverse_airway(*start, edge, *end, airway) else {
                continue;
            };

            return Some(
                expanded_fixes
                    .iter()
                    .map(|(fix, valid_direction, minimum_level)| {
                        let designator = self.fix_name_by_id[fix.0].clone();
                        let coordinate = self.fixes[fix.0].position;
                        let is_internal = locations.contains_nav_element(&designator, coordinate);

                        let af = AirwayFix {
                            fix: Fix {
                                designator: designator.to_string(),
                                coordinate: coordinate.0,
                            },
                            valid_direction: *valid_direction,
                            minimum_level: *minimum_level,
                        };
                        (af, is_internal)
                    })
                    .collect(),
            );
        }
        None
    }

    pub(crate) fn insert_or_update_segment(
        &mut self,
        airway_name: &str,
        from_name: &str,
        from_fix: GraphPosition,
        segment: &AirwayFix,
        airway_type: AirwayType,
    ) {
        let self_id = self.get_or_insert_fix_id(from_fix, from_name);
        let to_id = self.get_or_insert_fix_id(
            GraphPosition(segment.fix.coordinate),
            &segment.fix.designator,
        );
        let awy_id = self.get_or_insert_airway_id(airway_name);

        let to_edge = AirwayEdge {
            to: to_id,
            valid_direction: segment.valid_direction,
            minimum_level: segment.minimum_level,
            maximum_level: None,
            airway_type,
        };

        self.insert_or_update_edge(self_id, awy_id, to_edge);

        let from_edge = AirwayEdge {
            to: self_id,
            valid_direction: false,
            minimum_level: segment.minimum_level,
            maximum_level: None,
            airway_type,
        };

        self.insert_or_update_edge(to_id, awy_id, from_edge);
    }

    fn add_fix_raw(&mut self, fix: GraphPosition, name: &str) -> FixId {
        let id = FixId(self.fixes.len());
        self.fixes.push(GraphFix {
            position: fix,
            edges: HashMap::new(),
        });

        let name_arc = SharedStr::from(name);

        self.fix_name_by_id.push(name_arc.clone());
        self.fix_id_by_name.entry(name_arc).or_default().push(id);

        id
    }

    fn find_fix_id(&self, fix: &Fix) -> Option<&FixId> {
        let pos = GraphPosition(fix.coordinate);
        self.get_fix_ids(&fix.designator)?
            .iter()
            .find(|&id| self.fixes[id.0].position == pos)
    }

    fn find_fix_on_airway(&self, designator: &str, airway: AirwayId) -> Option<&FixId> {
        self.get_fix_ids(designator)?
            .iter()
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

        let name_arc = SharedStr::from(name);

        let id = AirwayId(self.airway_name_by_id.len());
        self.airway_name_by_id.push(name_arc.clone());
        self.airway_id_by_name.insert(name_arc, id);
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

    /// Walk every fix participating in `airway` into a single ordered chain,
    /// starting from a chain endpoint (a fix with only one edge on this airway) if
    /// one exists, otherwise from an arbitrary fix (closed loop).
    fn chain_fix_ids(&self, airway: AirwayId) -> Vec<FixId> {
        let participants: Vec<FixId> = self
            .fixes
            .iter()
            .enumerate()
            .filter(|(_, fix)| fix.edges.contains_key(&airway))
            .map(|(idx, _)| FixId(idx))
            .collect();

        let Some(&start) = participants
            .iter()
            .find(|&&id| {
                self.fixes[id.0]
                    .edges
                    .get(&airway)
                    .is_some_and(|edges| edges.len() == 1)
            })
            .or_else(|| participants.first())
        else {
            return participants;
        };

        std::iter::successors(Some((start, start)), |&(prev, current)| {
            let edges = self.fixes[current.0].edges.get(&airway)?;
            let next = match edges.len() {
                2 => edges.iter().find(|e| e.to != prev)?,
                1 if edges[0].to != prev => &edges[0],
                _ => return None,
            };
            (next.to != start).then_some((current, next.to))
        })
        .take(self.fixes.len() + 1)
        .map(|(_, current)| current)
        .collect()
    }
}

impl From<AirwayGraph> for AirwayGraphView {
    fn from(graph: AirwayGraph) -> Self {
        graph
            .airway_name_by_id
            .iter()
            .enumerate()
            .map(|(idx, name)| {
                let airway = AirwayId(idx);
                let chain = graph.chain_fix_ids(airway);

                let waypoints = chain
                    .iter()
                    .enumerate()
                    .map(|(pos, &fix_id)| {
                        let fix = &graph.fixes[fix_id.0];
                        let incoming_edge = pos.checked_sub(1).and_then(|prev_pos| {
                            let prev_id = chain[prev_pos];
                            graph.fixes[prev_id.0]
                                .edges
                                .get(&airway)?
                                .iter()
                                .find(|e| e.to == fix_id)
                        });

                        AirwayWaypointView {
                            fix: graph.fix_name_by_id[fix_id.0].to_string(),
                            coordinate: fix.position.0,
                            valid_direction: incoming_edge.is_none_or(|e| e.valid_direction),
                            minimum_level: incoming_edge.and_then(|e| e.minimum_level),
                            maximum_level: incoming_edge.and_then(|e| e.maximum_level),
                            airway_type: incoming_edge
                                .map_or(AirwayType::Unknown, |e| e.airway_type),
                        }
                    })
                    .collect();

                (name.to_string(), waypoints)
            })
            .collect()
    }
}

impl From<AirwayGraphView> for AirwayGraph {
    fn from(def: AirwayGraphView) -> Self {
        def.into_iter()
            .fold(Self::default(), |mut graph, (airway_name, waypoints)| {
                let awy_id = graph.get_or_insert_airway_id(&airway_name);

                waypoints.windows(2).for_each(|pair| {
                    let [from, to] = pair else { unreachable!() };
                    let from_id =
                        graph.get_or_insert_fix_id(GraphPosition(from.coordinate), &from.fix);
                    let to_id = graph.get_or_insert_fix_id(GraphPosition(to.coordinate), &to.fix);

                    graph.insert_or_update_edge(
                        from_id,
                        awy_id,
                        AirwayEdge {
                            to: to_id,
                            valid_direction: to.valid_direction,
                            minimum_level: to.minimum_level,
                            maximum_level: to.maximum_level,
                            airway_type: to.airway_type,
                        },
                    );
                    graph.insert_or_update_edge(
                        to_id,
                        awy_id,
                        AirwayEdge {
                            to: from_id,
                            valid_direction: false,
                            minimum_level: to.minimum_level,
                            maximum_level: to.maximum_level,
                            airway_type: to.airway_type,
                        },
                    );
                });

                graph
            })
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

#[derive(Clone, Debug, PartialEq)]
struct GraphFix {
    position: GraphPosition,
    edges: HashMap<AirwayId, Vec<AirwayEdge>>,
}

impl PartialEq<GraphPosition> for GraphFix {
    fn eq(&self, other: &GraphPosition) -> bool {
        &self.position == other
    }
}

#[cfg(test)]
mod tests {
    use geo::point;

    use super::*;
    use crate::adaptation::locations::Locations;

    fn fix(designator: &str, lng: f64, lat: f64) -> AirwayFix {
        AirwayFix {
            fix: Fix {
                designator: designator.to_string(),
                coordinate: point! { x: lng, y: lat },
            },
            valid_direction: true,
            minimum_level: None,
        }
    }

    fn build_graph() -> AirwayGraph {
        let mut graph = AirwayGraph::default();

        // linear airway UL997: FIXA - FIXB - FIXC
        graph.insert_or_update_segment(
            "UL997",
            "FIXA",
            GraphPosition(point! { x: 1.0, y: 1.0 }),
            &AirwayFix {
                minimum_level: Some(5000),
                ..fix("FIXB", 2.0, 2.0)
            },
            AirwayType::High,
        );
        graph.insert_or_update_segment(
            "UL997",
            "FIXB",
            GraphPosition(point! { x: 2.0, y: 2.0 }),
            &fix("FIXC", 3.0, 3.0),
            AirwayType::High,
        );

        // separate airway L601 with a single segment
        graph.insert_or_update_segment(
            "L601",
            "FIXD",
            GraphPosition(point! { x: 4.0, y: 4.0 }),
            &fix("FIXE", 5.0, 5.0),
            AirwayType::Low,
        );

        graph
    }

    #[test]
    fn serializes_keyed_by_airway_name() {
        let graph = build_graph();
        let value = serde_json::to_value(&graph).unwrap();
        let obj = value.as_object().unwrap();

        assert!(obj.contains_key("UL997"));
        assert!(obj.contains_key("L601"));

        let ul997 = obj["UL997"].as_array().unwrap();
        let names: Vec<&str> = ul997.iter().map(|w| w["fix"].as_str().unwrap()).collect();
        assert_eq!(names, ["FIXA", "FIXB", "FIXC"]);
    }

    #[test]
    fn round_trips_through_json() {
        let graph = build_graph();
        let locations = Locations::default();

        let before = graph
            .expand_airway_segment(&fix("FIXA", 1.0, 1.0).fix, "FIXC", "UL997", &locations)
            .unwrap();

        let json = serde_json::to_string(&graph).unwrap();
        let round_tripped: AirwayGraph = serde_json::from_str(&json).unwrap();

        let after = round_tripped
            .expand_airway_segment(&fix("FIXA", 1.0, 1.0).fix, "FIXC", "UL997", &locations)
            .unwrap();

        assert_eq!(before, after);
    }
}
