pub mod active;

use std::collections::HashMap;

use bevy_reflect::Reflect;
use geo_types::{Coord, MultiLineString};
use serde::Serialize;

use crate::topsky::{
    map::{FontSize, MapLine, MapRule, MapSymbol, OverrideSct, Text},
    Topsky,
};

use self::active::Active;

use super::{
    colours::{Colour, Colours},
    settings::Settings,
    Alignment, Locations,
};

pub type MapFolders = HashMap<String, Folder>;

#[derive(Clone, Debug, Serialize)]
pub struct Folder {
    pub name: String,
    pub hidden: bool,
    pub maps: HashMap<String, Map>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Map {
    pub name: String,
    pub folder: String,
    pub map_groups: Vec<MapGroup>,
    pub hidden: bool,
    pub active: Vec<Vec<Active>>,
}

impl Map {
    fn config_change<F>(&mut self, settings: &Settings, colour: Colour, mut change_fn: F)
    where
        F: FnMut(&mut MapGroup),
    {
        if let Some(last) = self.map_groups.last() {
            if last.lines.0.is_empty() && last.symbols.is_empty() && last.labels.is_empty() {
                change_fn(self.map_groups.last_mut().unwrap());
            } else {
                let mut map_group = MapGroup {
                    colour: last.colour,
                    font_size: last.font_size,
                    layer: last.layer,
                    asr_data: last.asr_data.clone(),
                    zoom: last.zoom,
                    line_style: last.line_style.clone(),
                    lines: MultiLineString::new(vec![]),
                    labels: vec![],
                    symbols: vec![],
                };
                change_fn(&mut map_group);
                self.map_groups.push(map_group)
            }
        } else {
            let mut default_map_group = MapGroup::default_from_settings(settings, colour);
            change_fn(&mut default_map_group);
            self.map_groups.push(default_map_group)
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct MapGroup {
    pub colour: Colour,
    pub font_size: f32,
    pub layer: f32,
    pub asr_data: Option<Vec<String>>,
    pub zoom: Option<f32>,
    pub lines: MultiLineString,
    pub labels: Vec<Label>,
    pub symbols: Vec<Symbol>,
    pub line_style: LineStyle,
}

impl MapGroup {
    fn default_from_settings(settings: &Settings, colour: Colour) -> Self {
        Self {
            colour,
            font_size: settings.maps.font_size,
            layer: settings.maps.layer,
            asr_data: Default::default(),
            zoom: Default::default(),
            lines: MultiLineString::new(vec![]),
            labels: Default::default(),
            symbols: Default::default(),
            line_style: Default::default(),
        }
    }

    fn add_topsky_symbol(
        &mut self,
        symbol: &MapSymbol,
        settings: &Settings,
        locations: &Locations,
    ) {
        if let Some(coordinate) = locations.convert_location(&symbol.location) {
            self.symbols.push(Symbol {
                name: symbol.name.clone(),
                coordinate,
                label: symbol.label.as_ref().map(|l| l.text.clone()),
                // TODO global/map-level alignment
                label_alignment: symbol.label_alignment.clone().unwrap_or_default(),
                label_offset: symbol
                    .label
                    .as_ref()
                    .map(|l| l.pos)
                    .unwrap_or(settings.maps.label_offset),
            })
        } else {
            eprintln!("Could not convert {:?}", symbol.location);
        }
    }
    fn add_topsky_lines(&mut self, lines: &[MapLine], locations: &Locations) {
        self.lines.0.extend(lines.iter().map(|line| {
            line.points
                .iter()
                .filter_map(|loc| {
                    let coord = locations.convert_location(loc);
                    if coord.is_none() {
                        eprintln!("Could not convert {:?}", loc);
                    }
                    coord
                })
                .collect()
        }))
    }
    fn add_topsky_text(&mut self, text: &Text, locations: &Locations) {
        if let Some(coordinate) = locations.convert_location(&text.location) {
            self.labels.push(Label {
                coordinate,
                text: text.content.clone(),
                // TODO global/map-level alignment
                alignment: text.alignment.clone().unwrap_or_default(),
            })
        } else {
            eprintln!("Could not convert {:?}", text.location);
        }
    }
}

#[derive(Clone, Debug, Reflect, Serialize, PartialEq, Eq)]
pub enum LineStyleType {
    Solid,
    Alternate,
    Dot,
    Dash,
    DashDot,
    DashDotDot,
    Custom(String),
}

#[derive(Clone, Debug, Reflect, Serialize, PartialEq, Eq)]
pub struct LineStyle {
    pub width: i32,
    pub style: LineStyleType,
}
impl Default for LineStyle {
    fn default() -> Self {
        Self {
            width: 1,
            style: LineStyleType::Solid,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Label {
    pub coordinate: Coord,
    pub alignment: Alignment,
    pub text: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Symbol {
    pub name: String,
    pub coordinate: Coord,
    pub label: Option<String>,
    pub label_alignment: Alignment,
    pub label_offset: (f64, f64),
}

pub fn from_topsky(
    topsky: &Topsky,
    settings: &Settings,
    colours: &Colours,
    locations: &Locations,
) -> MapFolders {
    topsky
        .maps
        .iter()
        .fold(HashMap::new(), |mut folders, topsky_map| {
            if let Some(colour) = topsky_map.rules.iter().find_map(|rule| match rule {
                MapRule::Colour(c) => colours.get(c, settings),
                _ => None,
            }) {
                let mut map = topsky_map.rules.iter().fold(
                    Map {
                        name: topsky_map.name.clone(),
                        folder: settings.maps.auto_folder.clone(),
                        active: vec![],
                        map_groups: vec![MapGroup::default_from_settings(settings, colour)],
                        hidden: false,
                    },
                    |mut map, rule| {
                        match rule {
                            MapRule::Folder(folder) => map.folder = folder.clone(),
                            MapRule::Hidden => map.hidden = true,
                            MapRule::Colour(new_colour_name) => {
                                if let Some(new_colour) = colours.get(new_colour_name, settings) {
                                    map.config_change(settings, colour, |map_group| {
                                        map_group.colour = new_colour
                                    })
                                }
                            }
                            MapRule::AsrData(asr_data) => {
                                map.config_change(settings, colour, |map_group| {
                                    map_group.asr_data = asr_data.clone()
                                })
                            }
                            MapRule::Active(active) => map.active.push(vec![active.clone()]),
                            MapRule::AndActive(active) => {
                                if let Some(actives) = map.active.last_mut() {
                                    actives.push(active.clone())
                                } else {
                                    eprintln!("AndActive unreachable?!")
                                }
                            }
                            MapRule::Layer(layer) => {
                                map.config_change(settings, colour, |map_group| {
                                    map_group.layer = *layer as f32
                                })
                            }
                            MapRule::Zoom(zoom) => {
                                map.config_change(settings, colour, |map_group| {
                                    map_group.zoom = Some(*zoom)
                                })
                            }
                            MapRule::FontSize(font_size_mod) => {
                                map.config_change(settings, colour, |map_group| {
                                    map_group.font_size = match font_size_mod {
                                        FontSize::Exact(fs) => *fs,
                                        FontSize::Add(fs) => map_group.font_size + fs,
                                        FontSize::Subtract(fs) => map_group.font_size - fs,
                                        FontSize::Multiply(fs) => map_group.font_size * fs,
                                        FontSize::Default => settings.maps.font_size,
                                    }
                                })
                            }
                            MapRule::LineStyle(ls) => {
                                map.config_change(settings, colour, |map_group| {
                                    map_group.line_style = ls.clone()
                                })
                            }
                            // safe unwrap due to initial element above
                            MapRule::Symbol(s) => map
                                .map_groups
                                .last_mut()
                                .unwrap()
                                .add_topsky_symbol(s, settings, locations),
                            // safe unwrap due to initial element above
                            MapRule::Line(l) => map
                                .map_groups
                                .last_mut()
                                .unwrap()
                                .add_topsky_lines(l, locations),
                            // safe unwrap due to initial element above
                            MapRule::Text(t) => map
                                .map_groups
                                .last_mut()
                                .unwrap()
                                .add_topsky_text(t, locations),
                            // intentionally ignored
                            MapRule::Global | MapRule::ScreenSpecific => (),
                        };

                        map
                    },
                );
                folders
                    .entry(map.folder.clone())
                    .and_modify(|folder| {
                        if folder.maps.contains_key(&map.name) {
                            let mut i = 2;
                            while folder.maps.contains_key(&format!("{}_{i}", map.name)) {
                                i += 1;
                            }
                            map.name = format!("{}_{i}", map.name);
                        }

                        folder.maps.insert(map.name.clone(), map.clone());
                    })
                    .or_insert(Folder {
                        name: map.folder.clone(),
                        hidden: map.folder == settings.maps.auto_folder
                            || topsky.overrides.contains(&OverrideSct {
                                folder: map.folder.clone(),
                                name: None,
                            }),
                        maps: HashMap::from([(map.name.clone(), map)]),
                    });
            } else {
                eprintln!("No colour or colour not found in map `{}`", topsky_map.name)
            }

            folders
        })
}

// TODO make test (used to be in blip, but logic moved here)
// rules: vec![
//     MapRule::Active(Active::Aup(vec![])),
//     MapRule::Active(Active::Notam("TEST".to_string(), vec![])),
//     MapRule::AndActive(Active::Id(ActiveIds {
//         own: None,
//         own_excludes: None,
//         online: None,
//         online_excludes: None,
//     })),
// ],
// active: vec![
//     vec![Active::Aup(vec![])],
//     vec![
//         Active::Notam("TEST".to_string(), vec![]),
//         Active::Id(ActiveIds {
//             own: None,
//             own_excludes: None,
//             online: None,
//             online_excludes: None,
//         }),
//     ],
// ],
