use std::collections::HashSet;

use geo::{Coord, HasDimensions, Line, LineString, MultiLineString};
use itertools::Itertools as _;
use multimap::MultiMap;
use serde::Serialize;
use tracing::warn;

use crate::sct::{Airway, Label, Region, Sct};

use super::{
    colours::{Colour, Colours},
    locations::Locations,
    settings::Settings,
};

#[derive(Clone, Debug, Serialize)]
pub struct ColouredLines {
    pub colour: Colour,
    pub lines: MultiLineString,
}

fn line_hash(line: Line) -> String {
    format!(
        "x{:.6}y{:.6}x{:.6}y{:.6}",
        line.start.x, line.start.y, line.end.x, line.end.y
    )
}
fn coord_hash(c: Coord) -> String {
    format!("x{:.6}y{:.6}", c.x, c.y)
}

impl ColouredLines {
    fn from_location_coloured_lines(
        line_group: &crate::sct::ColouredLines,
        locations: &Locations,
        colours: &Colours,
        settings: &Settings,
        name: &str,
        typ: &str,
    ) -> Option<Self> {
        let lines: MultiLineString = line_group
            .lines
            .iter()
            .filter_map(|loc_line| {
                let line: LineString = loc_line
                    .points
                    .iter()
                    .filter_map(|loc| {
                        let point = locations.convert_location(loc);
                        if point.is_none() {
                            warn!("Could not convert {:?} in .sct {typ} {name}", loc);
                        }
                        point
                    })
                    .dedup()
                    .collect();

                (!line.is_empty()).then_some(line)
            })
            .collect();

        (!lines.is_empty()).then_some(ColouredLines {
            colour: line_group
                .colour_name
                .as_ref()
                .and_then(|colour_name| colours.get(colour_name, settings))
                .unwrap_or(colours.map.sid),
            lines,
        })
    }
}

fn build_linestring(
    point: Coord,
    visited: HashSet<String>,
    adjacency: &MultiMap<String, Line>,
) -> (LineString, std::collections::HashSet<String>) {
    adjacency
        .get_vec(&coord_hash(point))
        .and_then(|connected_lines| {
            connected_lines
                .iter()
                .find(|line| !visited.contains(&line_hash(**line)))
        })
        .map(|next_line| {
            let next_point = if next_line.start == point {
                next_line.end
            } else {
                next_line.start
            };
            let mut tmp_visited = visited.clone();
            tmp_visited.insert(line_hash(*next_line));
            let (mut path, new_visited) = build_linestring(next_point, tmp_visited, adjacency);
            path.0.insert(0, point);
            (path, new_visited)
        })
        .unwrap_or((LineString::new(vec![point]), visited))
}

fn airway_to_multi_line_string(
    airways: &[Airway],
    locations: &Locations,
    name: &str,
    typ: &str,
) -> MultiLineString {
    let lines = airways
        .iter()
        .filter_map(|airway| {
            let line = locations
                .convert_location(&airway.start)
                .zip(locations.convert_location(&airway.start))
                .map(|(start, end)| Line::new(start, end));
            if line.is_none() {
                warn!("Could not convert {airway:?} in .sct {typ} {name}");
            }
            line
        })
        .collect::<Vec<_>>();

    let adjacency = lines
        .iter()
        .flat_map(|line| {
            vec![
                (coord_hash(line.start), *line),
                (coord_hash(line.end), *line),
            ]
        })
        .collect();

    lines
        .iter()
        .fold(
            (
                MultiLineString::new(vec![]),
                std::collections::HashSet::new(),
            ),
            |(mut acc, mut visited), line| {
                if visited.contains(&line_hash(*line)) {
                    (acc, visited)
                } else {
                    visited.insert(line_hash(*line));
                    let (linestring, new_visited) =
                        build_linestring(line.start, visited, &adjacency);
                    acc.0.push(linestring);

                    (acc, new_visited)
                }
            },
        )
        .0
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct SctItems {
    pub sids: MultiMap<String, ColouredLines>,
    pub stars: MultiMap<String, ColouredLines>,
    pub high_airways: MultiMap<String, MultiLineString>,
    pub low_airways: MultiMap<String, MultiLineString>,
    pub artccs_high: MultiMap<String, ColouredLines>,
    pub artccs: MultiMap<String, ColouredLines>,
    pub artccs_low: MultiMap<String, ColouredLines>,
    pub geo: MultiMap<String, ColouredLines>,
    //polygons
    pub regions: MultiMap<String, Region>,
    //text
    pub labels: MultiMap<String, Label>,
}

impl SctItems {
    pub fn from_sct(
        sct: Sct,
        locations: &Locations,
        colours: &Colours,
        settings: &Settings,
    ) -> Self {
        Self {
            sids: sct
                .sids
                .into_iter()
                .flat_map(|sid| {
                    sid.line_groups
                        .iter()
                        .filter_map(|loc_line_group| {
                            ColouredLines::from_location_coloured_lines(
                                loc_line_group,
                                locations,
                                colours,
                                settings,
                                &sid.name,
                                "SID",
                            )
                            .map(|line_group| (sid.name.clone(), line_group))
                        })
                        .collect::<Vec<_>>()
                })
                .collect(),
            stars: sct
                .stars
                .into_iter()
                .flat_map(|star| {
                    star.line_groups
                        .iter()
                        .filter_map(|loc_line_group| {
                            ColouredLines::from_location_coloured_lines(
                                loc_line_group,
                                locations,
                                colours,
                                settings,
                                &star.name,
                                "STAR",
                            )
                            .map(|line_group| (star.name.clone(), line_group))
                        })
                        .collect::<Vec<_>>()
                })
                .collect(),
            high_airways: sct
                .high_airways
                .into_iter()
                .map(|awy| (awy.designator.clone(), awy))
                .collect::<MultiMap<_, _>>()
                .into_iter()
                .map(|(name, awy)| {
                    let lines = airway_to_multi_line_string(&awy, locations, &name, "High Airways");
                    (name, lines)
                })
                .collect(),

            low_airways: sct
                .low_airways
                .into_iter()
                .map(|awy| (awy.designator.clone(), awy))
                .collect::<MultiMap<_, _>>()
                .into_iter()
                .map(|(name, awy)| {
                    let lines = airway_to_multi_line_string(&awy, locations, &name, "Low Airways");
                    (name, lines)
                })
                .collect(),
            artccs_high: sct
                .artccs_high
                .into_iter()
                .flat_map(|artcc| {
                    artcc
                        .line_groups
                        .iter()
                        .filter_map(|loc_line_group| {
                            ColouredLines::from_location_coloured_lines(
                                loc_line_group,
                                locations,
                                colours,
                                settings,
                                &artcc.name,
                                "ARTCC High",
                            )
                            .map(|line_group| (artcc.name.clone(), line_group))
                        })
                        .collect::<Vec<_>>()
                })
                .collect(),
            artccs: sct
                .artccs
                .into_iter()
                .flat_map(|artcc| {
                    artcc
                        .line_groups
                        .iter()
                        .filter_map(|loc_line_group| {
                            ColouredLines::from_location_coloured_lines(
                                loc_line_group,
                                locations,
                                colours,
                                settings,
                                &artcc.name,
                                "ARTCC",
                            )
                            .map(|line_group| (artcc.name.clone(), line_group))
                        })
                        .collect::<Vec<_>>()
                })
                .collect(),
            artccs_low: sct
                .artccs_low
                .into_iter()
                .flat_map(|artcc| {
                    artcc
                        .line_groups
                        .iter()
                        .filter_map(|loc_line_group| {
                            ColouredLines::from_location_coloured_lines(
                                loc_line_group,
                                locations,
                                colours,
                                settings,
                                &artcc.name,
                                "ARTCC Low",
                            )
                            .map(|line_group| (artcc.name.clone(), line_group))
                        })
                        .collect::<Vec<_>>()
                })
                .collect(),
            geo: sct
                .geo
                .into_iter()
                .flat_map(|geo| {
                    geo.line_groups
                        .iter()
                        .filter_map(|loc_line_group| {
                            ColouredLines::from_location_coloured_lines(
                                loc_line_group,
                                locations,
                                colours,
                                settings,
                                &geo.name,
                                "Geo",
                            )
                            .map(|line_group| (geo.name.clone(), line_group))
                        })
                        .collect::<Vec<_>>()
                })
                .collect(),
            regions: sct
                .regions
                .into_iter()
                .map(|region| (region.name.clone(), region))
                .collect(),
            labels: sct
                .labels
                .into_iter()
                .map(|label| (label.name.clone(), label))
                .collect(),
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_airway_to_multi_line_string() {}
}
