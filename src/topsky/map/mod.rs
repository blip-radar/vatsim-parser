pub mod active;

use std::collections::HashMap;

use bevy_reflect::Reflect;
use geo::Coord;
use pest::{
    iterators::{Pair, Pairs},
    Parser,
};
use serde::Serialize;

use crate::{
    adaptation::{colours::Colour, line_styles::LineStyle, maps::active::Active, Alignment},
    read_to_string, DegMinSec, FromDegMinSec, Location,
};

use super::{
    parse_point,
    symbol::{parse_symbol, SymbolDef},
    Rule, TopskyError, TopskyParser,
};

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
                        _ => unreachable!("{hemi} is not a hemisphere"),
                    };
                let min = sct_coord_part.next().unwrap().as_str().parse().unwrap();
                let sec = sct_coord_part.next().unwrap().as_str().parse().unwrap();

                Self::DegMinSec((degrees, min, sec))
            }
            rule => unreachable!("{rule:?}"),
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
            Rule::colon_delimited_text => Self::Fix(pair.as_str().to_string()),
            Rule::coordinate => Self::Coordinate(parse_coord(pair)),
            rule => unreachable!("{rule:?}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Label {
    pub text: String,
    /// scaled unprojected offset in pixels
    pub pos: (f64, f64),
}
#[derive(Clone, Debug, PartialEq, Serialize)]
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

#[derive(Clone, Debug, PartialEq, Serialize)]
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
    Exact(f32),
    Add(f32),
    Subtract(f32),
    Multiply(f32),
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
                modifier => unreachable!("invalid font size modifier: {modifier}"),
            }
        }
    }
}

impl LineStyle {
    fn parse(pair: Pair<Rule>) -> Self {
        let mut linestyle = pair.into_inner();
        let style = linestyle.next().unwrap().as_str().to_uppercase();
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

#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum MapRule {
    Folder(String),
    Colour(String),
    AsrData(Option<Vec<String>>),
    Active(Active),
    AndActive(Active),
    Global,
    ScreenSpecific,
    Hidden,
    Layer(f32),
    Symbol(MapSymbol),
    Zoom(f32),
    FontSize(FontSize),
    LineStyle(LineStyle),
    Line(Vec<MapLine>),
    Text(Text),
    CoordPoly(String),
    CoordLine,
    Coord(Location),
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
                    // TODO missing optional :FillColorName:FillBgColorName
                    Rule::colour => Some(MapRule::Colour(
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
                            rule => unreachable!("{rule:?}"),
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
                    Rule::hidden => Some(MapRule::Hidden),
                    Rule::coordline => Some(MapRule::CoordLine),
                    Rule::coord => Some(MapRule::Coord(Location::parse(
                        pair.into_inner().next().unwrap(),
                    ))),
                    Rule::coordpoly => Some(MapRule::CoordPoly(
                        pair.into_inner().next().unwrap().as_str().to_string(),
                    )),
                    // TODO
                    Rule::circle => None,
                    Rule::fontstyle => None,
                    Rule::textalign => None,
                    Rule::override_sct => None,
                    Rule::sctfiledata => None,
                    Rule::sctdata => None,
                    rule => unreachable!("{rule:?}"),
                }
            })
            .collect::<Vec<_>>()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Reflect, Serialize)]
pub struct OverrideSct {
    pub folder: String,
    pub name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Reflect, Serialize)]
pub struct ColourDef {
    pub name: String,
    pub colour: Colour,
}

#[derive(Clone, Debug, PartialEq, Reflect, Serialize)]
pub struct LineStyleDef {
    pub name: String,
    pub brush: String,
    pub hatch: String,
    pub dash_lengths: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MapDef {
    pub name: String,
    pub rules: Vec<MapRule>,
}

pub(super) fn parse_map(pair: Pair<Rule>) -> Option<MapDef> {
    match pair.as_rule() {
        Rule::map => {
            let mut symbol = pair.into_inner();
            let name = symbol.next().unwrap().as_str().to_string();
            let rules = MapRule::parse(symbol);
            Some(MapDef { name, rules })
        }
        Rule::EOI => None,
        rule => unreachable!("{rule:?}"),
    }
}

pub(super) fn parse_linestyle(pair: Pair<Rule>) -> Option<LineStyleDef> {
    match pair.as_rule() {
        Rule::linestyledef => {
            let mut colour = pair.into_inner();
            let name = colour.next().unwrap().as_str().to_string();
            let brush = colour.next().unwrap().as_str().to_string();
            let hatch = colour.next().unwrap().as_str().to_string();
            let dash_lengths = colour.map(|pair| pair.as_str().parse().unwrap()).collect();
            Some(LineStyleDef {
                name,
                brush,
                hatch,
                dash_lengths,
            })
        }
        Rule::EOI => None,
        rule => unreachable!("{rule:?}"),
    }
}

pub(super) fn parse_colour(pair: Pair<Rule>) -> Option<ColourDef> {
    match pair.as_rule() {
        Rule::colourdef => {
            let mut colour = pair.into_inner();
            let name = colour.next().unwrap().as_str().to_string();
            let r = colour.next().unwrap().as_str().parse().unwrap();
            let g = colour.next().unwrap().as_str().parse().unwrap();
            let b = colour.next().unwrap().as_str().parse().unwrap();
            Some(ColourDef {
                name,
                colour: Colour::from_rgb(r, g, b),
            })
        }
        Rule::EOI => None,
        rule => unreachable!("{rule:?}"),
    }
}

pub(super) fn parse_override(pair: Pair<Rule>) -> Option<OverrideSct> {
    match pair.as_rule() {
        Rule::override_sct => {
            let mut override_sct = pair.into_inner();
            let folder = override_sct.next().unwrap().as_str().to_string();
            let name = override_sct.next().map(|name| name.as_str().to_string());
            Some(OverrideSct { folder, name })
        }
        Rule::EOI => None,
        rule => unreachable!("{rule:?}"),
    }
}

pub enum MapDefinition {
    Map(MapDef),
    Colour(ColourDef),
    Symbol(SymbolDef),
    Override(OverrideSct),
    LineStyle(LineStyleDef),
}
type ParseMapResult = Result<
    (
        Vec<MapDef>,
        HashMap<String, SymbolDef>,
        HashMap<String, ColourDef>,
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
                    Rule::colourdef => parse_colour(pair).map(MapDefinition::Colour),
                    Rule::symboldef => parse_symbol(pair).map(MapDefinition::Symbol),
                    Rule::linestyledef => parse_linestyle(pair).map(MapDefinition::LineStyle),
                    Rule::override_sct => parse_override(pair).map(MapDefinition::Override),
                    // TODO not implemented yet
                    Rule::sctfilepath => None,
                    Rule::EOI => None,
                    rule => unreachable!("{rule:?}"),
                })
                .fold(
                    (
                        vec![],
                        HashMap::new(),
                        HashMap::new(),
                        HashMap::new(),
                        vec![],
                    ),
                    |(mut maps, mut symbols, mut colours, mut line_styles, mut overrides), def| {
                        match def {
                            MapDefinition::Map(map) => {
                                maps.push(map);
                            }
                            MapDefinition::Colour(colour) => {
                                colours.insert(colour.name.clone(), colour);
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
                        (maps, symbols, colours, line_styles, overrides)
                    },
                )
        })
        .map_err(Into::into)
}

#[cfg(test)]
mod test {
    use crate::{
        adaptation::maps::active::{ActiveIds, ActiveRunways, RunwayIdentifier},
        topsky::map::{parse_topsky_maps, Active, MapRule},
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
        let (maps, ..) = parse_topsky_maps(maps_str).unwrap();

        assert_eq!(
            maps.iter()
                .find(|map| map.name == "SYMBOLS")
                .unwrap()
                .rules
                .iter()
                .filter(|rule| matches!(rule, MapRule::Active(_)))
                .collect::<Vec<_>>(),
            vec![&MapRule::Active(Active::True)]
        );

        assert_eq!(
            maps.iter()
                .find(|map| map.name == "AOR ALTMUEHL")
                .unwrap()
                .rules
                .iter()
                .filter(|rule| matches!(rule, MapRule::Active(_)))
                .collect::<Vec<_>>(),
            vec![
                &MapRule::Active(Active::Id(ActiveIds {
                    own: None,
                    own_excludes: None,
                    online: Some(vec!["IGL".to_string()]),
                    online_excludes: None,
                })),
                &MapRule::Active(Active::Id(ActiveIds {
                    own: Some(vec!["IGL".to_string()]),
                    own_excludes: None,
                    online: None,
                    online_excludes: None,
                }))
            ]
        );

        assert_eq!(
            maps.iter()
                .find(|map| map.name == "EDMO_RNP22")
                .unwrap()
                .rules
                .iter()
                .filter(|rule| matches!(rule, MapRule::Active(_)))
                .collect::<Vec<_>>(),
            vec![&MapRule::Active(Active::Runway(ActiveRunways {
                arrival: Some(vec![RunwayIdentifier {
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
        let (maps, ..) = parse_topsky_maps(maps_str).unwrap();

        let empty_rules: Vec<&MapRule> = vec![];
        assert_eq!(
            maps.iter()
                .find(|map| map.name == "SYMBOLS")
                .unwrap()
                .rules
                .iter()
                .filter(|rule| matches!(rule, MapRule::AsrData(_)))
                .collect::<Vec<&MapRule>>(),
            empty_rules
        );

        assert_eq!(
            maps.iter()
                .find(|map| map.name == "AOR ALTMUEHL")
                .unwrap()
                .rules
                .iter()
                .filter(|rule| matches!(rule, MapRule::AsrData(_)))
                .collect::<Vec<&MapRule>>(),
            vec![&MapRule::AsrData(Some(vec![
                "CTR".to_string(),
                "EDDM_APP".to_string()
            ]))]
        );

        assert_eq!(
            maps.iter()
                .find(|map| map.name == "EDMO_RNP22")
                .unwrap()
                .rules
                .iter()
                .filter(|rule| matches!(rule, MapRule::AsrData(_)))
                .collect::<Vec<_>>(),
            vec![&MapRule::AsrData(Some(vec![
                "APP".to_string(),
                "CTR".to_string()
            ]))]
        );
    }
}
