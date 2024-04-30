mod active;

use std::collections::HashMap;

use bevy_reflect::Reflect;
use geo_types::Coord;
use pest::{
    iterators::{Pair, Pairs},
    Parser,
};
use serde::Serialize;

use crate::{read_to_string, Color, DegMinSec, FromDegMinSec, Location};

pub use self::active::{Active, ActiveIds, ActiveRunways};

use super::{
    parse_point,
    symbol::{parse_symbol, SymbolDef},
    Rule, TopskyError, TopskyParser,
};

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
                        "N" | "n" | "E" | "e" => 1.0,
                        "S" | "s" | "W" | "w" => -1.0,
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

fn parse_coord(pair: Pair<Rule>) -> Coord {
    let mut coordinate = pair.into_inner();
    let lat = CoordinatePart::parse(coordinate.next().unwrap());
    let lng = CoordinatePart::parse(coordinate.next().unwrap());
    match (lat, lng) {
        (CoordinatePart::Decimal(y), CoordinatePart::Decimal(x)) => Coord { x, y },
        (CoordinatePart::DegMinSec(lat), CoordinatePart::DegMinSec(lng)) => {
            Coord::from_deg_min_sec(lat, lng)
        }
        _ => panic!("Consistency!"),
    }
}

impl Location {
    fn parse(pair: Pair<Rule>) -> Self {
        match pair.as_rule() {
            Rule::name => Self::Fix(pair.as_str().to_string()),
            Rule::coordinate => Self::Coordinate(parse_coord(pair)),
            ruletype => {
                eprintln!("unhandled {ruletype:?}");
                unreachable!()
            }
        }
    }
}

#[derive(Clone, Debug, Reflect, Serialize, PartialEq)]
pub struct Label {
    pub text: String,
    pub pos: (f64, f64),
}

#[derive(Clone, Debug, Reflect, Serialize, PartialEq)]
pub struct MapSymbol {
    pub name: String,
    pub location: Location,
    pub label: Option<Label>,
    pub label_alignment: Option<Alignment>,
}
impl MapSymbol {
    fn parse(pair: Pair<Rule>) -> Self {
        let mut symbol = pair.into_inner();
        let label_alignment_or_name = symbol.next().unwrap();
        let (label_alignment, name) =
            if matches!(label_alignment_or_name.as_rule(), Rule::textalign_config) {
                // FIXME parse alignment
                (None, symbol.next().unwrap().as_str().to_string())
            } else {
                (None, label_alignment_or_name.as_str().to_string())
            };
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
            label_alignment,
        }
    }
}

#[derive(Clone, Debug, Reflect, Serialize, PartialEq, Eq)]
pub enum HorizontalAlignment {
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug, Reflect, Serialize, PartialEq, Eq)]
pub enum VerticalAlignment {
    Top,
    Center,
    Bottom,
}

#[derive(Clone, Debug, Reflect, Serialize, PartialEq, Eq)]
pub struct Alignment {
    pub horizontal: HorizontalAlignment,
    pub vertical: VerticalAlignment,
}

#[derive(Clone, Debug, Reflect, Serialize, PartialEq)]
pub struct Text {
    pub location: Location,
    pub content: String,
    pub alignment: Option<Alignment>,
}
impl Text {
    fn parse(pair: Pair<Rule>) -> Self {
        let mut text = pair.into_inner();
        let mut name_or_location = text.next().unwrap();
        let alignment = if name_or_location.as_rule() == Rule::textalign_config {
            // FIXME parse alignment
            name_or_location = text.next().unwrap();
            None
        } else {
            None
        };
        let location = Location::parse(name_or_location);
        let content = text.next().unwrap().as_str().to_string();

        Self {
            location,
            content,
            alignment,
        }
    }
}

#[derive(Clone, Debug, Reflect, Serialize, PartialEq)]
pub enum FontSize {
    Exact(f64),
    Add(f64),
    Subtract(f64),
    Multiply(f64),
    Default,
}

impl FontSize {
    fn parse(pair: Pair<Rule>) -> Self {
        let mut fontsize = pair.into_inner();
        let default_or_modifier = fontsize.next().unwrap();
        if matches!(default_or_modifier.as_rule(), Rule::fontsize_default) {
            Self::Default
        } else {
            let size = fontsize.next().unwrap().as_str().parse().unwrap();
            match default_or_modifier.as_str() {
                "=" => Self::Exact(size),
                "+" => Self::Add(size),
                "-" => Self::Subtract(size),
                "*" => Self::Multiply(size),
                _ => unreachable!(),
            }
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

#[derive(Clone, Debug, Reflect, Serialize, PartialEq)]
pub struct MapLine {
    pub points: Vec<Location>,
}
impl MapLine {
    fn parse(pair: Pair<Rule>) -> Vec<Self> {
        let lines = pair.into_inner();
        lines.fold(vec![], |mut acc, pair| {
            let mut line = pair.into_inner();
            let start = Location::parse(line.next().unwrap());
            let end = Location::parse(line.next().unwrap());
            if let Some(last_line) = acc.last_mut() {
                if let Some(last_loc) = last_line.points.last() {
                    if *last_loc == start {
                        if *last_loc != end {
                            last_line.points.push(end);
                        }

                        return acc;
                    }
                }
            }

            acc.push(Self {
                points: if start == end {
                    vec![start]
                } else {
                    vec![start, end]
                },
            });

            acc
        })
    }
}

#[derive(Clone, Debug, Reflect, Serialize, PartialEq)]
pub enum MapRule {
    Folder(String),
    Color(String),
    AsrData(Option<Vec<String>>),
    Active(Active),
    AndActive(Active),
    Global,
    ScreenSpecific,
    Layer(i32),
    Symbol(MapSymbol),
    Zoom(f32),
    FontSize(FontSize),
    LineStyle(LineStyle),
    Line(Vec<MapLine>),
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
                    Rule::asrdata => Some(MapRule::AsrData({
                        let data = pair.into_inner().next().unwrap();
                        match data.as_rule() {
                            Rule::wildcard => None,
                            Rule::names => Some(
                                data.into_inner()
                                    .map(|pair| pair.as_str().to_string())
                                    .collect(),
                            ),
                            _ => {
                                eprintln!("unhandled {ruletype:?}");
                                unreachable!()
                            }
                        }
                    })),
                    Rule::active => Some(MapRule::Active(Active::parse(pair))),
                    Rule::andactive => Some(MapRule::AndActive(Active::parse(pair))),
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
                    Rule::global => Some(MapRule::Global),
                    // TODO
                    Rule::circle => None,
                    Rule::coordline => None,
                    Rule::coord => None,
                    Rule::coordpoly => None,
                    Rule::fontstyle => None,
                    Rule::textalign => None,
                    Rule::override_sct => None,
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

#[derive(Clone, Debug, PartialEq, Reflect, Serialize)]
pub struct LineStyleDef {
    pub name: String,
    pub brush: String,
    pub hatch: String,
    pub dash_lengths: Vec<i32>,
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
                .unwrap_or("AUTO".to_string());
            let maybe_color = rules.iter().find_map(|rule| {
                if let MapRule::Color(color) = rule {
                    Some(color.clone())
                } else {
                    None
                }
            });
            if maybe_color.is_none() {
                eprintln!("map {name} doesn't include color");
            }
            maybe_color.map(|color| Map {
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

pub(super) fn parse_linestyle(pair: Pair<Rule>) -> Option<LineStyleDef> {
    match pair.as_rule() {
        Rule::linestyledef => {
            let mut color = pair.into_inner();
            let name = color.next().unwrap().as_str().to_string();
            let brush = color.next().unwrap().as_str().to_string();
            let hatch = color.next().unwrap().as_str().to_string();
            let dash_lengths = color.map(|pair| pair.as_str().parse().unwrap()).collect();
            Some(LineStyleDef {
                name,
                brush,
                hatch,
                dash_lengths,
            })
        }
        Rule::EOI => None,
        rule => {
            eprintln!("{rule:?}");
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

#[derive(Debug)]
pub enum MapDefinition {
    Map(Map),
    Color(ColorDef),
    Symbol(SymbolDef),
    Override(OverrideSct),
    LineStyle(LineStyleDef),
}
type ParseMapResult = Result<
    (
        HashMap<String, Map>,
        HashMap<String, SymbolDef>,
        HashMap<String, ColorDef>,
        HashMap<String, LineStyleDef>,
        Vec<OverrideSct>,
    ),
    TopskyError,
>;
pub(super) fn parse_topsky_maps(file_contents: &[u8]) -> ParseMapResult {
    TopskyParser::parse(Rule::maps, &read_to_string(file_contents)?)
        .map(|mut pairs| {
            pairs
                .next()
                .unwrap()
                .into_inner()
                .filter_map(|pair| match pair.as_rule() {
                    Rule::map => parse_map(pair).map(MapDefinition::Map),
                    Rule::colordef => parse_color(pair).map(MapDefinition::Color),
                    Rule::symboldef => parse_symbol(pair).map(MapDefinition::Symbol),
                    Rule::linestyledef => parse_linestyle(pair).map(MapDefinition::LineStyle),
                    Rule::override_sct => parse_override(pair).map(MapDefinition::Override),
                    Rule::EOI => None,
                    _ => {
                        eprintln!("{:?}", pair.as_rule());
                        unreachable!()
                    }
                })
                .fold(
                    (
                        HashMap::new(),
                        HashMap::new(),
                        HashMap::new(),
                        HashMap::new(),
                        vec![],
                    ),
                    |(mut maps, mut symbols, mut colors, mut line_styles, mut overrides), def| {
                        match def {
                            MapDefinition::Map(mut map) => {
                                if maps.contains_key(&map.name) {
                                    let mut i = 2;
                                    while maps.contains_key(&format!("{}_{i}", map.name)) {
                                        i += 1;
                                    }
                                    map.name = format!("{}_{i}", map.name);
                                }

                                maps.insert(map.name.clone(), map);
                            }
                            MapDefinition::Color(color) => {
                                colors.insert(color.name.clone(), color);
                            }
                            MapDefinition::Symbol(symbol) => {
                                symbols.insert(symbol.name.clone(), symbol);
                            }
                            MapDefinition::LineStyle(line_style) => {
                                line_styles.insert(line_style.name.clone(), line_style);
                            }
                            MapDefinition::Override(override_sct) => {
                                overrides.push(override_sct);
                            }
                        };
                        (maps, symbols, colors, line_styles, overrides)
                    },
                )
        })
        .map_err(Into::into)
}

#[cfg(test)]
mod test {
    use crate::topsky::map::{
        parse_topsky_maps, Active, ActiveIds, ActiveRunways, MapRule, Runway,
    };

    #[test]
    fn test_active() {
        let maps_str = br"
MAP:SYMBOLS
FOLDER:FIXES
COLOR:Active_Map_Type_20
ACTIVE:1

MAP:AOR ALTMUEHL
FOLDER:SECTORLINES
COLOR:Active_Map_Type_20
ASRDATA:CTR,EDDM_APP
STYLE:Dot:1
LAYER:-2
ACTIVE:ID:*:*:IGL:*
ACTIVE:ID:IGL:*:*:*

MAP:EDMO_RNP22
ASRDATA:APP,CTR
ZOOM:9
COLOR:Active_Map_Type_20
ACTIVE:RWY:ARR:EDMO22:DEP:*
";
        let (mut maps, ..) = parse_topsky_maps(maps_str).unwrap();

        assert_eq!(
            maps.remove("SYMBOLS")
                .unwrap()
                .rules
                .into_iter()
                .filter(|rule| matches!(rule, MapRule::Active(_)))
                .collect::<Vec<_>>(),
            vec![MapRule::Active(Active::True)]
        );

        assert_eq!(
            maps.remove("AOR ALTMUEHL")
                .unwrap()
                .rules
                .into_iter()
                .filter(|rule| matches!(rule, MapRule::Active(_)))
                .collect::<Vec<_>>(),
            vec![
                MapRule::Active(Active::Id(ActiveIds {
                    own: None,
                    own_excludes: None,
                    online: Some(vec!["IGL".to_string()]),
                    online_excludes: None,
                })),
                MapRule::Active(Active::Id(ActiveIds {
                    own: Some(vec!["IGL".to_string()]),
                    own_excludes: None,
                    online: None,
                    online_excludes: None,
                }))
            ]
        );

        assert_eq!(
            maps.remove("EDMO_RNP22")
                .unwrap()
                .rules
                .into_iter()
                .filter(|rule| matches!(rule, MapRule::Active(_)))
                .collect::<Vec<_>>(),
            vec![MapRule::Active(Active::Runway(ActiveRunways {
                arrival: Some(vec![Runway {
                    icao: "EDMO".to_string(),
                    designator: "22".to_string()
                }]),
                arrival_excludes: None,
                departure: None,
                departure_excludes: None,
            }))]
        );
    }

    #[test]
    fn test_asrdata() {
        let maps_str = br"
MAP:SYMBOLS
FOLDER:FIXES
COLOR:Active_Map_Type_20
ACTIVE:1

MAP:AOR ALTMUEHL
FOLDER:SECTORLINES
COLOR:Active_Map_Type_20
ASRDATA:CTR,EDDM_APP
STYLE:Dot:1
LAYER:-2
ACTIVE:ID:*:*:IGL:*
ACTIVE:ID:IGL:*:*:*

MAP:EDMO_RNP22
ASRDATA:APP,CTR
ZOOM:9
COLOR:Active_Map_Type_20
ACTIVE:RWY:ARR:EDMO22:DEP:*
";
        let (mut maps, ..) = parse_topsky_maps(maps_str).unwrap();

        assert_eq!(
            maps.remove("SYMBOLS")
                .unwrap()
                .rules
                .into_iter()
                .filter(|rule| matches!(rule, MapRule::AsrData(_)))
                .collect::<Vec<_>>(),
            vec![]
        );

        assert_eq!(
            maps.remove("AOR ALTMUEHL")
                .unwrap()
                .rules
                .into_iter()
                .filter(|rule| matches!(rule, MapRule::AsrData(_)))
                .collect::<Vec<_>>(),
            vec![MapRule::AsrData(Some(vec![
                "CTR".to_string(),
                "EDDM_APP".to_string()
            ]))]
        );

        assert_eq!(
            maps.remove("EDMO_RNP22")
                .unwrap()
                .rules
                .into_iter()
                .filter(|rule| matches!(rule, MapRule::AsrData(_)))
                .collect::<Vec<_>>(),
            vec![MapRule::AsrData(Some(vec![
                "APP".to_string(),
                "CTR".to_string()
            ]))]
        );
    }
}
