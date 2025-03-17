use std::fmt::Formatter;
use std::io;
use std::{collections::HashMap, fmt::Display};

use geo::Coord;
use itertools::Itertools as _;
use pest::{iterators::Pair, Parser};
use pest_derive::Parser;
use serde::Serialize;
use thiserror::Error;
use tracing::warn;

use crate::{
    adaptation::{
        colours::Colour,
        locations::{Fix, Runway, NDB, VOR},
    },
    DegMinSec, DegMinSecExt, Location,
};

use super::read_to_string;

#[derive(Parser)]
#[grammar = "pest/base.pest"]
#[grammar = "pest/sct.pest"]
pub struct SctParser;

#[derive(Error, Debug)]
pub enum SctError {
    #[error("failed to parse .sct file: {0}")]
    Parse(#[from] pest::error::Error<Rule>),
    #[error("failed to read .sct file: {0}")]
    FileRead(#[from] io::Error),
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Airport {
    pub designator: String,
    pub coordinate: Coord,
    pub ctr_airspace: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Airway {
    pub designator: String,
    pub start: Location,
    pub end: Location,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Region {
    pub name: String,
    pub colour_name: String,
    pub polygon: Vec<Coord>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Line {
    pub start: Location,
    pub end: Location,
    pub colour: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Geo {
    pub name: String,
    pub lines: Vec<Line>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Sid {
    pub name: String,
    pub lines: Vec<Line>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Star {
    pub name: String,
    pub lines: Vec<Line>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Artcc {
    pub name: String,
    pub lines: Vec<Line>,
}

// TODO ARTCC, SID, STAR, AIRWAY
#[derive(Debug, Serialize, PartialEq)]
pub struct Sct {
    pub info: SctInfo,
    pub colours: HashMap<String, Colour>,
    pub airports: Vec<Airport>,
    pub fixes: Vec<Fix>,
    pub ndbs: Vec<NDB>,
    pub vors: Vec<VOR>,
    pub runways: Vec<Runway>,
    pub sids: Vec<Sid>,
    pub stars: Vec<Star>,
    pub artccs_high: Vec<Artcc>,
    pub artccs: Vec<Artcc>,
    pub artccs_low: Vec<Artcc>,
    pub high_airways: Vec<Airway>,
    pub low_airways: Vec<Airway>,
    pub regions: Vec<Region>,
    pub geo: Vec<Geo>,
}

#[derive(Debug, Default, Serialize, PartialEq)]
pub struct SctInfo {
    pub name: String,
    pub default_callsign: String,
    pub default_airport: String,
    pub centre_point: Coord,
    pub miles_per_deg_lat: f64,
    pub miles_per_deg_lng: f64,
    pub magnetic_variation: f64,
    pub scale_factor: f64,
}

impl Display for SctInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "[INFO]")?;
        writeln!(f, "{}", self.name)?;
        writeln!(f, "{}", self.default_callsign)?;
        writeln!(f, "{}", self.default_airport)?;
        writeln!(f, "{}", self.centre_point.lat_deg_min_sec_fmt())?;
        writeln!(f, "{}", self.centre_point.lon_deg_min_sec_fmt())?;
        writeln!(f, "{}", self.miles_per_deg_lat)?;
        writeln!(f, "{}", self.miles_per_deg_lng)?;
        writeln!(f, "{}", self.magnetic_variation)?;
        writeln!(f, "{}", self.scale_factor)
    }
}
trait ToEuroscope {
    fn to_euroscope(&self) -> String;
}
impl ToEuroscope for Vec<VOR> {
    fn to_euroscope(&self) -> String {
        format!(
            "[VOR]\n{}\n",
            self.iter()
                .map(ToEuroscope::to_euroscope)
                .sorted()
                .join("\n")
        )
    }
}
impl ToEuroscope for VOR {
    fn to_euroscope(&self) -> String {
        format!(
            "{:<4} {} {}",
            self.designator,
            self.frequency,
            self.coordinate.deg_min_sec_fmt(),
        )
    }
}
impl ToEuroscope for Vec<NDB> {
    fn to_euroscope(&self) -> String {
        format!(
            "[NDB]\n{}\n",
            self.iter()
                .map(ToEuroscope::to_euroscope)
                .sorted()
                .join("\n")
        )
    }
}
impl ToEuroscope for NDB {
    fn to_euroscope(&self) -> String {
        format!(
            "{:<4} {} {}",
            self.designator,
            self.frequency,
            self.coordinate.deg_min_sec_fmt(),
        )
    }
}
impl ToEuroscope for Vec<Fix> {
    fn to_euroscope(&self) -> String {
        format!(
            "[FIXES]\n{}\n",
            self.iter()
                .map(ToEuroscope::to_euroscope)
                .sorted()
                .join("\n")
        )
    }
}
impl ToEuroscope for Fix {
    fn to_euroscope(&self) -> String {
        format!(
            "{:<5} {}",
            self.designator,
            self.coordinate.deg_min_sec_fmt()
        )
    }
}
impl ToEuroscope for Vec<Airport> {
    fn to_euroscope(&self) -> String {
        format!(
            "[AIRPORT]\n{}\n",
            self.iter()
                .map(ToEuroscope::to_euroscope)
                .sorted()
                .join("\n")
        )
    }
}
impl ToEuroscope for Airport {
    fn to_euroscope(&self) -> String {
        format!(
            "{} 000.000 {} {}",
            self.designator,
            self.coordinate.deg_min_sec_fmt(),
            self.ctr_airspace
        )
    }
}
impl ToEuroscope for Vec<Runway> {
    fn to_euroscope(&self) -> String {
        format!(
            "[RUNWAY]\n{}\n",
            self.iter()
                .sorted_by_key(|rwy| &rwy.aerodrome)
                .map(ToEuroscope::to_euroscope)
                .join("\n")
        )
    }
}
impl ToEuroscope for Runway {
    fn to_euroscope(&self) -> String {
        format!(
            "{:<3} {:<3} {:03} {:03} {} {} {}",
            self.designators.0,
            self.designators.1,
            self.headings.0,
            self.headings.1,
            self.location.0.deg_min_sec_fmt(),
            self.location.1.deg_min_sec_fmt(),
            self.aerodrome
        )
    }
}

impl ToEuroscope for Location {
    fn to_euroscope(&self) -> String {
        match self {
            Location::Fix(fix) => format!("{fix} {fix}"),
            Location::Coordinate(c) => c.deg_min_sec_fmt(),
        }
    }
}
impl ToEuroscope for Line {
    fn to_euroscope(&self) -> String {
        format!(
            "{} {}{}",
            self.start.to_euroscope(),
            self.end.to_euroscope(),
            self.colour
                .as_ref()
                .map_or(String::new(), |c| format!(" {c}"))
        )
    }
}
impl ToEuroscope for Vec<Geo> {
    fn to_euroscope(&self) -> String {
        format!(
            "[GEO]\n{}\n",
            self.iter()
                .sorted_by_key(|geo| &geo.name)
                .map(ToEuroscope::to_euroscope)
                .join("\n\n")
        )
    }
}
impl ToEuroscope for Geo {
    fn to_euroscope(&self) -> String {
        format!(
            "{:<40} {}{}",
            self.name,
            self.lines
                .first()
                .map_or(String::new(), ToEuroscope::to_euroscope),
            (self.lines.len() > 1)
                .then_some(
                    self.lines
                        .iter()
                        .skip(1)
                        .map(|l| format!("\n{:<40} {}", "", l.to_euroscope()))
                        .join("")
                )
                .unwrap_or_else(String::new)
        )
    }
}
impl ToEuroscope for Vec<Region> {
    fn to_euroscope(&self) -> String {
        format!(
            "[REGIONS]\n{}\n",
            self.iter()
                .sorted_by_key(|region| &region.name)
                .map(ToEuroscope::to_euroscope)
                .join("\n")
        )
    }
}
impl ToEuroscope for Region {
    fn to_euroscope(&self) -> String {
        format!(
            "REGIONNAME {}\n{:<26} {}{}",
            self.name,
            self.colour_name,
            self.polygon
                .first()
                .map_or(String::new(), DegMinSecExt::deg_min_sec_fmt),
            (self.polygon.len() > 1)
                .then_some(
                    self.polygon
                        .iter()
                        .skip(1)
                        .map(|c| format!("\n{:<26} {}", "", c.deg_min_sec_fmt()))
                        .join("")
                )
                .unwrap_or_else(String::new)
        )
    }
}
impl ToEuroscope for Vec<Airway> {
    fn to_euroscope(&self) -> String {
        format!(
            "{}\n",
            self.iter()
                .sorted_by_key(|airway| &airway.designator)
                .map(ToEuroscope::to_euroscope)
                .join("\n")
        )
    }
}
impl ToEuroscope for Airway {
    fn to_euroscope(&self) -> String {
        format!(
            "{:<10} {} {}",
            self.designator,
            self.start.to_euroscope(),
            self.end.to_euroscope()
        )
    }
}
impl ToEuroscope for Vec<Sid> {
    fn to_euroscope(&self) -> String {
        format!(
            "[SID]\n{}\n",
            self.iter()
                .sorted_by_key(|geo| &geo.name)
                .map(ToEuroscope::to_euroscope)
                .join("\n\n")
        )
    }
}
impl ToEuroscope for Sid {
    fn to_euroscope(&self) -> String {
        format!(
            "{:<40} {}{}",
            self.name,
            self.lines
                .first()
                .map_or(String::new(), ToEuroscope::to_euroscope),
            (self.lines.len() > 1)
                .then_some(
                    self.lines
                        .iter()
                        .skip(1)
                        .map(|l| format!("\n{:<40} {}", "", l.to_euroscope()))
                        .join("")
                )
                .unwrap_or_else(String::new)
        )
    }
}
impl ToEuroscope for Vec<Star> {
    fn to_euroscope(&self) -> String {
        format!(
            "[STAR]\n{}\n",
            self.iter()
                .sorted_by_key(|geo| &geo.name)
                .map(ToEuroscope::to_euroscope)
                .join("\n\n")
        )
    }
}
impl ToEuroscope for Star {
    fn to_euroscope(&self) -> String {
        format!(
            "{:<40} {}{}",
            self.name,
            self.lines
                .first()
                .map_or(String::new(), ToEuroscope::to_euroscope),
            (self.lines.len() > 1)
                .then_some(
                    self.lines
                        .iter()
                        .skip(1)
                        .map(|l| format!("\n{:<40} {}", "", l.to_euroscope()))
                        .join("")
                )
                .unwrap_or_else(String::new)
        )
    }
}
impl ToEuroscope for Vec<Artcc> {
    fn to_euroscope(&self) -> String {
        format!(
            "{}\n",
            self.iter()
                .sorted_by_key(|geo| &geo.name)
                .map(ToEuroscope::to_euroscope)
                .join("\n\n")
        )
    }
}
impl ToEuroscope for Artcc {
    fn to_euroscope(&self) -> String {
        format!(
            "{:<40} {}{}",
            self.name,
            self.lines
                .first()
                .map_or(String::new(), ToEuroscope::to_euroscope),
            (self.lines.len() > 1)
                .then_some(
                    self.lines
                        .iter()
                        .skip(1)
                        .map(|l| format!("\n{:<40} {}", "", l.to_euroscope()))
                        .join("")
                )
                .unwrap_or_else(String::new)
        )
    }
}

pub type SctResult = Result<Sct, SctError>;

#[derive(Debug)]
enum Section {
    Info(SctInfo),
    Airport(Vec<Airport>),
    Fixes(Vec<Fix>),
    HighAirways(Vec<Airway>),
    LowAirways(Vec<Airway>),
    NDBs(Vec<NDB>),
    VORs(Vec<VOR>),
    Runways(Vec<Runway>),
    Sid(Vec<Sid>),
    Star(Vec<Star>),
    ArtccHigh(Vec<Artcc>),
    Artcc(Vec<Artcc>),
    ArtccLow(Vec<Artcc>),
    Regions(Vec<Region>),
    Geo(Vec<Geo>),
    Unsupported,
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum SectionName {
    Info,
    Airport,
    HighAirway,
    LowAirway,
    Fixes,
    NDBs,
    VORs,
    Runways,
    Sid,
    Star,
    ArtccHigh,
    Artcc,
    ArtccLow,
    Regions,
    Geo,
    Unsupported,
}

fn parse_coordinate_part(pair: Pair<Rule>) -> DegMinSec {
    let mut coordinate_part = pair.into_inner();
    let hemi = coordinate_part.next().unwrap().as_str();
    let degrees_str = coordinate_part.next().unwrap().as_str();
    let degrees = degrees_str
        .parse::<i16>()
        .inspect_err(|e| warn!("Could not parse coordinate, {e}: {degrees_str}"))
        .unwrap()
        * match hemi {
            "N" | "n" | "E" | "e" => 1,
            "S" | "s" | "W" | "w" => -1,
            _ => unreachable!("{hemi} is not a hemisphere"),
        };
    let min = coordinate_part.next().unwrap().as_str().parse().unwrap();
    let sec = coordinate_part.next().unwrap().as_str().parse().unwrap();

    (degrees, min, sec)
}

fn parse_coordinate(pair: Pair<Rule>) -> Coord {
    let mut coordinate = pair.into_inner();
    let y = parse_coordinate_part(coordinate.next().unwrap());
    let x = parse_coordinate_part(coordinate.next().unwrap());
    Coord::from_deg_min_sec(y, x)
}

fn parse_airport(pair: Pair<Rule>) -> Airport {
    let mut location = pair.into_inner();
    let designator = location.next().unwrap().as_str().to_string();
    let coordinate = parse_coordinate(
        location
            .find(|pair| matches!(pair.as_rule(), Rule::sct_coordinate))
            .unwrap(),
    );
    let ctr_airspace = location.next().unwrap().as_str().to_string();

    Airport {
        designator,
        coordinate,
        ctr_airspace,
    }
}

fn parse_location(pair: Pair<Rule>) -> Location {
    match pair.as_rule() {
        Rule::sct_coordinate => Location::Coordinate(parse_coordinate(pair)),
        Rule::airway_fix => Location::Fix(pair.into_inner().next().unwrap().as_str().to_string()),
        rule => unreachable!("{rule:?}"),
    }
}

fn parse_airway(pair: Pair<Rule>) -> Option<Airway> {
    let mut airway = pair.into_inner();
    let designator = airway.next().unwrap().as_str().to_string();

    let (start, end) = if let (Some(start), Some(end)) = (airway.next(), airway.next()) {
        (parse_location(start), parse_location(end))
    } else {
        warn!("broken airway (initial parse): {airway:?}");
        return None;
    };

    Some(Airway {
        designator,
        start,
        end,
    })
}

fn parse_fix(pair: Pair<Rule>) -> Option<Fix> {
    if let Rule::fix = pair.as_rule() {
        let mut fix = pair.into_inner();
        let designator = fix.next().unwrap().as_str().to_string();
        let coordinate = parse_coordinate(fix.next().unwrap());

        Some(Fix {
            designator,
            coordinate,
        })
    } else {
        warn!("broken fix: {pair:?}");
        None
    }
}

fn parse_ndb(pair: Pair<Rule>) -> Option<NDB> {
    if let Rule::location = pair.as_rule() {
        let mut location = pair.into_inner();
        let designator = location.next().unwrap().as_str().to_string();
        let frequency = location.next().unwrap().as_str().to_string();
        let coordinate = parse_coordinate(location.next().unwrap());

        Some(NDB {
            designator,
            frequency,
            coordinate,
        })
    } else {
        warn!("broken ndb: {pair:?}");
        None
    }
}

fn parse_region(pair: Pair<Rule>) -> Region {
    let mut region = pair.into_inner();
    let name = region.next().unwrap().as_str().to_string();
    let colour_name = region.next().unwrap().as_str().to_string();
    let polygon = region.map(parse_coordinate).collect();

    Region {
        name,
        colour_name,
        polygon,
    }
}

fn parse_line(pair: Pair<Rule>) -> Option<Line> {
    let mut line = pair.into_inner();

    let (start, end) = if let (Some(start), Some(end)) = (line.next(), line.next()) {
        (parse_location(start), parse_location(end))
    } else {
        warn!("broken coloured line (initial parse): {line:?}");
        return None;
    };
    let colour = line.next().map(|pair| pair.as_str().to_string());

    Some(Line { start, end, colour })
}

fn parse_geo(pair: Pair<Rule>) -> Geo {
    let mut geo = pair.into_inner();
    let name = geo.next().unwrap().as_str().to_string();
    let lines = geo.filter_map(parse_line).collect();

    Geo { name, lines }
}

fn parse_sid(pair: Pair<Rule>) -> Sid {
    let mut sid = pair.into_inner();
    let name = sid.next().unwrap().as_str().to_string();
    let lines = sid.filter_map(parse_line).collect();

    Sid { name, lines }
}

fn parse_star(pair: Pair<Rule>) -> Star {
    let mut star = pair.into_inner();
    let name = star.next().unwrap().as_str().to_string();
    let lines = star.filter_map(parse_line).collect();

    Star { name, lines }
}

fn parse_artcc(pair: Pair<Rule>) -> Artcc {
    let mut artcc = pair.into_inner();
    let name = artcc.next().unwrap().as_str().to_string();
    let lines = artcc.filter_map(parse_line).collect();

    Artcc { name, lines }
}

fn parse_vor(pair: Pair<Rule>) -> VOR {
    let mut location = pair.into_inner();
    let designator = location.next().unwrap().as_str().to_string();
    let frequency = location.next().unwrap().as_str().to_string();
    let coordinate = parse_coordinate(location.next().unwrap());

    VOR {
        designator,
        frequency,
        coordinate,
    }
}

fn parse_runway(pair: Pair<Rule>) -> Runway {
    let mut runway = pair.into_inner();
    let designator1 = runway.next().unwrap().as_str().to_string();
    let designator2 = runway.next().unwrap().as_str().to_string();
    let heading1 = runway.next().unwrap().as_str().parse().unwrap();
    let heading2 = runway.next().unwrap().as_str().parse().unwrap();
    let loc1 = parse_coordinate(runway.next().unwrap());
    let loc2 = parse_coordinate(runway.next().unwrap());
    let aerodrome = runway.next().unwrap().as_str().to_string();

    Runway {
        designators: (designator1, designator2),
        headings: (heading1, heading2),
        location: (loc1, loc2),
        aerodrome,
    }
}

fn parse_info_section(pair: Pair<Rule>, colours: &mut HashMap<String, Colour>) -> SctInfo {
    let mut sct_info = SctInfo::default();
    let mut i = 0;
    let mut y = DegMinSec::default();

    for pair in pair.into_inner() {
        if let Rule::colour_definition = pair.as_rule() {
            store_colour(colours, pair);
        } else {
            match i {
                0 => sct_info.name = pair.as_str().to_string(),
                1 => sct_info.default_callsign = pair.as_str().to_string(),
                2 => sct_info.default_airport = pair.as_str().to_string(),
                3 => y = parse_coordinate_part(pair),
                4 => {
                    let x = parse_coordinate_part(pair);
                    sct_info.centre_point = Coord::from_deg_min_sec(y, x);
                }
                5 => sct_info.miles_per_deg_lat = pair.as_str().parse().unwrap(),
                6 => sct_info.miles_per_deg_lng = pair.as_str().parse().unwrap(),
                7 => sct_info.magnetic_variation = pair.as_str().parse().unwrap(),
                8 => sct_info.scale_factor = pair.as_str().parse().unwrap(),
                _ => unreachable!("Too many .sct info lines"),
            }
            i += 1;
        }
    }
    sct_info
}

fn parse_colour_definition(pair: Pair<Rule>) -> Option<(String, Colour)> {
    let mut pairs = pair.into_inner();
    let colour_name = pairs.next().unwrap().as_str().to_string();
    match Colour::from_euroscope(pairs.next().unwrap().as_str().parse().unwrap()) {
        Ok(colour_value) => Some((colour_name, colour_value)),
        Err(e) => {
            warn!("Could not parse colour {colour_name}: {e}");
            None
        }
    }
}

#[inline]
fn store_colour(colours: &mut HashMap<String, Colour>, pair: Pair<Rule>) {
    if let Some((colour_name, colour)) = parse_colour_definition(pair) {
        colours.insert(colour_name, colour);
    }
}

fn parse_independent_section(
    pair: Pair<Rule>,
    colours: &mut HashMap<String, Colour>,
) -> (SectionName, Section) {
    match pair.as_rule() {
        Rule::info_section => (
            SectionName::Info,
            Section::Info(parse_info_section(pair, colours)),
        ),
        Rule::airport_section => (
            SectionName::Airport,
            Section::Airport(
                pair.into_inner()
                    .filter_map(|pair| {
                        if let Rule::colour_definition = pair.as_rule() {
                            store_colour(colours, pair);
                            None
                        } else {
                            Some(parse_airport(pair))
                        }
                    })
                    .collect(),
            ),
        ),

        Rule::fixes_section => (
            SectionName::Fixes,
            Section::Fixes(
                pair.into_inner()
                    .filter_map(|pair| {
                        if let Rule::colour_definition = pair.as_rule() {
                            store_colour(colours, pair);
                            None
                        } else {
                            parse_fix(pair)
                        }
                    })
                    .collect(),
            ),
        ),

        Rule::ndb_section => (
            SectionName::NDBs,
            Section::NDBs(
                pair.into_inner()
                    .filter_map(|pair| {
                        if let Rule::colour_definition = pair.as_rule() {
                            store_colour(colours, pair);
                            None
                        } else {
                            parse_ndb(pair)
                        }
                    })
                    .collect(),
            ),
        ),

        Rule::vor_section => (
            SectionName::VORs,
            Section::VORs(
                pair.into_inner()
                    .filter_map(|pair| {
                        if let Rule::colour_definition = pair.as_rule() {
                            store_colour(colours, pair);
                            None
                        } else {
                            Some(parse_vor(pair))
                        }
                    })
                    .collect(),
            ),
        ),
        Rule::runway_section => (
            SectionName::Runways,
            Section::Runways(
                pair.into_inner()
                    .filter_map(|pair| {
                        if let Rule::colour_definition = pair.as_rule() {
                            store_colour(colours, pair);
                            None
                        } else {
                            Some(parse_runway(pair))
                        }
                    })
                    .collect(),
            ),
        ),
        Rule::sid_section => (
            SectionName::Sid,
            Section::Sid(
                pair.into_inner()
                    .filter_map(|pair| {
                        if let Rule::colour_definition = pair.as_rule() {
                            store_colour(colours, pair);
                            None
                        } else {
                            Some(parse_sid(pair))
                        }
                    })
                    .collect(),
            ),
        ),
        Rule::star_section => (
            SectionName::Star,
            Section::Star(
                pair.into_inner()
                    .filter_map(|pair| {
                        if let Rule::colour_definition = pair.as_rule() {
                            store_colour(colours, pair);
                            None
                        } else {
                            Some(parse_star(pair))
                        }
                    })
                    .collect(),
            ),
        ),
        Rule::artcc_high_section => (
            SectionName::ArtccHigh,
            Section::ArtccHigh(
                pair.into_inner()
                    .filter_map(|pair| {
                        if let Rule::colour_definition = pair.as_rule() {
                            store_colour(colours, pair);
                            None
                        } else {
                            Some(parse_artcc(pair))
                        }
                    })
                    .collect(),
            ),
        ),
        Rule::artcc_section => (
            SectionName::Artcc,
            Section::Artcc(
                pair.into_inner()
                    .filter_map(|pair| {
                        if let Rule::colour_definition = pair.as_rule() {
                            store_colour(colours, pair);
                            None
                        } else {
                            Some(parse_artcc(pair))
                        }
                    })
                    .collect(),
            ),
        ),
        Rule::artcc_low_section => (
            SectionName::ArtccLow,
            Section::ArtccLow(
                pair.into_inner()
                    .filter_map(|pair| {
                        if let Rule::colour_definition = pair.as_rule() {
                            store_colour(colours, pair);
                            None
                        } else {
                            Some(parse_artcc(pair))
                        }
                    })
                    .collect(),
            ),
        ),
        Rule::high_airway_section => (
            SectionName::HighAirway,
            Section::HighAirways(
                pair.into_inner()
                    .filter_map(|pair| {
                        if let Rule::colour_definition = pair.as_rule() {
                            store_colour(colours, pair);
                            None
                        } else {
                            parse_airway(pair)
                        }
                    })
                    .collect(),
            ),
        ),

        Rule::low_airway_section => (
            SectionName::LowAirway,
            Section::LowAirways(
                pair.into_inner()
                    .filter_map(|pair| {
                        if let Rule::colour_definition = pair.as_rule() {
                            store_colour(colours, pair);
                            None
                        } else {
                            parse_airway(pair)
                        }
                    })
                    .collect(),
            ),
        ),

        Rule::region_section => (
            SectionName::Regions,
            Section::Regions(
                pair.into_inner()
                    .filter_map(|pair| {
                        if let Rule::colour_definition = pair.as_rule() {
                            store_colour(colours, pair);
                            None
                        } else {
                            Some(parse_region(pair))
                        }
                    })
                    .collect(),
            ),
        ),

        Rule::geo_section => (
            SectionName::Geo,
            Section::Geo(
                pair.into_inner()
                    .filter_map(|pair| {
                        if let Rule::colour_definition = pair.as_rule() {
                            store_colour(colours, pair);
                            None
                        } else {
                            Some(parse_geo(pair))
                        }
                    })
                    .collect(),
            ),
        ),

        _ => (SectionName::Unsupported, Section::Unsupported),
    }
}

impl Sct {
    pub fn parse(content: &[u8]) -> SctResult {
        let unparsed_file = read_to_string(content)?;
        let mut colours = HashMap::new();
        let sct_parse = SctParser::parse(Rule::sct, &unparsed_file);
        let mut sections = sct_parse.map(|mut pairs| {
            pairs
                .next()
                .unwrap()
                .into_inner()
                .filter_map(|pair| {
                    if let Rule::colour_definition = pair.as_rule() {
                        store_colour(&mut colours, pair);
                        None
                    } else {
                        Some(parse_independent_section(pair, &mut colours))
                    }
                })
                .collect::<HashMap<_, _>>()
        })?;

        let info = match sections.remove_entry(&SectionName::Info) {
            Some((_, Section::Info(sct_info))) => sct_info,
            _ => SctInfo::default(),
        };
        let airports = match sections.remove_entry(&SectionName::Airport) {
            Some((_, Section::Airport(airports))) => airports,
            _ => vec![],
        };
        let fixes = match sections.remove_entry(&SectionName::Fixes) {
            Some((_, Section::Fixes(fixes))) => fixes,
            _ => vec![],
        };
        let ndbs = match sections.remove_entry(&SectionName::NDBs) {
            Some((_, Section::NDBs(ndbs))) => ndbs,
            _ => vec![],
        };
        let vors = match sections.remove_entry(&SectionName::VORs) {
            Some((_, Section::VORs(vors))) => vors,
            _ => vec![],
        };
        let runways = match sections.remove_entry(&SectionName::Runways) {
            Some((_, Section::Runways(runways))) => runways,
            _ => vec![],
        };
        let sids = match sections.remove_entry(&SectionName::Sid) {
            Some((_, Section::Sid(sids))) => sids,
            _ => vec![],
        };
        let stars = match sections.remove_entry(&SectionName::Star) {
            Some((_, Section::Star(stars))) => stars,
            _ => vec![],
        };
        let artccs_high = match sections.remove_entry(&SectionName::ArtccHigh) {
            Some((_, Section::ArtccHigh(artccs))) => artccs,
            _ => vec![],
        };
        let artccs = match sections.remove_entry(&SectionName::Artcc) {
            Some((_, Section::Artcc(artccs))) => artccs,
            _ => vec![],
        };
        let artccs_low = match sections.remove_entry(&SectionName::ArtccLow) {
            Some((_, Section::ArtccLow(artccs))) => artccs,
            _ => vec![],
        };
        let high_airways = match sections.remove_entry(&SectionName::HighAirway) {
            Some((_, Section::HighAirways(high_airways))) => high_airways,
            _ => vec![],
        };
        let low_airways = match sections.remove_entry(&SectionName::LowAirway) {
            Some((_, Section::LowAirways(low_airways))) => low_airways,
            _ => vec![],
        };
        let regions = match sections.remove_entry(&SectionName::Regions) {
            Some((_, Section::Regions(regions))) => regions,
            _ => vec![],
        };
        let geo = match sections.remove_entry(&SectionName::Geo) {
            Some((_, Section::Geo(geo))) => geo,
            _ => vec![],
        };

        let sct = Sct {
            info,
            airports,
            colours: colours.clone(),
            fixes,
            ndbs,
            vors,
            runways,
            sids,
            stars,
            artccs_high,
            artccs,
            artccs_low,
            high_airways,
            low_airways,
            regions,
            geo,
        };

        Ok(sct)
    }
}
impl Display for Sct {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.info)?;
        writeln!(
            f,
            "{}\n",
            self.colours
                .iter()
                .map(|(name, c)| format!("#define {name}\t{}", c.to_euroscope()))
                .sorted()
                .join("\n")
        )?;
        writeln!(f, "{}", self.vors.to_euroscope())?;
        writeln!(f, "{}", self.ndbs.to_euroscope())?;
        writeln!(f, "{}", self.fixes.to_euroscope())?;
        writeln!(f, "{}", self.airports.to_euroscope())?;
        writeln!(f, "{}", self.runways.to_euroscope())?;
        writeln!(f, "{}", self.sids.to_euroscope())?;
        writeln!(f, "{}", self.stars.to_euroscope())?;
        writeln!(f, "[ARTCC HIGH]\n{}", self.artccs_high.to_euroscope())?;
        writeln!(f, "[ARTCC]\n{}", self.artccs.to_euroscope())?;
        writeln!(f, "[ARTCC LOW]\n{}", self.artccs_low.to_euroscope())?;
        writeln!(f, "{}", self.geo.to_euroscope())?;
        writeln!(f, "{}", self.regions.to_euroscope())?;
        writeln!(f, "[HIGH AIRWAY]\n{}", self.high_airways.to_euroscope())?;
        write!(f, "[LOW AIRWAY]\n{}", self.low_airways.to_euroscope())
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use geo::Coord;
    use pretty_assertions_sorted::assert_eq_sorted;

    use crate::{
        adaptation::{
            colours::Colour,
            locations::{Fix, NDB, VOR},
        },
        sct::{Airport, Airway, Artcc, Geo, Line, Region, Runway, Sct, SctInfo, Sid, Star},
        Location,
    };

    #[test]
    fn test_sct() {
        let sct_bytes = b"
;=========================================================================================================================;

[INFO]
AeroNav M\xfcnchen 2401/1-1 EDMM 20240125
AERO_NAV
ZZZZ
N048.21.13.618
E011.47.09.909
60
39
-3
1

#define colour_APP       16711680
#define colour_AirspaceA  8421376
#define prohibitcolour 7697781		; 117,117,117	Prohibited areas

[VOR]
NUB  115.750 N049.30.10.508 E011.02.06.000 ; NUB Comment Test
OTT  112.300 N048.10.49.418 E011.48.59.529

[NDB]
MIQ  426.000 N048.34.12.810 E011.35.51.010 ;MIQ Comment Test
RTT  303.000 N047.25.51.319 E011.56.24.190

[FIXES]
(FM-C) N049.31.05.999 E008.26.42.000
ARMUT N049.43.20.999 E012.19.23.998
GEDSO N047.04.50.001 E011.52.13.000
INBED N049.23.15.000 E010.56.30.001
NAXAV N046.27.49.881 E011.19.19.858
UNKUL N049.08.13.999 E011.27.34.999
VEMUT N049.48.38.678 E012.27.40.489

[AIRPORT]
EDDM 000.000 N048.21.13.618 E011.47.09.909 D
EDNX 000.000 N048.14.20.399 E011.33.33.001 D
LIPB 000.000 N046.27.37.000 E011.19.35.000 D

[RUNWAY]
08R 26L 080 260 N048.20.26.408 E011.45.03.661 N048.20.41.269 E011.48.16.610 EDDM
08L 26R 080 260 N048.21.45.961 E011.46.03.179 N048.22.00.789 E011.49.16.219 EDDM
07  25  071 251 N048.14.17.710 E011.33.14.090 N048.14.24.388 E011.33.51.998 EDNX
04 22 037 217 N054.36.43.340 W005.52.47.140 N054.37.29.480 W005.51.51.950 EGAC Belfast/City

[SID]
EDDM SID 26L BIBAGxS                     N048.20.25.315 E011.44.49.465 N048.20.15.608 E011.42.33.951
                                         N048.20.15.608 E011.42.33.951 N048.17.15.248 E011.42.28.821
                                         N048.17.15.248 E011.42.28.821 N048.10.49.418 E011.48.59.529
                                         N048.10.49.418 E011.48.59.529 N048.13.54.868 E012.33.55.821
                                         N048.13.54.868 E012.33.55.821 N048.23.49.380 E012.44.59.211

EDDN SID 28 BOLSIxG                      N049.30.02.914 E011.03.23.507 N049.30.47.941 E010.55.42.060
                                         N049.30.47.941 E010.55.42.060 N049.27.44.369 E010.45.45.219
                                         N049.27.44.369 E010.45.45.219 N049.13.54.559 E010.45.30.628

[STAR]
EDDM TRAN RNP26R LANDU26                 N048.35.46.899 E012.16.26.140 N048.32.16.501 E011.30.05.529 colour_APP
                                         N048.32.16.501 E011.30.05.529 N048.25.44.061 E011.31.15.931 colour_APP
                                         N048.25.44.061 E011.31.15.931 N048.29.36.319 E012.22.42.409 colour_APP
                                         N048.29.36.319 E012.22.42.409 N048.30.47.361 E012.37.38.952 colour_APP

EDDN TRAN ILS10 DN430                    N049.33.11.289 E010.30.33.379 N049.32.37.168 E010.36.38.149 colour_APP
                                         N049.32.37.168 E010.36.38.149 N049.32.02.731 E010.42.42.681 colour_APP

EDQD STAR ALL LONLIxZ                    N050.04.29.060 E011.13.34.989 N049.53.50.819 E011.24.28.540


[ARTCC HIGH]
EDJA_ILR_APP                             N048.10.28.000 E009.34.15.000 N048.18.09.000 E009.55.25.000
                                         N048.10.28.000 E009.34.15.000 N048.10.00.000 E009.33.00.000
                                         N047.58.59.000 E010.50.38.000 N048.14.12.000 E010.25.42.000
                                         N048.14.12.000 E010.25.42.000 N048.18.09.000 E009.55.25.000
                                         N047.58.59.000 E010.50.38.000 N047.52.03.000 E010.28.00.000
                                         N047.52.03.000 E010.28.00.000 N047.50.30.000 E010.23.00.000
                                         N047.50.30.000 E010.23.00.000 N047.49.05.000 E009.58.21.000
                                         N047.53.53.000 E009.50.08.000 N047.48.55.000 E009.54.18.000
                                         N047.48.55.000 E009.54.18.000 N047.49.05.000 E009.58.21.000
                                         N047.53.53.000 E009.50.08.000 N047.53.24.000 E009.33.00.000
                                         N047.53.24.000 E009.33.00.000 N047.58.24.000 E009.33.00.000
                                         N047.58.24.000 E009.33.00.000 N048.10.00.000 E009.33.00.000

Release line EDMM ARBAX Window           N049.26.33.000 E012.52.22.000 N049.35.18.000 E013.00.24.000 colour_Releaseline
                                         N049.35.18.000 E013.00.24.000 N049.27.45.000 E013.17.13.000 colour_Releaseline
                                         N049.27.45.000 E013.17.13.000 N049.12.42.000 E013.13.14.000 colour_Releaseline
[ARTCC]
EDMM_ALB_CTR                             N049.08.17.000 E011.07.57.000 N049.10.00.000 E011.58.00.000
                                         N048.40.03.000 E011.47.39.000 N049.10.00.000 E011.58.00.000
                                         N048.40.03.000 E011.47.39.000 N048.40.04.000 E011.30.42.000
                                         N048.40.04.000 E011.30.42.000 N048.40.04.000 E011.19.15.000
                                         N048.40.04.000 E011.19.15.000 N049.07.10.000 E010.40.25.000
                                         N049.07.10.000 E010.40.25.000 N049.08.17.000 E011.07.57.000
                                         N049.07.10.000 E010.40.25.000 N049.10.43.000 E010.35.16.000
                                         N049.26.04.000 E011.49.08.000 N049.26.30.000 E010.58.00.000
                                         N049.10.43.000 E010.35.16.000 N049.26.30.000 E010.58.00.000
                                         N049.26.30.000 E010.58.00.000 N049.39.17.000 E010.45.56.000
                                         N049.10.43.000 E010.35.16.000 N049.17.00.000 E010.27.30.000
                                         N049.17.00.000 E010.27.30.000 N049.23.54.000 E010.21.22.000
                                         N049.23.54.000 E010.21.22.000 N049.30.40.000 E010.30.12.000
                                         N049.30.40.000 E010.30.12.000 N049.39.17.000 E010.45.56.000
                                         N049.10.00.000 E011.58.00.000 N049.26.04.000 E011.49.08.000

EDMM_WLD_CTR                             N048.40.00.000 E010.58.00.000 N048.40.10.000 E011.11.00.000
                                         N048.40.10.000 E011.11.00.000 N048.40.04.000 E011.19.15.000
                                         N049.01.02.000 E010.15.27.000 N049.06.40.000 E010.28.45.000
                                         N049.06.40.000 E010.28.45.000 N049.10.43.000 E010.35.16.000
                                         N048.40.04.000 E011.19.15.000 N049.07.10.000 E010.40.25.000
                                         N049.07.10.000 E010.40.25.000 N049.10.43.000 E010.35.16.000
                                         N048.37.28.000 E010.54.16.000 N049.01.02.000 E010.15.27.000
                                         N048.37.28.000 E010.54.16.000 N048.40.00.000 E010.58.00.000


[ARTCC LOW]
RMZ EDMS                                 N048.57.40.000 E012.23.34.000 N048.56.22.000 E012.39.38.000 colour_RMZ
                                         N048.56.22.000 E012.39.38.000 N048.50.25.000 E012.38.30.000 colour_RMZ
                                         N048.50.25.000 E012.38.30.000 N048.51.43.000 E012.22.28.000 colour_RMZ
                                         N048.51.43.000 E012.22.28.000 N048.57.40.000 E012.23.34.000 colour_RMZ

RMZ EDNX                                 N048.16.27.000 E011.27.21.000 N048.17.12.000 E011.32.05.000 colour_RMZ
                                         N048.17.12.000 E011.32.05.000 N048.16.25.000 E011.32.09.000 colour_RMZ
                                         N048.16.25.000 E011.32.09.000 N048.16.54.000 E011.38.27.000 colour_RMZ
                                         N048.16.54.000 E011.38.27.000 N048.15.17.000 E011.40.25.000 colour_RMZ
                                         N048.15.17.000 E011.40.25.000 N048.14.28.000 E011.40.40.000 colour_RMZ
                                         N048.14.28.000 E011.40.40.000 N048.12.14.000 E011.39.45.000 colour_RMZ
                                         N048.12.14.000 E011.39.45.000 N048.10.38.000 E011.29.22.000 colour_RMZ
                                         N048.10.38.000 E011.29.22.000 N048.16.27.000 E011.27.21.000 colour_RMZ


[GEO]

EDDN Groundlayout Holding Points         N049.29.58.736 E011.03.33.028 N049.29.58.942 E011.03.34.353 colour_Stopbar
                                         N049.29.56.786 E011.03.52.659 N049.29.57.006 E011.03.54.022 colour_Stopbar

EDQC Groundlayout                        N050.15.52.011 E010.59.29.553 N050.15.52.239 E010.59.29.566

EDQC Groundlayout                        N050.15.52.011 E010.59.29.553 N050.15.52.239 E010.59.29.566 colour_Stopbar
                                         N050.15.39.345 E011.00.04.860 N050.15.39.411 E011.00.05.139 colour_Stopbar
                                         N050.15.39.345 E011.00.04.860 N050.15.39.411 E011.00.05.139
                                         RAVSA RAVSA LCE13 LCE13 geoDefault
HIGHWAYS LOVV                            N048.12.09.100 E016.13.16.700 N048.11.27.100 E016.12.10.100 COLOR_Landmark2
                                         N050.15.39.345 E011.00.04.860 N050.15.39.411 E011.00.05.139 colour_Stopbar
[REGIONS]
REGIONNAME EDDM Groundlayout
colour_Stopbar              N048.21.53.621 E011.48.32.152
                           N048.21.54.514 E011.48.32.715
                           N048.21.54.527 E011.48.32.571
                           N048.21.53.676 E011.48.32.042
REGIONNAME EDMO Groundlayout
colour_HardSurface1         N048.04.25.470 E011.16.20.093
                           N048.04.24.509 E011.16.21.717
                           N048.05.20.677 E011.17.38.446
                           N048.05.21.638 E011.17.36.829
Surrounding Grass	grass	N51.11.37.165 E02.51.07.153
 N51.11.38.619 E02.51.08.463
 N51.11.39.660 E02.51.10.008

[HIGH AIRWAY]
B73        N054.54.46.000 E018.57.29.998 N055.36.12.999 E019.50.17.901
B74        N055.12.05.000 E019.38.03.001 N055.36.12.999 E019.50.17.901
B74        N054.38.16.000 E019.21.20.001 N055.12.05.000 E019.38.03.001
B75        ARMUT ARMUT VEMUT VEMUT
B76        ARMUT ARMUT RTT RTT

[LOW AIRWAY]
A361       N048.56.21.001 E000.57.11.001 N048.47.26.199 E000.31.49.000
A361       N049.01.42.700 E001.12.50.601 N048.56.21.001 E000.57.11.001
A4         N048.37.03.248 E017.32.28.201 N048.42.56.998 E017.23.09.999
A4         N048.17.25.569 E018.03.02.300 N048.37.03.248 E017.32.28.201
A4         N048.42.56.998 E017.23.09.999 N048.51.11.520 E017.10.04.238
A5         RTT RTT NUB NUB

        ";
        let sct = Sct::parse(sct_bytes);
        assert_eq!(
            sct.as_ref().unwrap().info,
            SctInfo {
                name: "AeroNav München 2401/1-1 EDMM 20240125".to_string(),
                default_callsign: "AERO_NAV".to_string(),
                default_airport: "ZZZZ".to_string(),
                centre_point: Coord {
                    y: 48.353_782_777_777_78,
                    x: 11.786_085_833_333_333
                },
                miles_per_deg_lat: 60.0,
                miles_per_deg_lng: 39.0,
                magnetic_variation: -3.0,
                scale_factor: 1.0,
            }
        );
        assert_eq!(
            sct.as_ref().unwrap().colours,
            HashMap::from([
                (
                    "colour_APP".to_string(),
                    Colour {
                        r: 0,
                        g: 0,
                        b: 255,
                        a: 255
                    }
                ),
                (
                    "colour_AirspaceA".to_string(),
                    Colour {
                        r: 0,
                        g: 128,
                        b: 128,
                        a: 255
                    }
                ),
                (
                    "prohibitcolour".to_string(),
                    Colour {
                        r: 117,
                        g: 117,
                        b: 117,
                        a: 255
                    }
                )
            ])
        );
        assert_eq!(
            sct.as_ref().unwrap().ndbs,
            vec![
                NDB {
                    designator: "MIQ".to_string(),
                    frequency: "426.000".to_string(),
                    coordinate: Coord {
                        y: 48.570_225,
                        x: 11.597_502_777_777_779,
                    }
                },
                NDB {
                    designator: "RTT".to_string(),
                    frequency: "303.000".to_string(),
                    coordinate: Coord {
                        y: 47.430_921_944_444_44,
                        x: 11.940_052_777_777_778
                    }
                }
            ]
        );
        assert_eq!(
            sct.as_ref().unwrap().vors,
            vec![
                VOR {
                    designator: "NUB".to_string(),
                    frequency: "115.750".to_string(),
                    coordinate: Coord {
                        y: 49.502_918_888_888_885,
                        x: 11.035
                    }
                },
                VOR {
                    designator: "OTT".to_string(),
                    frequency: "112.300".to_string(),
                    coordinate: Coord {
                        y: 48.180_393_888_888_89,
                        x: 11.816_535_833_333_335
                    }
                }
            ]
        );
        assert_eq!(
            sct.as_ref().unwrap().airports,
            vec![
                Airport {
                    designator: "EDDM".to_string(),
                    coordinate: Coord {
                        y: 48.353_782_777_777_78,
                        x: 11.786_085_833_333_333
                    },
                    ctr_airspace: "D".to_string()
                },
                Airport {
                    designator: "EDNX".to_string(),
                    coordinate: Coord {
                        y: 48.238_999_722_222_225,
                        x: 11.559_166_944_444_446
                    },
                    ctr_airspace: "D".to_string()
                },
                Airport {
                    designator: "LIPB".to_string(),
                    coordinate: Coord {
                        y: 46.460_277_777_777_78,
                        x: 11.326_388_888_888_89
                    },
                    ctr_airspace: "D".to_string()
                }
            ]
        );
        assert_eq!(
            sct.as_ref().unwrap().fixes,
            vec![
                Fix {
                    designator: "(FM-C)".to_string(),
                    coordinate: Coord {
                        y: 49.518_333_055_555_55,
                        x: 8.445
                    }
                },
                Fix {
                    designator: "ARMUT".to_string(),
                    coordinate: Coord {
                        y: 49.722_499_722_222_224,
                        x: 12.323_332_777_777_777
                    }
                },
                Fix {
                    designator: "GEDSO".to_string(),
                    coordinate: Coord {
                        y: 47.080_555_833_333_335,
                        x: 11.870_277_777_777_778
                    }
                },
                Fix {
                    designator: "INBED".to_string(),
                    coordinate: Coord {
                        y: 49.3875,
                        x: 10.941_666_944_444_444
                    }
                },
                Fix {
                    designator: "NAXAV".to_string(),
                    coordinate: Coord {
                        y: 46.463_855_833_333_334,
                        x: 11.322_182_777_777_778
                    }
                },
                Fix {
                    designator: "UNKUL".to_string(),
                    coordinate: Coord {
                        y: 49.137_221_944_444_44,
                        x: 11.459_721_944_444_444
                    }
                },
                Fix {
                    designator: "VEMUT".to_string(),
                    coordinate: Coord {
                        y: 49.810_743_888_888_89,
                        x: 12.461_246_944_444_444
                    }
                }
            ]
        );
        assert_eq!(
            sct.as_ref().unwrap().runways,
            vec![
                Runway {
                    designators: ("08R".to_string(), "26L".to_string()),
                    headings: (80, 260),
                    location: (
                        Coord {
                            y: 48.340_668_888_888_89,
                            x: 11.751_016_944_444_444
                        },
                        Coord {
                            y: 48.344_796_944_444_45,
                            x: 11.804_613_888_888_89
                        }
                    ),
                    aerodrome: "EDDM".to_string()
                },
                Runway {
                    designators: ("08L".to_string(), "26R".to_string()),
                    headings: (80, 260),
                    location: (
                        Coord {
                            y: 48.362_766_944_444_445,
                            x: 11.767_549_722_222_222
                        },
                        Coord {
                            y: 48.366_885_833_333_335,
                            x: 11.821_171_944_444_444
                        }
                    ),
                    aerodrome: "EDDM".to_string()
                },
                Runway {
                    designators: ("07".to_string(), "25".to_string()),
                    headings: (71, 251),
                    location: (
                        Coord {
                            y: 48.238_252_777_777_78,
                            x: 11.553_913_888_888_89
                        },
                        Coord {
                            y: 48.240_107_777_777_78,
                            x: 11.564_443_888_888_89
                        }
                    ),
                    aerodrome: "EDNX".to_string()
                },
                Runway {
                    designators: ("04".to_string(), "22".to_string()),
                    headings: (37, 217),
                    location: (
                        Coord {
                            y: 54.612_038_888_888_89,
                            x: -5.879_761_111_111_112
                        },
                        Coord {
                            y: 54.624_855_555_555_555,
                            x: -5.864_430_555_555_555
                        }
                    ),
                    aerodrome: "EGAC".to_string()
                }
            ]
        );
        assert_eq!(
            sct.as_ref().unwrap().sids,
            vec![
                Sid {
                    name: "EDDM SID 26L BIBAGxS".to_string(),
                    lines: vec![
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 11.747_073_611_111_11,
                                y: 48.340_365_277_777_78
                            }),
                            end: Location::Coordinate(Coord {
                                x: 11.709_430_833_333_332,
                                y: 48.337_668_888_888_89
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 11.709_430_833_333_332,
                                y: 48.337_668_888_888_89
                            }),
                            end: Location::Coordinate(Coord {
                                x: 11.708_005_833_333_333,
                                y: 48.287_568_888_888_885
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 11.708_005_833_333_333,
                                y: 48.287_568_888_888_885
                            }),
                            end: Location::Coordinate(Coord {
                                x: 11.816_535_833_333_335,
                                y: 48.180_393_888_888_89
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 11.816_535_833_333_335,
                                y: 48.180_393_888_888_89
                            }),
                            end: Location::Coordinate(Coord {
                                x: 12.565_505_833_333_335,
                                y: 48.231_907_777_777_78
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 12.565_505_833_333_335,
                                y: 48.231_907_777_777_78
                            }),
                            end: Location::Coordinate(Coord {
                                x: 12.749_780_833_333_332,
                                y: 48.39705
                            }),
                            colour: None
                        }
                    ]
                },
                Sid {
                    name: "EDDN SID 28 BOLSIxG".to_string(),
                    lines: vec![
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 11.056_529_722_222_223,
                                y: 49.500_809_444_444_44
                            }),
                            end: Location::Coordinate(Coord {
                                x: 10.92835,
                                y: 49.513_316_944_444_45
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 10.92835,
                                y: 49.513_316_944_444_45
                            }),
                            end: Location::Coordinate(Coord {
                                x: 10.762_560_833_333_334,
                                y: 49.462_324_722_222_23
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 10.762_560_833_333_334,
                                y: 49.462_324_722_222_23
                            }),
                            end: Location::Coordinate(Coord {
                                x: 10.758_507_777_777_778,
                                y: 49.231_821_944_444_45
                            }),
                            colour: None
                        }
                    ]
                }
            ]
        );
        assert_eq!(
            sct.as_ref().unwrap().stars,
            vec![
                Star {
                    name: "EDDM TRAN RNP26R LANDU26".to_string(),
                    lines: vec![
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 12.273_927_777_777_779,
                                y: 48.596_360_833_333_335
                            }),
                            end: Location::Coordinate(Coord {
                                x: 11.501_535_833_333_334,
                                y: 48.537_916_944_444_44
                            }),
                            colour: Some("colour_APP".to_string())
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 11.501_535_833_333_334,
                                y: 48.537_916_944_444_44
                            }),
                            end: Location::Coordinate(Coord {
                                x: 11.521_091_944_444_445,
                                y: 48.428_905_833_333_33
                            }),
                            colour: Some("colour_APP".to_string())
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 11.521_091_944_444_445,
                                y: 48.428_905_833_333_33
                            }),
                            end: Location::Coordinate(Coord {
                                x: 12.378_446_944_444_445,
                                y: 48.493_421_944_444_44
                            }),
                            colour: Some("colour_APP".to_string())
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 12.378_446_944_444_445,
                                y: 48.493_421_944_444_44
                            }),
                            end: Location::Coordinate(Coord {
                                x: 12.627_486_666_666_668,
                                y: 48.513_155_833_333_336
                            }),
                            colour: Some("colour_APP".to_string())
                        }
                    ]
                },
                Star {
                    name: "EDDN TRAN ILS10 DN430".to_string(),
                    lines: vec![
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 10.509_271_944_444_444,
                                y: 49.553_135_833_333_33
                            }),
                            end: Location::Coordinate(Coord {
                                x: 10.610_596_944_444_444,
                                y: 49.543_657_777_777_774
                            }),
                            colour: Some("colour_APP".to_string())
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 10.610_596_944_444_444,
                                y: 49.543_657_777_777_774
                            }),
                            end: Location::Coordinate(Coord {
                                x: 10.711_855_833_333_333,
                                y: 49.534_091_944_444_44
                            }),
                            colour: Some("colour_APP".to_string())
                        }
                    ]
                },
                Star {
                    name: "EDQD STAR ALL LONLIxZ".to_string(),
                    lines: vec![Line {
                        start: Location::Coordinate(Coord {
                            x: 11.226_385_833_333_334,
                            y: 50.074_738_888_888_895
                        }),
                        end: Location::Coordinate(Coord {
                            x: 11.407_927_777_777_779,
                            y: 49.897_449_722_222_22
                        }),
                        colour: None
                    }]
                }
            ]
        );
        assert_eq!(
            sct.as_ref().unwrap().artccs_high,
            vec![
                Artcc {
                    name: "EDJA_ILR_APP".to_string(),
                    lines: vec![
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 9.570_833_333_333_333,
                                y: 48.174_444_444_444_44
                            }),
                            end: Location::Coordinate(Coord {
                                x: 9.923_611_111_111_11,
                                y: 48.302_499_999_999_995
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 9.570_833_333_333_333,
                                y: 48.174_444_444_444_44
                            }),
                            end: Location::Coordinate(Coord {
                                x: 9.55,
                                y: 48.166_666_666_666_664
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 10.843_888_888_888_89,
                                y: 47.983_055_555_555_56
                            }),
                            end: Location::Coordinate(Coord {
                                x: 10.428_333_333_333_333,
                                y: 48.236_666_666_666_665
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 10.428_333_333_333_333,
                                y: 48.236_666_666_666_665
                            }),
                            end: Location::Coordinate(Coord {
                                x: 9.923_611_111_111_11,
                                y: 48.302_499_999_999_995
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 10.843_888_888_888_89,
                                y: 47.983_055_555_555_56
                            }),
                            end: Location::Coordinate(Coord {
                                x: 10.466_666_666_666_667,
                                y: 47.8675
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 10.466_666_666_666_667,
                                y: 47.8675
                            }),
                            end: Location::Coordinate(Coord {
                                x: 10.383_333_333_333_333,
                                y: 47.841_666_666_666_67
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 10.383_333_333_333_333,
                                y: 47.841_666_666_666_67
                            }),
                            end: Location::Coordinate(Coord {
                                x: 9.9725,
                                y: 47.818_055_555_555_56
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 9.835_555_555_555_556,
                                y: 47.898_055_555_555_56
                            }),
                            end: Location::Coordinate(Coord {
                                x: 9.905_000_000_000_001,
                                y: 47.815_277_777_777_77
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 9.905_000_000_000_001,
                                y: 47.815_277_777_777_77
                            }),
                            end: Location::Coordinate(Coord {
                                x: 9.9725,
                                y: 47.818_055_555_555_56
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 9.835_555_555_555_556,
                                y: 47.898_055_555_555_56
                            }),
                            end: Location::Coordinate(Coord { x: 9.55, y: 47.89 }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord { x: 9.55, y: 47.89 }),
                            end: Location::Coordinate(Coord {
                                x: 9.55,
                                y: 47.973_333_333_333_336
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 9.55,
                                y: 47.973_333_333_333_336
                            }),
                            end: Location::Coordinate(Coord {
                                x: 9.55,
                                y: 48.166_666_666_666_664
                            }),
                            colour: None
                        }
                    ]
                },
                Artcc {
                    name: "Release line EDMM ARBAX Window".to_string(),
                    lines: vec![
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 12.872_777_777_777_777,
                                y: 49.442_499_999_999_995
                            }),
                            end: Location::Coordinate(Coord {
                                x: 13.006_666_666_666_666,
                                y: 49.588_333_333_333_34
                            }),
                            colour: Some("colour_Releaseline".to_string())
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 13.006_666_666_666_666,
                                y: 49.588_333_333_333_34
                            }),
                            end: Location::Coordinate(Coord {
                                x: 13.286_944_444_444_444,
                                y: 49.462_500_000_000_006
                            }),
                            colour: Some("colour_Releaseline".to_string())
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 13.286_944_444_444_444,
                                y: 49.462_500_000_000_006
                            }),
                            end: Location::Coordinate(Coord {
                                x: 13.220_555_555_555_556,
                                y: 49.211_666_666_666_666
                            }),
                            colour: Some("colour_Releaseline".to_string())
                        }
                    ]
                }
            ]
        );
        assert_eq!(
            sct.as_ref().unwrap().artccs,
            vec![
                Artcc {
                    name: "EDMM_ALB_CTR".to_string(),
                    lines: vec![
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 11.1325,
                                y: 49.138_055_555_555_55
                            }),
                            end: Location::Coordinate(Coord {
                                x: 11.966_666_666_666_667,
                                y: 49.166_666_666_666_664
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 11.794_166_666_666_667,
                                y: 48.6675
                            }),
                            end: Location::Coordinate(Coord {
                                x: 11.966_666_666_666_667,
                                y: 49.166_666_666_666_664
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 11.794_166_666_666_667,
                                y: 48.6675
                            }),
                            end: Location::Coordinate(Coord {
                                x: 11.511_666_666_666_667,
                                y: 48.667_777_777_777_77
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 11.511_666_666_666_667,
                                y: 48.667_777_777_777_77
                            }),
                            end: Location::Coordinate(Coord {
                                x: 11.320_833_333_333_333,
                                y: 48.667_777_777_777_77
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 11.320_833_333_333_333,
                                y: 48.667_777_777_777_77
                            }),
                            end: Location::Coordinate(Coord {
                                x: 10.673_611_111_111_11,
                                y: 49.119_444_444_444_45
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 10.673_611_111_111_11,
                                y: 49.119_444_444_444_45
                            }),
                            end: Location::Coordinate(Coord {
                                x: 11.1325,
                                y: 49.138_055_555_555_55
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 10.673_611_111_111_11,
                                y: 49.119_444_444_444_45
                            }),
                            end: Location::Coordinate(Coord {
                                x: 10.587_777_777_777_779,
                                y: 49.178_611_111_111_11
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 11.818_888_888_888_889,
                                y: 49.434_444_444_444_44
                            }),
                            end: Location::Coordinate(Coord {
                                x: 10.966_666_666_666_667,
                                y: 49.441_666_666_666_66
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 10.587_777_777_777_779,
                                y: 49.178_611_111_111_11
                            }),
                            end: Location::Coordinate(Coord {
                                x: 10.966_666_666_666_667,
                                y: 49.441_666_666_666_66
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 10.966_666_666_666_667,
                                y: 49.441_666_666_666_66
                            }),
                            end: Location::Coordinate(Coord {
                                x: 10.765_555_555_555_556,
                                y: 49.654_722_222_222_22
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 10.587_777_777_777_779,
                                y: 49.178_611_111_111_11
                            }),
                            end: Location::Coordinate(Coord {
                                x: 10.458_333_333_333_332,
                                y: 49.283_333_333_333_33
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 10.458_333_333_333_332,
                                y: 49.283_333_333_333_33
                            }),
                            end: Location::Coordinate(Coord {
                                x: 10.356_111_111_111_11,
                                y: 49.398_333_333_333_33
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 10.356_111_111_111_11,
                                y: 49.398_333_333_333_33
                            }),
                            end: Location::Coordinate(Coord {
                                x: 10.503_333_333_333_334,
                                y: 49.511_111_111_111_11
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 10.503_333_333_333_334,
                                y: 49.511_111_111_111_11
                            }),
                            end: Location::Coordinate(Coord {
                                x: 10.765_555_555_555_556,
                                y: 49.654_722_222_222_22
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 11.966_666_666_666_667,
                                y: 49.166_666_666_666_664
                            }),
                            end: Location::Coordinate(Coord {
                                x: 11.818_888_888_888_889,
                                y: 49.434_444_444_444_44
                            }),
                            colour: None
                        }
                    ]
                },
                Artcc {
                    name: "EDMM_WLD_CTR".to_string(),
                    lines: vec![
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 10.966_666_666_666_667,
                                y: 48.666_666_666_666_664
                            }),
                            end: Location::Coordinate(Coord {
                                x: 11.183_333_333_333_334,
                                y: 48.669_444_444_444_444
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 11.183_333_333_333_334,
                                y: 48.669_444_444_444_444
                            }),
                            end: Location::Coordinate(Coord {
                                x: 11.320_833_333_333_333,
                                y: 48.667_777_777_777_77
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 10.2575,
                                y: 49.017_222_222_222_22
                            }),
                            end: Location::Coordinate(Coord {
                                x: 10.479_166_666_666_666,
                                y: 49.111_111_111_111_114
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 10.479_166_666_666_666,
                                y: 49.111_111_111_111_114
                            }),
                            end: Location::Coordinate(Coord {
                                x: 10.587_777_777_777_779,
                                y: 49.178_611_111_111_11
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 11.320_833_333_333_333,
                                y: 48.667_777_777_777_77
                            }),
                            end: Location::Coordinate(Coord {
                                x: 10.673_611_111_111_11,
                                y: 49.119_444_444_444_45
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 10.673_611_111_111_11,
                                y: 49.119_444_444_444_45
                            }),
                            end: Location::Coordinate(Coord {
                                x: 10.587_777_777_777_779,
                                y: 49.178_611_111_111_11
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 10.904_444_444_444_445,
                                y: 48.624_444_444_444_44
                            }),
                            end: Location::Coordinate(Coord {
                                x: 10.2575,
                                y: 49.017_222_222_222_22
                            }),
                            colour: None
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 10.904_444_444_444_445,
                                y: 48.624_444_444_444_44
                            }),
                            end: Location::Coordinate(Coord {
                                x: 10.966_666_666_666_667,
                                y: 48.666_666_666_666_664
                            }),
                            colour: None
                        }
                    ]
                }
            ]
        );
        assert_eq!(
            sct.as_ref().unwrap().artccs_low,
            vec![
                Artcc {
                    name: "RMZ EDMS".to_string(),
                    lines: vec![
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 12.392_777_777_777_777,
                                y: 48.961_111_111_111_116
                            }),
                            end: Location::Coordinate(Coord {
                                x: 12.660_555_555_555_556,
                                y: 48.939_444_444_444_44
                            }),
                            colour: Some("colour_RMZ".to_string())
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 12.660_555_555_555_556,
                                y: 48.939_444_444_444_44
                            }),
                            end: Location::Coordinate(Coord {
                                x: 12.641_666_666_666_666,
                                y: 48.840_277_777_777_78
                            }),
                            colour: Some("colour_RMZ".to_string())
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 12.641_666_666_666_666,
                                y: 48.840_277_777_777_78
                            }),
                            end: Location::Coordinate(Coord {
                                x: 12.374_444_444_444_444,
                                y: 48.861_944_444_444_45
                            }),
                            colour: Some("colour_RMZ".to_string())
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 12.374_444_444_444_444,
                                y: 48.861_944_444_444_45
                            }),
                            end: Location::Coordinate(Coord {
                                x: 12.392_777_777_777_777,
                                y: 48.961_111_111_111_116
                            }),
                            colour: Some("colour_RMZ".to_string())
                        }
                    ]
                },
                Artcc {
                    name: "RMZ EDNX".to_string(),
                    lines: vec![
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 11.455_833_333_333_333,
                                y: 48.274_166_666_666_666
                            }),
                            end: Location::Coordinate(Coord {
                                x: 11.534_722_222_222_221,
                                y: 48.286_666_666_666_66
                            }),
                            colour: Some("colour_RMZ".to_string())
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 11.534_722_222_222_221,
                                y: 48.286_666_666_666_66
                            }),
                            end: Location::Coordinate(Coord {
                                x: 11.535_833_333_333_333,
                                y: 48.273_611_111_111_11
                            }),
                            colour: Some("colour_RMZ".to_string())
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 11.535_833_333_333_333,
                                y: 48.273_611_111_111_11
                            }),
                            end: Location::Coordinate(Coord {
                                x: 11.640_833_333_333_333,
                                y: 48.281_666_666_666_666
                            }),
                            colour: Some("colour_RMZ".to_string())
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 11.640_833_333_333_333,
                                y: 48.281_666_666_666_666
                            }),
                            end: Location::Coordinate(Coord {
                                x: 11.673_611_111_111_11,
                                y: 48.254_722_222_222_22
                            }),
                            colour: Some("colour_RMZ".to_string())
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 11.673_611_111_111_11,
                                y: 48.254_722_222_222_22
                            }),
                            end: Location::Coordinate(Coord {
                                x: 11.677_777_777_777_777,
                                y: 48.241_111_111_111_11
                            }),
                            colour: Some("colour_RMZ".to_string())
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 11.677_777_777_777_777,
                                y: 48.241_111_111_111_11
                            }),
                            end: Location::Coordinate(Coord {
                                x: 11.6625,
                                y: 48.203_888_888_888_89
                            }),
                            colour: Some("colour_RMZ".to_string())
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 11.6625,
                                y: 48.203_888_888_888_89
                            }),
                            end: Location::Coordinate(Coord {
                                x: 11.489_444_444_444_443,
                                y: 48.177_222_222_222_22
                            }),
                            colour: Some("colour_RMZ".to_string())
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 11.489_444_444_444_443,
                                y: 48.177_222_222_222_22
                            }),
                            end: Location::Coordinate(Coord {
                                x: 11.455_833_333_333_333,
                                y: 48.274_166_666_666_666
                            }),
                            colour: Some("colour_RMZ".to_string())
                        }
                    ]
                }
            ]
        );
        assert_eq!(
            sct.as_ref().unwrap().high_airways,
            vec![
                Airway {
                    designator: "B73".to_string(),
                    start: Location::Coordinate(Coord {
                        y: 54.912_777_777_777_78,
                        x: 18.958_332_777_777_777
                    }),
                    end: Location::Coordinate(Coord {
                        y: 55.603_610_833_333_335,
                        x: 19.838_305_833_333_333
                    })
                },
                Airway {
                    designator: "B74".to_string(),
                    start: Location::Coordinate(Coord {
                        y: 55.201_388_888_888_89,
                        x: 19.634_166_944_444_445
                    }),
                    end: Location::Coordinate(Coord {
                        y: 55.603_610_833_333_335,
                        x: 19.838_305_833_333_333
                    })
                },
                Airway {
                    designator: "B74".to_string(),
                    start: Location::Coordinate(Coord {
                        y: 54.637_777_777_777_78,
                        x: 19.355_555_833_333_334
                    }),
                    end: Location::Coordinate(Coord {
                        y: 55.201_388_888_888_89,
                        x: 19.634_166_944_444_445
                    })
                },
                Airway {
                    designator: "B75".to_string(),
                    start: Location::Fix("ARMUT".to_string()),
                    end: Location::Fix("VEMUT".to_string())
                },
                Airway {
                    designator: "B76".to_string(),
                    start: Location::Fix("ARMUT".to_string()),
                    end: Location::Fix("RTT".to_string())
                },
            ]
        );
        assert_eq!(
            sct.as_ref().unwrap().low_airways,
            vec![
                Airway {
                    designator: "A361".to_string(),
                    start: Location::Coordinate(Coord {
                        y: 48.939_166_944_444_445,
                        x: 0.953_055_833_333_333_3
                    }),
                    end: Location::Coordinate(Coord {
                        y: 48.790_610_833_333_33,
                        x: 0.530_277_777_777_777_8
                    })
                },
                Airway {
                    designator: "A361".to_string(),
                    start: Location::Coordinate(Coord {
                        y: 49.028_527_777_777_775,
                        x: 1.214_055_833_333_333_3
                    }),
                    end: Location::Coordinate(Coord {
                        y: 48.939_166_944_444_445,
                        x: 0.953_055_833_333_333_3
                    })
                },
                Airway {
                    designator: "A4".to_string(),
                    start: Location::Coordinate(Coord {
                        y: 48.617_568_888_888_89,
                        x: 17.541_166_944_444_445
                    }),
                    end: Location::Coordinate(Coord {
                        y: 48.715_832_777_777_78,
                        x: 17.386_110_833_333_333
                    })
                },
                Airway {
                    designator: "A4".to_string(),
                    start: Location::Coordinate(Coord {
                        y: 48.290_435_833_333_33,
                        x: 18.050_638_888_888_89
                    }),
                    end: Location::Coordinate(Coord {
                        y: 48.617_568_888_888_89,
                        x: 17.541_166_944_444_445
                    })
                },
                Airway {
                    designator: "A4".to_string(),
                    start: Location::Coordinate(Coord {
                        y: 48.715_832_777_777_78,
                        x: 17.386_110_833_333_333
                    }),
                    end: Location::Coordinate(Coord {
                        y: 48.8532,
                        x: 17.167_843_888_888_89
                    })
                },
                Airway {
                    designator: "A5".to_string(),
                    start: Location::Fix("RTT".to_string()),
                    end: Location::Fix("NUB".to_string())
                }
            ]
        );
        assert_eq_sorted!(
            sct.as_ref().unwrap().regions,
            vec![
                Region {
                    name: "EDDM Groundlayout".to_string(),
                    colour_name: "colour_Stopbar".to_string(),
                    polygon: vec![
                        Coord {
                            x: 11.808_931_111_111_113,
                            y: 48.364_894_722_222_225,
                        },
                        Coord {
                            x: 11.809_087_5,
                            y: 48.365_142_777_777_78,
                        },
                        Coord {
                            x: 11.809_047_5,
                            y: 48.365_146_388_888_89,
                        },
                        Coord {
                            x: 11.808_900_555_555_557,
                            y: 48.364_91,
                        },
                    ]
                },
                Region {
                    name: "EDMO Groundlayout".to_string(),
                    colour_name: "colour_HardSurface1".to_string(),
                    polygon: vec![
                        Coord {
                            x: 11.272_248_055_555_556,
                            y: 48.073_741_666_666_67,
                        },
                        Coord {
                            x: 11.272_699_166_666_667,
                            y: 48.073_474_722_222_22,
                        },
                        Coord {
                            x: 11.294_012_777_777_777,
                            y: 48.089_076_944_444_45,
                        },
                        Coord {
                            x: 11.293_563_611_111_11,
                            y: 48.089_343_888_888_89,
                        },
                    ],
                },
                Region {
                    name: "Surrounding Grass".to_string(),
                    colour_name: "grass".to_string(),
                    polygon: vec![
                        Coord {
                            x: 2.851_986_944_444_444_6,
                            y: 51.193_656_944_444_44,
                        },
                        Coord {
                            x: 2.852_350_833_333_333_4,
                            y: 51.194_060_833_333_33,
                        },
                        Coord {
                            x: 2.852_78,
                            y: 51.194_35,
                        },
                    ]
                }
            ]
        );
        assert_eq_sorted!(
            sct.as_ref().unwrap().geo,
            vec![
                Geo {
                    name: "EDDN Groundlayout Holding Points".to_string(),
                    lines: vec![
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 11.059_174_444_444_444,
                                y: 49.499_648_888_888_89,
                            }),
                            end: Location::Coordinate(Coord {
                                x: 11.059_542_500_000_001,
                                y: 49.499_706_111_111_11,
                            }),
                            colour: Some("colour_Stopbar".to_string()),
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 11.064_627_5,
                                y: 49.499_107_222_222_22,
                            }),
                            end: Location::Coordinate(Coord {
                                x: 11.065_006_111_111_112,
                                y: 49.499_168_333_333_34,
                            }),
                            colour: Some("colour_Stopbar".to_string()),
                        },
                    ],
                },
                Geo {
                    name: "EDQC Groundlayout".to_string(),
                    lines: vec![Line {
                        start: Location::Coordinate(Coord {
                            x: 10.991_542_5,
                            y: 50.264_447_5,
                        }),
                        end: Location::Coordinate(Coord {
                            x: 10.991_546_111_111_111,
                            y: 50.264_510_833_333_33,
                        }),
                        colour: None,
                    },],
                },
                Geo {
                    name: "EDQC Groundlayout".to_string(),
                    lines: vec![
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 10.991_542_5,
                                y: 50.264_447_5,
                            }),
                            end: Location::Coordinate(Coord {
                                x: 10.991_546_111_111_111,
                                y: 50.264_510_833_333_33,
                            }),
                            colour: Some("colour_Stopbar".to_string()),
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 11.001_35,
                                y: 50.260_929_166_666_66,
                            }),
                            end: Location::Coordinate(Coord {
                                x: 11.001_427_5,
                                y: 50.260_947_5,
                            }),
                            colour: Some("colour_Stopbar".to_string()),
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 11.001_35,
                                y: 50.260_929_166_666_66,
                            }),
                            end: Location::Coordinate(Coord {
                                x: 11.001_427_5,
                                y: 50.260_947_5,
                            }),
                            colour: None,
                        },
                        Line {
                            start: Location::Fix("RAVSA".to_string()),
                            end: Location::Fix("LCE13".to_string()),
                            colour: Some("geoDefault".to_string()),
                        },
                    ],
                },
                Geo {
                    name: "HIGHWAYS LOVV".to_string(),
                    lines: vec![
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 16.221_305_555_555_553,
                                y: 48.202_527_777_777_78,
                            }),
                            end: Location::Coordinate(Coord {
                                x: 16.202_805_555_555_553,
                                y: 48.190_861_111_111_104,
                            }),
                            colour: Some("COLOR_Landmark2".to_string()),
                        },
                        Line {
                            start: Location::Coordinate(Coord {
                                x: 11.001_35,
                                y: 50.260_929_166_666_66,
                            }),
                            end: Location::Coordinate(Coord {
                                x: 11.001_427_5,
                                y: 50.260_947_5,
                            }),
                            colour: Some("colour_Stopbar".to_string()),
                        },
                    ]
                },
            ]
        );
    }

    #[test]
    fn test_sct_fmt() {
        let sct_bytes = b"
;=========================================================================================================================;

[INFO]
AeroNav M\xfcnchen 2401/1-1 EDMM 20240125
AERO_NAV
ZZZZ
N048.21.13.618
E011.47.09.909
60
39
-3
1

#define colour_APP       16711680
#define colour_AirspaceA  8421376
#define prohibitcolour 7697781		; 117,117,117	Prohibited areas

[VOR]
NUB  115.750 N049.30.10.508 E011.02.06.000 ; NUB Comment Test
OTT  112.300 N048.10.49.418 E011.48.59.529

[NDB]
MIQ  426.000 N048.34.12.810 E011.35.51.010 ;MIQ Comment Test
RTT  303.000 N047.25.51.319 E011.56.24.190

[FIXES]
(FM-C) N049.31.05.999 E008.26.42.000
ARMUT N049.43.20.999 E012.19.23.998
GEDSO N047.04.50.001 E011.52.13.000
INBED N049.23.15.000 E010.56.30.001
NAXAV N046.27.49.881 E011.19.19.858
UNKUL N049.08.13.999 E011.27.34.999
VEMUT N049.48.38.678 E012.27.40.489

[AIRPORT]
EDDM 000.000 N048.21.13.618 E011.47.09.909 D
EDNX 000.000 N048.14.20.399 E011.33.33.001 D
LIPB 000.000 N046.27.37.000 E011.19.35.000 D

[RUNWAY]
08R 26L 080 260 N048.20.26.408 E011.45.03.661 N048.20.41.269 E011.48.16.610 EDDM
08L 26R 080 260 N048.21.45.961 E011.46.03.179 N048.22.00.789 E011.49.16.219 EDDM
07  25  071 251 N048.14.17.710 E011.33.14.090 N048.14.24.388 E011.33.51.998 EDNX
04 22 037 217 N054.36.43.340 W005.52.47.140 N054.37.29.480 W005.51.51.950 EGAC Belfast/City

[SID]
EDDM SID 26L BIBAGxS                     N048.20.25.315 E011.44.49.465 N048.20.15.608 E011.42.33.951
                                         N048.20.15.608 E011.42.33.951 N048.17.15.248 E011.42.28.821
                                         N048.17.15.248 E011.42.28.821 N048.10.49.418 E011.48.59.529
                                         N048.10.49.418 E011.48.59.529 N048.13.54.868 E012.33.55.821
                                         N048.13.54.868 E012.33.55.821 N048.23.49.380 E012.44.59.211

EDDN SID 28 BOLSIxG                      N049.30.02.914 E011.03.23.507 N049.30.47.941 E010.55.42.060
                                         N049.30.47.941 E010.55.42.060 N049.27.44.369 E010.45.45.219
                                         N049.27.44.369 E010.45.45.219 N049.13.54.559 E010.45.30.628

[STAR]
EDDM TRAN RNP26R LANDU26                 N048.35.46.899 E012.16.26.140 N048.32.16.501 E011.30.05.529 colour_APP
                                         N048.32.16.501 E011.30.05.529 N048.25.44.061 E011.31.15.931 colour_APP
                                         N048.25.44.061 E011.31.15.931 N048.29.36.319 E012.22.42.409 colour_APP
                                         N048.29.36.319 E012.22.42.409 N048.30.47.361 E012.37.38.952 colour_APP

EDDN TRAN ILS10 DN430                    N049.33.11.289 E010.30.33.379 N049.32.37.168 E010.36.38.149 colour_APP
                                         N049.32.37.168 E010.36.38.149 N049.32.02.731 E010.42.42.681 colour_APP

EDQD STAR ALL LONLIxZ                    N050.04.29.060 E011.13.34.989 N049.53.50.819 E011.24.28.540


[ARTCC HIGH]
EDJA_ILR_APP                             N048.10.28.000 E009.34.15.000 N048.18.09.000 E009.55.25.000
                                         N048.10.28.000 E009.34.15.000 N048.10.00.000 E009.33.00.000
                                         N047.58.59.000 E010.50.38.000 N048.14.12.000 E010.25.42.000
                                         N048.14.12.000 E010.25.42.000 N048.18.09.000 E009.55.25.000
                                         N047.58.59.000 E010.50.38.000 N047.52.03.000 E010.28.00.000
                                         N047.52.03.000 E010.28.00.000 N047.50.30.000 E010.23.00.000
                                         N047.50.30.000 E010.23.00.000 N047.49.05.000 E009.58.21.000
                                         N047.53.53.000 E009.50.08.000 N047.48.55.000 E009.54.18.000
                                         N047.48.55.000 E009.54.18.000 N047.49.05.000 E009.58.21.000
                                         N047.53.53.000 E009.50.08.000 N047.53.24.000 E009.33.00.000
                                         N047.53.24.000 E009.33.00.000 N047.58.24.000 E009.33.00.000
                                         N047.58.24.000 E009.33.00.000 N048.10.00.000 E009.33.00.000

Release line EDMM ARBAX Window           N049.26.33.000 E012.52.22.000 N049.35.18.000 E013.00.24.000 colour_Releaseline
                                         N049.35.18.000 E013.00.24.000 N049.27.45.000 E013.17.13.000 colour_Releaseline
                                         N049.27.45.000 E013.17.13.000 N049.12.42.000 E013.13.14.000 colour_Releaseline
[ARTCC]
EDMM_ALB_CTR                             N049.08.17.000 E011.07.57.000 N049.10.00.000 E011.58.00.000
                                         N048.40.03.000 E011.47.39.000 N049.10.00.000 E011.58.00.000
                                         N048.40.03.000 E011.47.39.000 N048.40.04.000 E011.30.42.000
                                         N048.40.04.000 E011.30.42.000 N048.40.04.000 E011.19.15.000
                                         N048.40.04.000 E011.19.15.000 N049.07.10.000 E010.40.25.000
                                         N049.07.10.000 E010.40.25.000 N049.08.17.000 E011.07.57.000
                                         N049.07.10.000 E010.40.25.000 N049.10.43.000 E010.35.16.000
                                         N049.26.04.000 E011.49.08.000 N049.26.30.000 E010.58.00.000
                                         N049.10.43.000 E010.35.16.000 N049.26.30.000 E010.58.00.000
                                         N049.26.30.000 E010.58.00.000 N049.39.17.000 E010.45.56.000
                                         N049.10.43.000 E010.35.16.000 N049.17.00.000 E010.27.30.000
                                         N049.17.00.000 E010.27.30.000 N049.23.54.000 E010.21.22.000
                                         N049.23.54.000 E010.21.22.000 N049.30.40.000 E010.30.12.000
                                         N049.30.40.000 E010.30.12.000 N049.39.17.000 E010.45.56.000
                                         N049.10.00.000 E011.58.00.000 N049.26.04.000 E011.49.08.000

EDMM_WLD_CTR                             N048.40.00.000 E010.58.00.000 N048.40.10.000 E011.11.00.000
                                         N048.40.10.000 E011.11.00.000 N048.40.04.000 E011.19.15.000
                                         N049.01.02.000 E010.15.27.000 N049.06.40.000 E010.28.45.000
                                         N049.06.40.000 E010.28.45.000 N049.10.43.000 E010.35.16.000
                                         N048.40.04.000 E011.19.15.000 N049.07.10.000 E010.40.25.000
                                         N049.07.10.000 E010.40.25.000 N049.10.43.000 E010.35.16.000
                                         N048.37.28.000 E010.54.16.000 N049.01.02.000 E010.15.27.000
                                         N048.37.28.000 E010.54.16.000 N048.40.00.000 E010.58.00.000


[ARTCC LOW]
RMZ EDMS                                 N048.57.40.000 E012.23.34.000 N048.56.22.000 E012.39.38.000 colour_RMZ
                                         N048.56.22.000 E012.39.38.000 N048.50.25.000 E012.38.30.000 colour_RMZ
                                         N048.50.25.000 E012.38.30.000 N048.51.43.000 E012.22.28.000 colour_RMZ
                                         N048.51.43.000 E012.22.28.000 N048.57.40.000 E012.23.34.000 colour_RMZ

RMZ EDNX                                 N048.16.27.000 E011.27.21.000 N048.17.12.000 E011.32.05.000 colour_RMZ
                                         N048.17.12.000 E011.32.05.000 N048.16.25.000 E011.32.09.000 colour_RMZ
                                         N048.16.25.000 E011.32.09.000 N048.16.54.000 E011.38.27.000 colour_RMZ
                                         N048.16.54.000 E011.38.27.000 N048.15.17.000 E011.40.25.000 colour_RMZ
                                         N048.15.17.000 E011.40.25.000 N048.14.28.000 E011.40.40.000 colour_RMZ
                                         N048.14.28.000 E011.40.40.000 N048.12.14.000 E011.39.45.000 colour_RMZ
                                         N048.12.14.000 E011.39.45.000 N048.10.38.000 E011.29.22.000 colour_RMZ
                                         N048.10.38.000 E011.29.22.000 N048.16.27.000 E011.27.21.000 colour_RMZ


[GEO]
EDDN Groundlayout Holding Points         N049.29.58.736 E011.03.33.028 N049.29.58.942 E011.03.34.353 colour_Stopbar
                                         N049.29.56.786 E011.03.52.659 N049.29.57.006 E011.03.54.022 colour_Stopbar

EDQC Groundlayout                        N050.15.52.011 E010.59.29.553 N050.15.52.239 E010.59.29.566

EDQC Groundlayout                        N050.15.52.011 E010.59.29.553 N050.15.52.239 E010.59.29.566 colour_Stopbar
                                         N050.15.39.345 E011.00.04.860 N050.15.39.411 E011.00.05.139 colour_Stopbar
                                         N050.15.39.345 E011.00.04.860 N050.15.39.411 E011.00.05.139
                                         RAVSA RAVSA LCE13 LCE13 geoDefault
HIGHWAYS LOVV                            N048.12.09.100 E016.13.16.700 N048.11.27.100 E016.12.10.100 COLOR_Landmark2
                                         N050.15.39.345 E011.00.04.860 N050.15.39.411 E011.00.05.139 colour_Stopbar
[REGIONS]
REGIONNAME EDDM Groundlayout
colour_Stopbar              N048.21.53.621 E011.48.32.152
                           N048.21.54.514 E011.48.32.715
                           N048.21.54.527 E011.48.32.571
                           N048.21.53.676 E011.48.32.042
REGIONNAME EDMO Groundlayout
colour_HardSurface1         N048.04.25.470 E011.16.20.093
                           N048.04.24.509 E011.16.21.717
                           N048.05.20.677 E011.17.38.446
                           N048.05.21.638 E011.17.36.829
Surrounding Grass	grass	N51.11.37.165 E02.51.07.153
 N51.11.38.619 E02.51.08.463
 N51.11.39.660 E02.51.10.008

[HIGH AIRWAY]
B73        N054.54.46.000 E018.57.29.998 N055.36.12.999 E019.50.17.901
B74        N055.12.05.000 E019.38.03.001 N055.36.12.999 E019.50.17.901
B74        N054.38.16.000 E019.21.20.001 N055.12.05.000 E019.38.03.001
B75        ARMUT ARMUT VEMUT VEMUT
B76        ARMUT ARMUT RTT RTT

[LOW AIRWAY]
A361       N048.56.21.001 E000.57.11.001 N048.47.26.199 E000.31.49.000
A361       N049.01.42.700 E001.12.50.601 N048.56.21.001 E000.57.11.001
A4         N048.37.03.248 E017.32.28.201 N048.42.56.998 E017.23.09.999
A4         N048.17.25.569 E018.03.02.300 N048.37.03.248 E017.32.28.201
A4         N048.42.56.998 E017.23.09.999 N048.51.11.520 E017.10.04.238
A5         RTT RTT NUB NUB
        ";
        let sct = Sct::parse(sct_bytes).unwrap();

        let sct_generated = sct.to_string();
        let expected_generated = "[INFO]
AeroNav München 2401/1-1 EDMM 20240125
AERO_NAV
ZZZZ
N048.21.13.618
E011.47.09.909
60
39
-3
1

#define colour_APP\t16711680
#define colour_AirspaceA\t8421376
#define prohibitcolour\t7697781

[VOR]
NUB  115.750 N049.30.10.508 E011.02.06.000
OTT  112.300 N048.10.49.418 E011.48.59.529

[NDB]
MIQ  426.000 N048.34.12.810 E011.35.51.010
RTT  303.000 N047.25.51.319 E011.56.24.190

[FIXES]
(FM-C) N049.31.05.999 E008.26.42.000
ARMUT N049.43.20.999 E012.19.23.998
GEDSO N047.04.50.001 E011.52.13.000
INBED N049.23.15.000 E010.56.30.001
NAXAV N046.27.49.881 E011.19.19.858
UNKUL N049.08.13.999 E011.27.34.999
VEMUT N049.48.38.678 E012.27.40.489

[AIRPORT]
EDDM 000.000 N048.21.13.618 E011.47.09.909 D
EDNX 000.000 N048.14.20.399 E011.33.33.001 D
LIPB 000.000 N046.27.37.000 E011.19.35.000 D

[RUNWAY]
08R 26L 080 260 N048.20.26.408 E011.45.03.661 N048.20.41.269 E011.48.16.610 EDDM
08L 26R 080 260 N048.21.45.961 E011.46.03.179 N048.22.00.789 E011.49.16.219 EDDM
07  25  071 251 N048.14.17.710 E011.33.14.090 N048.14.24.388 E011.33.51.998 EDNX
04  22  037 217 N054.36.43.340 W005.52.47.140 N054.37.29.480 W005.51.51.950 EGAC

[SID]
EDDM SID 26L BIBAGxS                     N048.20.25.315 E011.44.49.465 N048.20.15.608 E011.42.33.951
                                         N048.20.15.608 E011.42.33.951 N048.17.15.248 E011.42.28.821
                                         N048.17.15.248 E011.42.28.821 N048.10.49.418 E011.48.59.529
                                         N048.10.49.418 E011.48.59.529 N048.13.54.868 E012.33.55.821
                                         N048.13.54.868 E012.33.55.821 N048.23.49.380 E012.44.59.211

EDDN SID 28 BOLSIxG                      N049.30.02.914 E011.03.23.507 N049.30.47.941 E010.55.42.060
                                         N049.30.47.941 E010.55.42.060 N049.27.44.369 E010.45.45.219
                                         N049.27.44.369 E010.45.45.219 N049.13.54.559 E010.45.30.628

[STAR]
EDDM TRAN RNP26R LANDU26                 N048.35.46.899 E012.16.26.140 N048.32.16.501 E011.30.05.529 colour_APP
                                         N048.32.16.501 E011.30.05.529 N048.25.44.061 E011.31.15.931 colour_APP
                                         N048.25.44.061 E011.31.15.931 N048.29.36.319 E012.22.42.409 colour_APP
                                         N048.29.36.319 E012.22.42.409 N048.30.47.361 E012.37.38.952 colour_APP

EDDN TRAN ILS10 DN430                    N049.33.11.289 E010.30.33.379 N049.32.37.168 E010.36.38.149 colour_APP
                                         N049.32.37.168 E010.36.38.149 N049.32.02.731 E010.42.42.681 colour_APP

EDQD STAR ALL LONLIxZ                    N050.04.29.060 E011.13.34.989 N049.53.50.819 E011.24.28.540

[ARTCC HIGH]
EDJA_ILR_APP                             N048.10.28.000 E009.34.15.000 N048.18.09.000 E009.55.25.000
                                         N048.10.28.000 E009.34.15.000 N048.10.00.000 E009.33.00.000
                                         N047.58.59.000 E010.50.38.000 N048.14.12.000 E010.25.42.000
                                         N048.14.12.000 E010.25.42.000 N048.18.09.000 E009.55.25.000
                                         N047.58.59.000 E010.50.38.000 N047.52.03.000 E010.28.00.000
                                         N047.52.03.000 E010.28.00.000 N047.50.30.000 E010.23.00.000
                                         N047.50.30.000 E010.23.00.000 N047.49.05.000 E009.58.21.000
                                         N047.53.53.000 E009.50.08.000 N047.48.55.000 E009.54.18.000
                                         N047.48.55.000 E009.54.18.000 N047.49.05.000 E009.58.21.000
                                         N047.53.53.000 E009.50.08.000 N047.53.24.000 E009.33.00.000
                                         N047.53.24.000 E009.33.00.000 N047.58.24.000 E009.33.00.000
                                         N047.58.24.000 E009.33.00.000 N048.10.00.000 E009.33.00.000

Release line EDMM ARBAX Window           N049.26.33.000 E012.52.22.000 N049.35.18.000 E013.00.24.000 colour_Releaseline
                                         N049.35.18.000 E013.00.24.000 N049.27.45.000 E013.17.13.000 colour_Releaseline
                                         N049.27.45.000 E013.17.13.000 N049.12.42.000 E013.13.14.000 colour_Releaseline

[ARTCC]
EDMM_ALB_CTR                             N049.08.17.000 E011.07.57.000 N049.10.00.000 E011.58.00.000
                                         N048.40.03.000 E011.47.39.000 N049.10.00.000 E011.58.00.000
                                         N048.40.03.000 E011.47.39.000 N048.40.04.000 E011.30.42.000
                                         N048.40.04.000 E011.30.42.000 N048.40.04.000 E011.19.15.000
                                         N048.40.04.000 E011.19.15.000 N049.07.10.000 E010.40.25.000
                                         N049.07.10.000 E010.40.25.000 N049.08.17.000 E011.07.57.000
                                         N049.07.10.000 E010.40.25.000 N049.10.43.000 E010.35.16.000
                                         N049.26.04.000 E011.49.08.000 N049.26.30.000 E010.58.00.000
                                         N049.10.43.000 E010.35.16.000 N049.26.30.000 E010.58.00.000
                                         N049.26.30.000 E010.58.00.000 N049.39.17.000 E010.45.56.000
                                         N049.10.43.000 E010.35.16.000 N049.17.00.000 E010.27.30.000
                                         N049.17.00.000 E010.27.30.000 N049.23.54.000 E010.21.22.000
                                         N049.23.54.000 E010.21.22.000 N049.30.40.000 E010.30.12.000
                                         N049.30.40.000 E010.30.12.000 N049.39.17.000 E010.45.56.000
                                         N049.10.00.000 E011.58.00.000 N049.26.04.000 E011.49.08.000

EDMM_WLD_CTR                             N048.40.00.000 E010.58.00.000 N048.40.10.000 E011.11.00.000
                                         N048.40.10.000 E011.11.00.000 N048.40.04.000 E011.19.15.000
                                         N049.01.02.000 E010.15.27.000 N049.06.40.000 E010.28.45.000
                                         N049.06.40.000 E010.28.45.000 N049.10.43.000 E010.35.16.000
                                         N048.40.04.000 E011.19.15.000 N049.07.10.000 E010.40.25.000
                                         N049.07.10.000 E010.40.25.000 N049.10.43.000 E010.35.16.000
                                         N048.37.28.000 E010.54.16.000 N049.01.02.000 E010.15.27.000
                                         N048.37.28.000 E010.54.16.000 N048.40.00.000 E010.58.00.000

[ARTCC LOW]
RMZ EDMS                                 N048.57.40.000 E012.23.34.000 N048.56.22.000 E012.39.38.000 colour_RMZ
                                         N048.56.22.000 E012.39.38.000 N048.50.25.000 E012.38.30.000 colour_RMZ
                                         N048.50.25.000 E012.38.30.000 N048.51.43.000 E012.22.28.000 colour_RMZ
                                         N048.51.43.000 E012.22.28.000 N048.57.40.000 E012.23.34.000 colour_RMZ

RMZ EDNX                                 N048.16.27.000 E011.27.21.000 N048.17.12.000 E011.32.05.000 colour_RMZ
                                         N048.17.12.000 E011.32.05.000 N048.16.25.000 E011.32.09.000 colour_RMZ
                                         N048.16.25.000 E011.32.09.000 N048.16.54.000 E011.38.27.000 colour_RMZ
                                         N048.16.54.000 E011.38.27.000 N048.15.17.000 E011.40.25.000 colour_RMZ
                                         N048.15.17.000 E011.40.25.000 N048.14.28.000 E011.40.40.000 colour_RMZ
                                         N048.14.28.000 E011.40.40.000 N048.12.14.000 E011.39.45.000 colour_RMZ
                                         N048.12.14.000 E011.39.45.000 N048.10.38.000 E011.29.22.000 colour_RMZ
                                         N048.10.38.000 E011.29.22.000 N048.16.27.000 E011.27.21.000 colour_RMZ

[GEO]
EDDN Groundlayout Holding Points         N049.29.58.736 E011.03.33.028 N049.29.58.942 E011.03.34.353 colour_Stopbar
                                         N049.29.56.786 E011.03.52.659 N049.29.57.006 E011.03.54.022 colour_Stopbar

EDQC Groundlayout                        N050.15.52.011 E010.59.29.553 N050.15.52.239 E010.59.29.566

EDQC Groundlayout                        N050.15.52.011 E010.59.29.553 N050.15.52.239 E010.59.29.566 colour_Stopbar
                                         N050.15.39.345 E011.00.04.860 N050.15.39.411 E011.00.05.139 colour_Stopbar
                                         N050.15.39.345 E011.00.04.860 N050.15.39.411 E011.00.05.139
                                         RAVSA RAVSA LCE13 LCE13 geoDefault

HIGHWAYS LOVV                            N048.12.09.100 E016.13.16.700 N048.11.27.100 E016.12.10.100 COLOR_Landmark2
                                         N050.15.39.345 E011.00.04.860 N050.15.39.411 E011.00.05.139 colour_Stopbar

[REGIONS]
REGIONNAME EDDM Groundlayout
colour_Stopbar             N048.21.53.621 E011.48.32.152
                           N048.21.54.514 E011.48.32.715
                           N048.21.54.527 E011.48.32.571
                           N048.21.53.676 E011.48.32.042
REGIONNAME EDMO Groundlayout
colour_HardSurface1        N048.04.25.470 E011.16.20.093
                           N048.04.24.509 E011.16.21.717
                           N048.05.20.677 E011.17.38.446
                           N048.05.21.638 E011.17.36.829
REGIONNAME Surrounding Grass
grass                      N051.11.37.165 E002.51.07.153
                           N051.11.38.619 E002.51.08.463
                           N051.11.39.660 E002.51.10.008

[HIGH AIRWAY]
B73        N054.54.46.000 E018.57.29.998 N055.36.12.999 E019.50.17.901
B74        N055.12.05.000 E019.38.03.001 N055.36.12.999 E019.50.17.901
B74        N054.38.16.000 E019.21.20.001 N055.12.05.000 E019.38.03.001
B75        ARMUT ARMUT VEMUT VEMUT
B76        ARMUT ARMUT RTT RTT

[LOW AIRWAY]
A361       N048.56.21.001 E000.57.11.001 N048.47.26.199 E000.31.49.000
A361       N049.01.42.700 E001.12.50.601 N048.56.21.001 E000.57.11.001
A4         N048.37.03.248 E017.32.28.201 N048.42.56.998 E017.23.09.999
A4         N048.17.25.569 E018.03.02.300 N048.37.03.248 E017.32.28.201
A4         N048.42.56.998 E017.23.09.999 N048.51.11.520 E017.10.04.238
A5         RTT RTT NUB NUB
";
        assert_eq_sorted!(expected_generated, sct_generated);
    }

    #[test]
    fn test_empty_section() {
        let sct_bytes = b"
;=========================================================================================================================;

[INFO]
AeroNav M\xfcnchen 2401/1-1 EDMM 20240125
AERO_NAV
ZZZZ
N048.21.13.618
E011.47.09.909
60
39
-3
1

#define colour_APP       16711680
#define colour_AirspaceA  8421376

[SID]

[STAR]
EDDM TRAN RNP26R LANDU26                 N048.35.46.899 E012.16.26.140 N048.32.16.501 E011.30.05.529 colour_APP
                                         N048.32.16.501 E011.30.05.529 N048.25.44.061 E011.31.15.931 colour_APP
                                         N048.25.44.061 E011.31.15.931 N048.29.36.319 E012.22.42.409 colour_APP
                                         N048.29.36.319 E012.22.42.409 N048.30.47.361 E012.37.38.952 colour_APP

EDDN TRAN ILS10 DN430                    N049.33.11.289 E010.30.33.379 N049.32.37.168 E010.36.38.149 colour_APP
                                         N049.32.37.168 E010.36.38.149 N049.32.02.731 E010.42.42.681 colour_APP

EDQD STAR ALL LONLIxZ                    N050.04.29.060 E011.13.34.989 N049.53.50.819 E011.24.28.540
[Regions]
REGIONNAME test
colour N048.35.46.899 E012.16.26.140";
        let sct = Sct::parse(sct_bytes);

        assert!(sct.is_ok());
    }

    #[test]
    fn test_geo_at_end() {
        let sct_bytes = b"[GEO]

EDDN Groundlayout Holding Points         N049.29.58.736 E011.03.33.028 N049.29.58.942 E011.03.34.353 colour_Stopbar
                                         N049.29.56.786 E011.03.52.659 N049.29.57.006 E011.03.54.022 colour_Stopbar";
        let sct = Sct::parse(sct_bytes);

        assert!(sct.is_ok());
    }
}
