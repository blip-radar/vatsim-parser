use bevy_reflect::Reflect;
use pest::iterators::{Pair, Pairs};
use serde::Serialize;

use crate::{Color, Coordinate, DegMinSec};

use super::{parse_point, Rule};

#[derive(Clone, Debug, Eq, PartialEq, Hash, Reflect, Serialize)]
pub struct Runway {
    pub icao: String,
    pub designator: String,
}
impl Runway {
    fn parse(pair: Pair<Rule>) -> Self {
        let mut rwy = pair.into_inner();
        let icao = rwy.next().unwrap().as_str().to_string();
        let designator = rwy.next().unwrap().as_str().to_string();
        Self { icao, designator }
    }
}

#[derive(Clone, Debug, PartialEq, Reflect, Serialize)]
pub enum ActiveRunwaysType {
    Wildcard,
    Active(Vec<Runway>),
}
impl ActiveRunwaysType {
    fn parse(pair: Pair<Rule>) -> Self {
        match pair.as_rule() {
            Rule::wildcard => ActiveRunwaysType::Wildcard,
            Rule::runways => {
                ActiveRunwaysType::Active(pair.into_inner().map(Runway::parse).collect())
            }
            rule => {
                eprintln!("{rule:?}");
                unreachable!()
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Reflect, Serialize)]
pub struct ActiveRunways {
    pub arrival: ActiveRunwaysType,
    pub arrival_excludes: ActiveRunwaysType,
    pub departure: ActiveRunwaysType,
    pub departure_excludes: ActiveRunwaysType,
}

impl ActiveRunways {
    fn parse(pair: Pair<Rule>) -> Self {
        let mut active = pair.into_inner();
        let arrival = ActiveRunwaysType::parse(active.next().unwrap());
        let departure = ActiveRunwaysType::parse(active.next().unwrap());
        Self {
            arrival,
            arrival_excludes: ActiveRunwaysType::Wildcard,
            departure,
            departure_excludes: ActiveRunwaysType::Wildcard,
        }
    }

    fn parse_with_excludes(pair: Pair<Rule>) -> Self {
        let mut active = pair.into_inner();
        let arrival = ActiveRunwaysType::parse(active.next().unwrap());
        let arrival_excludes = ActiveRunwaysType::parse(active.next().unwrap());
        let departure = ActiveRunwaysType::parse(active.next().unwrap());
        let departure_excludes = ActiveRunwaysType::parse(active.next().unwrap());
        Self {
            arrival,
            arrival_excludes,
            departure,
            departure_excludes,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Reflect, Serialize)]
pub enum Active {
    True,
    False,
    Schedule,
    Id,
    Runway(ActiveRunways),
}

impl Active {
    fn parse(pair: Pair<Rule>) -> Self {
        let active = pair.into_inner().next().unwrap();
        match active.as_rule() {
            Rule::active_always => Self::True,
            Rule::active_id => Self::Id,          // TODO
            Rule::active_sched => Self::Schedule, // TODO
            Rule::active_rwy => Self::Runway(ActiveRunways::parse(active)),
            Rule::active_rwy_with_excludes => {
                Self::Runway(ActiveRunways::parse_with_excludes(active))
            }
            rule => {
                eprintln!("{rule:?}");
                unreachable!()
            }
        }
    }
}

enum CoordinatePart {
    Decimal(f64),
    DegMinSec(DegMinSec),
}
impl CoordinatePart {
    fn parse(pair: Pair<Rule>) -> Self {
        let coordinate_part = pair.into_inner().next().unwrap();
        match coordinate_part.as_rule() {
            Rule::decimal => Self::Decimal(coordinate_part.as_str().parse().unwrap()),
            Rule::sct_coord_part => {
                let mut sct_coord_part = coordinate_part.into_inner();
                let hemi = sct_coord_part.next().unwrap().as_str();
                let degrees = sct_coord_part
                    .next()
                    .unwrap()
                    .as_str()
                    .parse::<f64>()
                    .unwrap()
                    * match hemi {
                        "N" | "E" => 1.0,
                        "S" | "W" => -1.0,
                        _ => unreachable!(),
                    };
                let min = sct_coord_part.next().unwrap().as_str().parse().unwrap();
                let sec = sct_coord_part.next().unwrap().as_str().parse().unwrap();

                Self::DegMinSec((degrees, min, sec))
            }
            _ => unreachable!(),
        }
    }
}

impl Coordinate {
    fn parse(pair: Pair<Rule>) -> Coordinate {
        let mut coordinate = pair.into_inner();
        let lat = CoordinatePart::parse(coordinate.next().unwrap());
        let lng = CoordinatePart::parse(coordinate.next().unwrap());
        match (lat, lng) {
            (CoordinatePart::Decimal(lat), CoordinatePart::Decimal(lng)) => Coordinate { lat, lng },
            (CoordinatePart::DegMinSec(lat), CoordinatePart::DegMinSec(lng)) => {
                Coordinate::from_deg_min_sec(lat, lng)
            }
            _ => panic!("Consistency!"),
        }
    }
}

#[derive(Clone, Debug, Reflect, Serialize)]
pub enum Location {
    Fix(String),
    Coordinate(Coordinate),
}
impl Location {
    fn parse(pair: Pair<Rule>) -> Self {
        match pair.as_rule() {
            Rule::name => Self::Fix(pair.as_str().to_string()),
            Rule::coordinate => Self::Coordinate(Coordinate::parse(pair)),
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Debug, Reflect, Serialize)]
pub struct Label {
    pub text: String,
    pub pos: (f64, f64),
}

#[derive(Clone, Debug, Reflect, Serialize)]
pub struct MapSymbol {
    pub name: String,
    pub location: Location,
    pub label: Option<Label>,
}
impl MapSymbol {
    fn parse(pair: Pair<Rule>) -> Self {
        let mut symbol = pair.into_inner();
        let name = symbol.next().unwrap().as_str().to_string();
        let location = Location::parse(symbol.next().unwrap());
        let label = symbol.next().map(|label_pair| {
            let mut label_pairs = label_pair.into_inner();
            let text = label_pairs.next().unwrap().as_str().to_string();
            let pos = parse_point(label_pairs.next().unwrap());
            Label { text, pos }
        });

        Self {
            name,
            location,
            label,
        }
    }
}

#[derive(Clone, Debug, Reflect, Serialize)]
pub enum HorizontalAlignment {
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug, Reflect, Serialize)]
pub enum VerticalAlignment {
    Top,
    Center,
    Bottom,
}

#[derive(Clone, Debug, Reflect, Serialize)]
pub struct Alignment {
    pub horizontal: HorizontalAlignment,
    pub vertical: VerticalAlignment,
}

#[derive(Clone, Debug, Reflect, Serialize)]
pub struct Text {
    pub location: Location,
    pub content: String,
    pub alignment: Option<Alignment>,
}
impl Text {
    fn parse(pair: Pair<Rule>) -> Self {
        let mut text = pair.into_inner();
        let mut name_or_location = text.next().unwrap();
        let alignment = if name_or_location.as_rule() == Rule::textalign {
            // FIXME parse alignment
            name_or_location = text.next().unwrap();
            None
        } else {
            None
        };
        let location = Location::parse(name_or_location);
        let content = text.next().unwrap().as_str().to_string();

        Self {
            content,
            location,
            alignment,
        }
    }
}

#[derive(Clone, Debug, Reflect, Serialize)]
pub enum FontSize {
    Exact(i32),
    Add(i32),
    Subtract(i32),
    Multiply(i32),
    Default,
}

impl FontSize {
    fn parse(pair: Pair<Rule>) -> Self {
        let mut fontsize = pair.into_inner();
        if fontsize.as_str() == "0" {
            Self::Default
        } else {
            let modifier = fontsize.next().unwrap().as_str();
            let size = fontsize.next().unwrap().as_str().parse().unwrap();
            match modifier {
                "=" => Self::Exact(size),
                "+" => Self::Add(size),
                "-" => Self::Subtract(size),
                "*" => Self::Multiply(size),
                _ => unreachable!(),
            }
        }
    }
}

#[derive(Clone, Debug, Reflect, Serialize)]
pub enum LineStyleType {
    Solid,
    Alternate,
    Dot,
    Dash,
    DashDot,
    DashDotDot,
    Custom(String),
}
#[derive(Clone, Debug, Reflect, Serialize)]
pub struct LineStyle {
    pub width: i32,
    pub style: LineStyleType,
}
impl LineStyle {
    fn parse(pair: Pair<Rule>) -> Self {
        let mut linestyle = pair.into_inner();
        let upper_style = linestyle.next().unwrap().as_str().to_uppercase();
        let style = match &*upper_style {
            "DEFAULT" | "SOLID" => LineStyleType::Solid,
            "ALTERNATE" => LineStyleType::Alternate,
            "DASH" => LineStyleType::Dash,
            "DOT" => LineStyleType::Dot,
            "DASHDOT" => LineStyleType::DashDot,
            "DASHDOTDOT" => LineStyleType::DashDotDot,
            _ => LineStyleType::Custom(upper_style),
        };
        let width = linestyle
            .next()
            .and_then(|w| w.as_str().parse().ok())
            .unwrap_or(1);

        Self { width, style }
    }
}

#[derive(Clone, Debug, Reflect, Serialize)]
pub struct MapLine {
    pub start: Location,
    pub end: Location,
}
impl MapLine {
    fn parse(pair: Pair<Rule>) -> Self {
        let mut line = pair.into_inner();
        let mut locations = line.next().unwrap().into_inner();
        let start = Location::parse(locations.next().unwrap());
        let end = Location::parse(locations.next().unwrap());

        Self { start, end }
    }
}

#[derive(Clone, Debug, Reflect, Serialize)]
pub enum MapRule {
    Folder(String),
    Color(String),
    AsrData,
    Active(Active),
    ScreenSpecific,
    Layer(i32),
    Symbol(MapSymbol),
    Zoom(i32),
    FontSize(FontSize),
    LineStyle(LineStyle),
    Line(MapLine),
    Text(Text),
}

impl MapRule {
    fn parse(pairs: Pairs<Rule>) -> Vec<MapRule> {
        pairs
            .filter_map(|pair| {
                let ruletype = pair.as_rule();
                match ruletype {
                    Rule::folder => Some(MapRule::Folder(
                        pair.into_inner().next().unwrap().as_str().to_string(),
                    )),
                    Rule::color => Some(MapRule::Color(
                        pair.into_inner().next().unwrap().as_str().to_string(),
                    )),
                    Rule::asrdata => Some(MapRule::AsrData), // TODO
                    Rule::active => Some(MapRule::Active(Active::parse(pair))),
                    Rule::layer => Some(MapRule::Layer(
                        pair.into_inner().next().unwrap().as_str().parse().unwrap(),
                    )),
                    Rule::mapsymbol => Some(MapRule::Symbol(MapSymbol::parse(pair))),
                    Rule::fontsize => Some(MapRule::FontSize(FontSize::parse(pair))),
                    Rule::zoom => Some(MapRule::Zoom(
                        pair.into_inner().next().unwrap().as_str().parse().unwrap(),
                    )),
                    Rule::style => Some(MapRule::LineStyle(LineStyle::parse(pair))),
                    Rule::mapline => Some(MapRule::Line(MapLine::parse(pair))),
                    Rule::text => Some(MapRule::Text(Text::parse(pair))),
                    Rule::screen_specific => Some(MapRule::ScreenSpecific),
                    Rule::circle => None,
                    Rule::coordline => None,
                    Rule::coord => None,
                    Rule::coordpoly => None,
                    _ => {
                        eprintln!("unhandled {ruletype:?}");
                        unreachable!()
                    }
                }
            })
            .collect::<Vec<_>>()
    }
}

#[derive(Clone, Debug, Reflect, Serialize)]
pub struct Map {
    pub name: String,
    pub folder: String,
    pub color: String,
    pub rules: Vec<MapRule>,
    pub active: Active,
}

#[derive(Clone, Debug, Reflect, Serialize)]
pub struct OverrideSct {
    pub folder: Option<String>,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Reflect, Serialize)]
pub struct ColorDef {
    pub name: String,
    pub color: Color,
}

pub(super) fn parse_map(pair: Pair<Rule>) -> Option<Map> {
    match pair.as_rule() {
        Rule::map => {
            let mut symbol = pair.into_inner();
            let name = symbol.next().unwrap().as_str().to_string();
            let rules = MapRule::parse(symbol);
            let folder = rules
                .iter()
                .find_map(|rule| {
                    if let MapRule::Folder(folder) = rule {
                        Some(folder.clone())
                    } else {
                        None
                    }
                })
                .unwrap_or("DEFAULT".to_string());
            let maybe_color = rules.iter().find_map(|rule| {
                if let MapRule::Color(color) = rule {
                    Some(color.clone())
                } else {
                    None
                }
            });
            let active = rules
                .iter()
                .find_map(|rule| {
                    if let MapRule::Active(active) = rule {
                        Some(active.clone())
                    } else {
                        None
                    }
                })
                .unwrap_or(Active::False);
            if maybe_color.is_none() {
                eprintln!("map {name} doesn't include color");
            }
            maybe_color.map(|color| Map {
                active,
                name,
                folder,
                color,
                rules,
            })
        }
        Rule::EOI => None,
        _ => {
            eprintln!("{:?}", pair.as_rule());
            unreachable!()
        }
    }
}

pub(super) fn parse_color(pair: Pair<Rule>) -> Option<ColorDef> {
    match pair.as_rule() {
        Rule::colordef => {
            let mut color = pair.into_inner();
            let name = color.next().unwrap().as_str().to_string();
            let r = color.next().unwrap().as_str().parse().unwrap();
            let g = color.next().unwrap().as_str().parse().unwrap();
            let b = color.next().unwrap().as_str().parse().unwrap();
            Some(ColorDef {
                name,
                color: Color::from_rgb(r, g, b),
            })
        }
        Rule::EOI => None,
        rule => {
            eprintln!("{rule:?}");
            unreachable!()
        }
    }
}

pub(super) fn parse_override(pair: Pair<Rule>) -> Option<OverrideSct> {
    match pair.as_rule() {
        Rule::override_sct => {
            let mut override_sct = pair.into_inner();
            let folder = override_sct.next().and_then(|folder| {
                if folder.as_str().is_empty() {
                    None
                } else {
                    Some(folder.as_str().to_string())
                }
            });
            let name = override_sct.next().unwrap().as_str().to_string();
            Some(OverrideSct { folder, name })
        }
        Rule::EOI => None,
        rule => {
            eprintln!("{rule:?}");
            unreachable!()
        }
    }
}
