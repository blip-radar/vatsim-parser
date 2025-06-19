use std::collections::HashMap;
use std::io;

use bevy_reflect::Reflect;
use geo::{Coord, LineString};
use pest::{iterators::Pair, Parser};
use pest_derive::Parser;
use serde::Serialize;
use thiserror::Error;
use tracing::warn;

use crate::{adaptation::maps::active::RunwayIdentifier, DegMinSec, DegMinSecExt as _};

use super::read_to_string;

#[derive(Parser)]
#[grammar = "pest/base.pest"]
#[grammar = "pest/ese.pest"]
pub struct EseParser;

#[derive(Error, Debug)]
pub enum EseError {
    #[error("failed to parse .ese file: {0}")]
    Parse(#[from] pest::error::Error<Rule>),
    #[error("failed to read .ese file: {0}")]
    FileRead(#[from] io::Error),
}

#[derive(Clone, Debug, Reflect, Serialize, PartialEq)]
pub struct Position {
    pub name: String,
    pub callsign: String,
    pub frequency: String,
    pub identifier: String,
    pub prefix: String,
    pub middle: String,
    pub suffix: String,
    pub squawk_range: Option<(u16, u16)>,
    #[reflect(ignore)]
    pub visibility_points: Vec<Coord>,
}

#[derive(Clone, Debug, Reflect, Serialize, PartialEq)]
pub struct MSAW {
    pub altitude: u32,
    #[reflect(ignore)]
    pub points: Vec<Coord>,
}
impl MSAW {
    fn parse(pair: Pair<Rule>) -> (String, Self) {
        let mut msaw = pair.into_inner();
        let id = msaw.next().unwrap().as_str().to_string();
        let altitude = msaw.next().unwrap().as_str().parse().unwrap();
        let points = msaw.map(parse_coordinate).collect();

        (id, Self { altitude, points })
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct SectorLine {
    pub points: LineString,
    // TODO
    // pub display_config: Vec<(String, String, String)>,
}
impl SectorLine {
    fn parse(pair: Pair<Rule>) -> (String, Self) {
        let mut sectorline = pair.into_inner();
        let id = sectorline.next().unwrap().as_str().to_string();
        // TODO _display
        let (_display, coords): (Vec<Pair<Rule>>, Vec<Pair<Rule>>) = sectorline
            .partition(|display_or_coord| matches!(display_or_coord.as_rule(), Rule::display));

        (
            id,
            Self {
                points: coords.into_iter().map(parse_coordinate).collect(),
            },
        )
    }
}

#[derive(Clone, Debug, Reflect, Serialize, PartialEq)]
pub struct CircleSectorLine {
    pub center: String,
    pub radius: f32,
}
impl CircleSectorLine {
    fn parse(pair: Pair<Rule>) -> (String, Self) {
        let mut circle_sectorline = pair.into_inner();
        let id = circle_sectorline.next().unwrap().as_str().to_string();
        let center = circle_sectorline.next().unwrap().as_str().to_string();
        let radius = circle_sectorline.next().unwrap().as_str().parse().unwrap();
        // TODO _display

        (id, Self { center, radius })
    }
}

#[derive(Clone, Debug, Reflect, Serialize, Eq, PartialEq)]
enum SectorSubsetting {
    Owner(Vec<String>),
    AlternativeOwner(String, Vec<String>),
    Border(Vec<String>),
    Active(String, String),
    DepartureAirports(Vec<String>),
    ArrivalAirports(Vec<String>),
    // TODO
    Guest,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Sector {
    pub id: String,
    pub bottom: u32,
    pub top: u32,
    pub border: Vec<SectorLine>,
    pub owner_priority: Vec<String>,
    pub departure_airports: Vec<String>,
    pub arrival_airports: Vec<String>,
    // TODO?
    // pub alt_owner_priority: Vec<String>,
    // pub guest: Vec<String>,
    pub runway_filter: Vec<RunwayIdentifier>,
}

impl Sector {
    fn parse(pair: Pair<Rule>) -> (Vec<String>, Self) {
        let mut sector = pair.into_inner();
        let id = sector.next().unwrap().as_str().to_string();
        let bottom = sector.next().unwrap().as_str().parse().unwrap();
        let top = sector.next().unwrap().as_str().parse().unwrap();
        let subsettings = sector.map(Self::parse_subsettings).collect::<Vec<_>>();
        let owner_priority = subsettings
            .iter()
            .find_map(|subsetting| {
                if let SectorSubsetting::Owner(owner) = subsetting {
                    Some(owner.clone())
                } else {
                    None
                }
            })
            .unwrap_or(vec![]);
        let departure_airports = subsettings
            .iter()
            .find_map(|subsetting| {
                if let SectorSubsetting::DepartureAirports(airports) = subsetting {
                    Some(airports.clone())
                } else {
                    None
                }
            })
            .unwrap_or(vec![]);
        let arrival_airports = subsettings
            .iter()
            .find_map(|subsetting| {
                if let SectorSubsetting::ArrivalAirports(airports) = subsetting {
                    Some(airports.clone())
                } else {
                    None
                }
            })
            .unwrap_or(vec![]);
        let border = subsettings
            .iter()
            .find_map(|subsetting| {
                if let SectorSubsetting::Border(border) = subsetting {
                    Some(border.clone())
                } else {
                    None
                }
            })
            .unwrap_or(vec![]);
        let runway_filter = subsettings
            .iter()
            .filter_map(|subsetting| {
                if let SectorSubsetting::Active(icao, designator) = subsetting {
                    Some(RunwayIdentifier {
                        icao: icao.clone(),
                        designator: designator.clone(),
                    })
                } else {
                    None
                }
            })
            .collect();

        (
            border,
            Self {
                id,
                bottom,
                top,
                owner_priority,
                departure_airports,
                arrival_airports,
                runway_filter,
                // replaced later on
                border: vec![],
            },
        )
    }

    fn parse_subsettings(pair: Pair<Rule>) -> SectorSubsetting {
        match pair.as_rule() {
            Rule::owner => SectorSubsetting::Owner(
                pair.into_inner()
                    .map(|pair| pair.as_str().to_string())
                    .collect(),
            ),
            Rule::altowner => {
                let mut altowner = pair.into_inner();
                SectorSubsetting::AlternativeOwner(
                    altowner.next().unwrap().as_str().to_string(),
                    altowner.map(|pair| pair.as_str().to_string()).collect(),
                )
            }
            Rule::border => SectorSubsetting::Border(
                pair.into_inner()
                    .map(|pair| pair.as_str().to_string())
                    .collect(),
            ),
            Rule::guest => SectorSubsetting::Guest,
            Rule::active => {
                let mut active = pair.into_inner();
                SectorSubsetting::Active(
                    active.next().unwrap().as_str().to_string(),
                    active.next().unwrap().as_str().to_string(),
                )
            }
            Rule::depapt => SectorSubsetting::DepartureAirports(
                pair.into_inner()
                    .map(|pair| pair.as_str().to_string())
                    .collect(),
            ),
            Rule::arrapt => SectorSubsetting::ArrivalAirports(
                pair.into_inner()
                    .map(|pair| pair.as_str().to_string())
                    .collect(),
            ),
            rule => unreachable!("{rule:?}"),
        }
    }
}

fn parse_wildcard_string(pair: &Pair<Rule>) -> Option<String> {
    match pair.as_rule() {
        Rule::wildcard => None,
        Rule::colon_delimited_text | Rule::designator => Some(pair.as_str().to_string()),
        rule => unreachable!("{rule:?}"),
    }
}

fn parse_wildcard_u32(pair: &Pair<Rule>) -> Option<u32> {
    match pair.as_rule() {
        Rule::wildcard => None,
        Rule::integer => Some(pair.as_str().parse().unwrap()),
        rule => unreachable!("{rule:?}"),
    }
}

#[derive(Clone, Debug, Reflect, Serialize, PartialEq, Eq)]
pub struct Agreement {
    pub previous_fix: Option<String>,
    pub departure_runway: Option<String>,
    pub subsequent_fix: Option<String>,
    pub arrival_runway: Option<String>,
    pub fix: Option<String>,
    pub exit_sector: String,
    pub entry_sector: String,
    pub climb_level: Option<u32>,
    pub descent_level: Option<u32>,
    pub description: String,
}
impl Agreement {
    fn parse(pair: Pair<Rule>) -> Self {
        let mut cop = pair.into_inner();
        let previous_fix = parse_wildcard_string(&cop.next().unwrap());
        let departure_runway = parse_wildcard_string(&cop.next().unwrap());
        let fix = parse_wildcard_string(&cop.next().unwrap());
        let subsequent_fix = parse_wildcard_string(&cop.next().unwrap());
        let arrival_runway = parse_wildcard_string(&cop.next().unwrap());
        let exit_sector = cop.next().unwrap().as_str().to_string();
        let entry_sector = cop.next().unwrap().as_str().to_string();
        let climb_level = parse_wildcard_u32(&cop.next().unwrap());
        let descent_level = parse_wildcard_u32(&cop.next().unwrap());
        let description = cop.next().unwrap().as_str().to_string();

        Self {
            previous_fix,
            departure_runway,
            subsequent_fix,
            arrival_runway,
            fix,
            exit_sector,
            entry_sector,
            climb_level,
            descent_level,
            description,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct SID {
    pub name: String,
    pub airport: String,
    pub runway: Option<String>,
    pub waypoints: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct STAR {
    pub name: String,
    pub airport: String,
    pub runway: Option<String>,
    pub waypoints: Vec<String>,
}

#[derive(Debug, Default, Serialize, PartialEq)]
pub struct Ese {
    pub positions: HashMap<String, Position>,
    pub sectors: HashMap<String, Sector>,
    pub agreements: Vec<Agreement>,
    pub sids_stars: Vec<SidStar>,
}

pub type EseResult = Result<Ese, EseError>;

#[derive(Debug)]
enum Section {
    Positions(HashMap<String, Position>),
    Sectors((HashMap<String, Sector>, Vec<Agreement>)),
    SidsStars(Vec<SidStar>),
    Unsupported,
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum SectionName {
    Position,
    Airspace,
    SidsStars,
    Unsupported,
}

fn parse_coordinate_part(pair: Pair<Rule>) -> DegMinSec {
    let mut coordinate_part = pair.into_inner().next().unwrap().into_inner();
    let hemi = coordinate_part.next().unwrap().as_str();
    let degrees = coordinate_part
        .next()
        .unwrap()
        .as_str()
        .parse::<i16>()
        .unwrap()
        * match hemi {
            "N" | "E" | "n" | "e" => 1,
            "S" | "W" | "s" | "w" => -1,
            _ => unreachable!("{hemi} is not a hemisphere"),
        };
    let min = coordinate_part.next().unwrap().as_str().parse().unwrap();
    let sec = coordinate_part.next().unwrap().as_str().parse().unwrap();

    (degrees, min, sec)
}

// TODO generalise this and other similar into trait
fn parse_coordinate(pair: Pair<Rule>) -> Coord {
    let mut coordinate = pair.into_inner();
    let lat = parse_coordinate_part(coordinate.next().unwrap());
    let lng = parse_coordinate_part(coordinate.next().unwrap());
    Coord::from_deg_min_sec(lat, lng)
}

enum SectorRule {
    Sector((Vec<String>, Sector)),
    SectorLine((String, SectorLine)),
    FirCop(Agreement),
    Cop(Agreement),
    CircleSectorLine((String, CircleSectorLine)),
    DisplaySectorline,
    Msaw((String, MSAW)),
}

fn parse_airspace(pair: Pair<Rule>) -> SectorRule {
    match pair.as_rule() {
        Rule::sectorline => SectorRule::SectorLine(SectorLine::parse(pair)),
        Rule::sector => SectorRule::Sector(Sector::parse(pair)),
        Rule::cop => SectorRule::Cop(Agreement::parse(pair)),
        Rule::fir_cop => SectorRule::FirCop(Agreement::parse(pair)),
        Rule::display_sectorline => SectorRule::DisplaySectorline,
        Rule::circle_sectorline => SectorRule::CircleSectorLine(CircleSectorLine::parse(pair)),
        Rule::msaw => SectorRule::Msaw(MSAW::parse(pair)),
        rule => unreachable!("{rule:?}"),
    }
}

fn parse_squawk_range(maybe_pair: Option<Pair<Rule>>) -> Option<(u16, u16)> {
    maybe_pair.and_then(|pair| match pair.as_rule() {
        Rule::squawk_range => {
            let mut squawk_range = pair.into_inner();
            let squawk_begin = squawk_range.next().unwrap().as_str().parse().unwrap();
            let squawk_end = squawk_range.next().unwrap().as_str().parse().unwrap();
            Some((squawk_begin, squawk_end))
        }
        _ => None,
    })
}

fn parse_position(pair: Pair<Rule>) -> (String, Position) {
    let mut position = pair.into_inner();
    let name = position.next().unwrap().as_str().to_string();
    let callsign = position.next().unwrap().as_str().to_string();
    let frequency = position.next().unwrap().as_str().to_string();
    let identifier = position.next().unwrap().as_str().to_string();
    let middle = position.next().unwrap().as_str().to_string();
    let prefix = position.next().unwrap().as_str().to_string();
    let suffix = position.next().unwrap().as_str().to_string();
    // skip unused fields
    let mut position = position.skip(2);
    let squawk_range = parse_squawk_range(position.next());
    let visibility_points = position.map(parse_coordinate).collect();

    (
        identifier.clone(),
        Position {
            name,
            callsign,
            frequency,
            identifier,
            prefix,
            middle,
            suffix,
            squawk_range,
            visibility_points,
        },
    )
}

type SectorsLinesBorders = (
    HashMap<String, Sector>,
    HashMap<String, CircleSectorLine>,
    HashMap<String, SectorLine>,
    HashMap<String, Vec<String>>,
    Vec<Agreement>,
);
fn collect_sectors(
    (mut sectors, mut circle_sector_lines, mut sector_lines, mut borders, mut agreements): SectorsLinesBorders,
    rule: SectorRule,
) -> SectorsLinesBorders {
    match rule {
        SectorRule::Cop(cop) => {
            agreements.push(cop);
        }
        SectorRule::FirCop(cop) => agreements.push(cop),
        SectorRule::SectorLine((id, sector_line)) => {
            if let Some(_overwritten) = sector_lines.insert(id.clone(), sector_line) {
                warn!("duplicate sector_line: {id}");
            }
        }
        SectorRule::CircleSectorLine((id, circle_sector_line)) => {
            if let Some(_overwritten) = circle_sector_lines.insert(id.clone(), circle_sector_line) {
                warn!("duplicate cirle_sector_line: {id}");
            }
        }
        SectorRule::Sector((border, sector)) => {
            borders.insert(sector.id.clone(), border);
            if let Some(overwritten) = sectors.insert(sector.id.clone(), sector) {
                warn!("duplicate sector: {}", overwritten.id);
            }
        }
        // TODO
        SectorRule::Msaw((_sector, _level)) => (),
        SectorRule::DisplaySectorline => (),
    }
    (
        sectors,
        circle_sector_lines,
        sector_lines,
        borders,
        agreements,
    )
}

fn combine_sectors_with_borders(
    (sectors, _circle_sector_lines, lines, borders, agreements): SectorsLinesBorders,
) -> (HashMap<String, Sector>, Vec<Agreement>) {
    // TODO circle_sector_lines
    (
        sectors
            .into_iter()
            .map(|(id, mut sector)| {
                sector.border = borders
                    .get(&id)
                    .map(|border| {
                        border
                            .iter()
                            .filter_map(|line_id| lines.get(line_id).cloned())
                            .collect()
                    })
                    .unwrap_or(vec![]);

                (id, sector)
            })
            .collect(),
        agreements,
    )
}

#[derive(Debug, PartialEq, Serialize)]
pub enum SidStar {
    Sid(SID),
    Star(STAR),
}
fn parse_sid_star(pair: Pair<Rule>) -> SidStar {
    let rule = pair.as_rule();
    let mut sid_star = pair.into_inner();
    let airport = sid_star.next().unwrap().as_str().to_string();
    let runway_pair = sid_star.next().unwrap();
    let runway = match runway_pair.as_rule() {
        Rule::runway_designator => Some(runway_pair.as_str().to_string()),
        Rule::none => None,
        rule => unreachable!("{rule:?}"),
    };
    let name = sid_star.next().unwrap().as_str().to_string();
    let waypoints = sid_star
        .next()
        .unwrap()
        .into_inner()
        .map(|wpt| wpt.as_str().to_string())
        .collect();
    match rule {
        Rule::sid => SidStar::Sid(SID {
            name,
            airport,
            runway,
            waypoints,
        }),
        Rule::star => SidStar::Star(STAR {
            name,
            airport,
            runway,
            waypoints,
        }),
        rule => unreachable!("{rule:?}"),
    }
}

fn parse_section(pair: Pair<Rule>) -> (SectionName, Section) {
    match pair.as_rule() {
        Rule::position_section => (
            SectionName::Position,
            Section::Positions(pair.into_inner().map(parse_position).collect()),
        ),
        Rule::airspace_section => (
            SectionName::Airspace,
            Section::Sectors({
                combine_sectors_with_borders(
                    pair.into_inner()
                        .map(parse_airspace)
                        // TODO combine sectors with lines
                        .fold(
                            (
                                HashMap::new(),
                                HashMap::new(),
                                HashMap::new(),
                                HashMap::new(),
                                vec![],
                            ),
                            collect_sectors,
                        ),
                )
            }),
        ),
        Rule::sidsstars_section => (
            SectionName::SidsStars,
            Section::SidsStars(pair.into_inner().map(parse_sid_star).collect()),
        ),
        _ => (SectionName::Unsupported, Section::Unsupported),
    }
}

impl Ese {
    pub fn parse(content: &[u8]) -> EseResult {
        let unparsed_file = read_to_string(content)?;
        let mut sections = EseParser::parse(Rule::ese, &unparsed_file).map(|mut pairs| {
            pairs
                .next()
                .unwrap()
                .into_inner()
                .map(parse_section)
                .collect::<HashMap<_, _>>()
        })?;
        let positions = match sections.remove_entry(&SectionName::Position) {
            Some((_, Section::Positions(positions))) => positions,
            _ => HashMap::new(),
        };
        let (sectors, agreements) = match sections.remove_entry(&SectionName::Airspace) {
            Some((_, Section::Sectors(sectors))) => sectors,
            _ => (HashMap::new(), vec![]),
        };
        let sids_stars = match sections.remove_entry(&SectionName::SidsStars) {
            Some((_, Section::SidsStars(sids_stars))) => sids_stars,
            _ => vec![],
        };

        Ok(Ese {
            positions,
            sectors,
            agreements,
            sids_stars,
        })
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use geo::line_string;

    use crate::{
        ese::{Agreement, Ese, Position, SectorLine, SidStar, SID, STAR},
        Coord,
    };

    #[test]
    fn test_ese_positions() {
        let ese_bytes = b"
[POSITIONS]
EDDM_ATIS:Muenchen ATIS:123.130:MX::EDDM:ATIS:::0000:0000
EDMM_ALB_CTR:Muenchen Radar:129.100:ALB:ALB:EDMM:CTR:::2354:2367:N049.02.24.501:E012.31.35.850
EDMM_TEG_CTR:Muenchen Radar:133.680:TEG:TEG:EDMM:CTR:::2354:2367:N048.10.49.419:E011.48.59.530
EDXX_FIS_CTR:Langen Information:128.950:GIXX:FIS:EDXX:CTR:::2001:2577:N049.26.51.334:E010.13.06.336:N052.28.08.891:E010.52.12.796
EGBB_ATIS:Birmingham ATIS:136.030:BBI:B:EGBB:ATIS:-:-::

[SIDSSTARS]
STAR:EDDN:28:UPALA1V:UPALA DN463 DN462 DN461 DN452 DN453 DN454 DN455 DN456 DN457 DN458 DN459 DN439 DN438 DN437 OSNUB NGD32
STAR:EDMO:22:MAHxRNP:MAH MO220 MO221 EDIMO
SID:EDDM:26R:GIVMI1N:DM060 DM063 GIVMI
        ";
        let ese = Ese::parse(ese_bytes);
        assert_eq!(
            ese.as_ref().unwrap().positions,
            HashMap::from([
                (
                    "MX".to_string(),
                    Position {
                        name: "EDDM_ATIS".to_string(),
                        callsign: "Muenchen ATIS".to_string(),
                        frequency: "123.130".to_string(),
                        identifier: "MX".to_string(),
                        prefix: "EDDM".to_string(),
                        middle: String::new(),
                        suffix: "ATIS".to_string(),
                        squawk_range: Some((0, 0)),
                        visibility_points: vec![],
                    }
                ),
                (
                    "ALB".to_string(),
                    Position {
                        name: "EDMM_ALB_CTR".to_string(),
                        callsign: "Muenchen Radar".to_string(),
                        frequency: "129.100".to_string(),
                        identifier: "ALB".to_string(),
                        prefix: "EDMM".to_string(),
                        middle: "ALB".to_string(),
                        suffix: "CTR".to_string(),
                        squawk_range: Some((2354, 2367)),
                        visibility_points: vec![Coord {
                            y: 49.040_139_166_666_66,
                            x: 12.526_625_000_000_001
                        }],
                    }
                ),
                (
                    "TEG".to_string(),
                    Position {
                        name: "EDMM_TEG_CTR".to_string(),
                        callsign: "Muenchen Radar".to_string(),
                        frequency: "133.680".to_string(),
                        identifier: "TEG".to_string(),
                        prefix: "EDMM".to_string(),
                        middle: "TEG".to_string(),
                        suffix: "CTR".to_string(),
                        squawk_range: Some((2354, 2367)),
                        visibility_points: vec![Coord {
                            y: 48.180_394_166_666_666,
                            x: 11.816_536_111_111_112
                        }],
                    }
                ),
                (
                    "GIXX".to_string(),
                    Position {
                        name: "EDXX_FIS_CTR".to_string(),
                        callsign: "Langen Information".to_string(),
                        frequency: "128.950".to_string(),
                        identifier: "GIXX".to_string(),
                        prefix: "EDXX".to_string(),
                        middle: "FIS".to_string(),
                        suffix: "CTR".to_string(),
                        squawk_range: Some((2001, 2577)),
                        visibility_points: vec![
                            Coord {
                                y: 49.447_592_777_777_77,
                                x: 10.218_426_666_666_668
                            },
                            Coord {
                                y: 52.469_136_388_888_89,
                                x: 10.870_221_111_111_112
                            }
                        ],
                    }
                ),
                (
                    "BBI".to_string(),
                    Position {
                        name: "EGBB_ATIS".to_string(),
                        callsign: "Birmingham ATIS".to_string(),
                        frequency: "136.030".to_string(),
                        identifier: "BBI".to_string(),
                        prefix: "EGBB".to_string(),
                        middle: "B".to_string(),
                        suffix: "ATIS".to_string(),
                        squawk_range: None,
                        visibility_points: vec![]
                    }
                )
            ])
        );
    }

    #[test]
    fn test_ese_sectors() {
        let ese_bytes = b"
[AIRSPACE]

SECTORLINE:109
COORD:N049.08.17.000:E011.07.57.000 ; inline comment
COORD:N049.10.00.000:E011.58.00.000
DISPLAY:EDMM\xb7ETSIA\xb7000\xb7075:EDMM\xb7ETSIA\xb7000\xb7075:EDMM\xb7EDMMALB\xb7000\xb7105
DISPLAY:EDMM\xb7ETSIA\xb7000\xb7075:EDMM\xb7ETSIA\xb7000\xb7075:EDMM\xb7EDMMFRK\xb7000\xb7135
DISPLAY:EDMM\xb7EDMMALB\xb7000\xb7105:EDMM\xb7EDMMALB\xb7000\xb7105:EDMM\xb7ETSIA\xb7000\xb7075
DISPLAY:EDMM\xb7EDMMALB\xb7000\xb7105:EDMM\xb7EDMMALB\xb7000\xb7105:EDMM\xb7EDMMFRK\xb7000\xb7135
DISPLAY:EDMM\xb7EDMMFRK\xb7000\xb7135:EDMM\xb7EDMMFRK\xb7000\xb7135:EDMM\xb7ETSIA\xb7000\xb7075
DISPLAY:EDMM\xb7EDMMFRK\xb7000\xb7135:EDMM\xb7EDMMFRK\xb7000\xb7135:EDMM\xb7EDMMALB\xb7000\xb7105
DISPLAY:EDMM\xb7EDMMFRK\xb7000\xb7135:EDMM\xb7EDMMFRK\xb7000\xb7135:EDMM\xb7EDMMALB\xb7105\xb7135
DISPLAY:EDMM\xb7EDMMALB\xb7105\xb7135:EDMM\xb7EDMMALB\xb7105\xb7135:EDMM\xb7EDMMFRK\xb7000\xb7135

SECTORLINE:152
COORD:N048.40.03.000:E011.47.39.000
COORD:N049.10.00.000:E011.58.00.000
DISPLAY:EDMM\xb7EDMMALB\xb7000\xb7105:EDMM\xb7EDMMALB\xb7000\xb7105:EDMM\xb7EDMMRDG\xb7000\xb7135
DISPLAY:EDMM\xb7EDMMALB\xb7105\xb7135:EDMM\xb7EDMMALB\xb7105\xb7135:EDMM\xb7EDMMRDG\xb7000\xb7135
DISPLAY:EDMM\xb7EDMMALB\xb7135\xb7245:EDMM\xb7EDMMALB\xb7135\xb7245:EDMM\xb7EDMMRDG\xb7135\xb7245
DISPLAY:EDMM\xb7EDMMALB\xb7245\xb7315:EDMM\xb7EDMMALB\xb7245\xb7315:EDMM\xb7EDMMRDG\xb7245\xb7315
DISPLAY:EDMM\xb7EDMMRDG\xb7000\xb7135:EDMM\xb7EDMMRDG\xb7000\xb7135:EDMM\xb7EDMMALB\xb7000\xb7105
DISPLAY:EDMM\xb7EDMMRDG\xb7000\xb7135:EDMM\xb7EDMMRDG\xb7000\xb7135:EDMM\xb7EDMMALB\xb7105\xb7135
DISPLAY:EDMM\xb7EDMMRDG\xb7135\xb7245:EDMM\xb7EDMMRDG\xb7135\xb7245:EDMM\xb7EDMMALB\xb7135\xb7245
DISPLAY:EDMM\xb7EDMMRDG\xb7245\xb7315:EDMM\xb7EDMMRDG\xb7245\xb7315:EDMM\xb7EDMMALB\xb7245\xb7315

SECTORLINE:153
COORD:N048.40.03.000:E011.47.39.000
COORD:N048.40.04.000:E011.30.42.000
COORD:N048.40.04.000:E011.19.15.000
DISPLAY:EDMM\xb7EDMMALB\xb7000\xb7105:EDMM\xb7EDMMALB\xb7000\xb7105:EDMM\xb7EDMMTMANH\xb7095\xb7195
DISPLAY:EDMM\xb7EDMMALB\xb7000\xb7105:EDMM\xb7EDMMALB\xb7000\xb7105:EDMM\xb7EDMMTMANL\xb7000\xb7095
DISPLAY:EDMM\xb7EDMMTMANH\xb7095\xb7195:EDMM\xb7EDMMTMANH\xb7095\xb7195:EDMM\xb7EDMMALB\xb7000\xb7105
DISPLAY:EDMM\xb7EDMMTMANH\xb7095\xb7195:EDMM\xb7EDMMTMANH\xb7095\xb7195:EDMM\xb7EDMMALB\xb7105\xb7135
DISPLAY:EDMM\xb7EDMMTMANH\xb7095\xb7195:EDMM\xb7EDMMTMANH\xb7095\xb7195:EDMM\xb7EDMMALB\xb7135\xb7245
DISPLAY:EDMM\xb7EDMMTMANL\xb7000\xb7095:EDMM\xb7EDMMTMANL\xb7000\xb7095:EDMM\xb7EDMMALB\xb7000\xb7105
DISPLAY:EDMM\xb7EDMMALB\xb7105\xb7135:EDMM\xb7EDMMALB\xb7105\xb7135:EDMM\xb7EDMMTMANH\xb7095\xb7195
DISPLAY:EDMM\xb7EDMMALB\xb7135\xb7245:EDMM\xb7EDMMALB\xb7135\xb7245:EDMM\xb7EDMMTMANH\xb7095\xb7195
DISPLAY:EDMM\xb7EDMMALB\xb7135\xb7245:EDMM\xb7EDMMALB\xb7135\xb7245:EDMM\xb7EDMMNDG\xb7195\xb7245
DISPLAY:EDMM\xb7EDMMALB\xb7245\xb7315:EDMM\xb7EDMMALB\xb7245\xb7315:EDMM\xb7EDMMNDG\xb7245\xb7315
DISPLAY:EDMM\xb7EDMMNDG\xb7195\xb7245:EDMM\xb7EDMMNDG\xb7195\xb7245:EDMM\xb7EDMMALB\xb7135\xb7245
DISPLAY:EDMM\xb7EDMMNDG\xb7245\xb7315:EDMM\xb7EDMMNDG\xb7245\xb7315:EDMM\xb7EDMMALB\xb7245\xb7315

SECTORLINE:154
COORD:N048.40.04.000:E011.19.15.000
COORD:N049.07.10.000:E010.40.25.000
DISPLAY:EDMM\xb7EDMMALB\xb7000\xb7105:EDMM\xb7EDMMALB\xb7000\xb7105:EDMM\xb7EDMMWLD\xb7000\xb7105
DISPLAY:EDMM\xb7EDMMWLD\xb7000\xb7105:EDMM\xb7EDMMWLD\xb7000\xb7105:EDMM\xb7EDMMALB\xb7000\xb7105
DISPLAY:EDMM\xb7EDMMALB\xb7105\xb7135:EDMM\xb7EDMMALB\xb7105\xb7135:EDMM\xb7EDMMWLD\xb7105\xb7245
DISPLAY:EDMM\xb7EDMMALB\xb7135\xb7245:EDMM\xb7EDMMALB\xb7135\xb7245:EDMM\xb7EDMMWLD\xb7105\xb7245
DISPLAY:EDMM\xb7EDMMALB\xb7245\xb7315:EDMM\xb7EDMMALB\xb7245\xb7315:EDMM\xb7EDMMWLD\xb7245\xb7315
DISPLAY:EDMM\xb7EDMMWLD\xb7105\xb7245:EDMM\xb7EDMMWLD\xb7105\xb7245:EDMM\xb7EDMMALB\xb7105\xb7135
DISPLAY:EDMM\xb7EDMMWLD\xb7105\xb7245:EDMM\xb7EDMMWLD\xb7105\xb7245:EDMM\xb7EDMMALB\xb7135\xb7245
DISPLAY:EDMM\xb7EDMMWLD\xb7245\xb7315:EDMM\xb7EDMMWLD\xb7245\xb7315:EDMM\xb7EDMMALB\xb7245\xb7315

SECTORLINE:155
COORD:N049.07.10.000:E010.40.25.000
COORD:N049.08.17.000:E011.07.57.000
DISPLAY:EDMM\xb7EDMMALB\xb7000\xb7105:EDMM\xb7EDMMALB\xb7000\xb7105:EDMM\xb7EDMMFRK\xb7000\xb7135
DISPLAY:EDMM\xb7EDMMFRK\xb7000\xb7135:EDMM\xb7EDMMFRK\xb7000\xb7135:EDMM\xb7EDMMALB\xb7000\xb7105
DISPLAY:EDMM\xb7EDMMFRK\xb7000\xb7135:EDMM\xb7EDMMFRK\xb7000\xb7135:EDMM\xb7EDMMALB\xb7105\xb7135
DISPLAY:EDMM\xb7EDMMALB\xb7105\xb7135:EDMM\xb7EDMMALB\xb7105\xb7135:EDMM\xb7EDMMFRK\xb7000\xb7135

SECTORLINE:167
COORD:N049.07.10.000:E010.40.25.000
COORD:N049.10.43.000:E010.35.16.000
DISPLAY:EDMM\xb7EDMMFRK\xb7000\xb7135:EDMM\xb7EDMMFRK\xb7000\xb7135:EDMM\xb7EDMMWLD\xb7000\xb7105
DISPLAY:EDMM\xb7EDMMFRK\xb7000\xb7135:EDMM\xb7EDMMFRK\xb7000\xb7135:EDMM\xb7EDMMWLD\xb7105\xb7245
DISPLAY:EDMM\xb7EDMMWLD\xb7000\xb7105:EDMM\xb7EDMMWLD\xb7000\xb7105:EDMM\xb7EDMMFRK\xb7000\xb7135
DISPLAY:EDMM\xb7EDMMALB\xb7135\xb7245:EDMM\xb7EDMMALB\xb7135\xb7245:EDMM\xb7EDMMWLD\xb7105\xb7245
DISPLAY:EDMM\xb7EDMMALB\xb7245\xb7315:EDMM\xb7EDMMALB\xb7245\xb7315:EDMM\xb7EDMMWLD\xb7245\xb7315
DISPLAY:EDMM\xb7EDMMWLD\xb7105\xb7245:EDMM\xb7EDMMWLD\xb7105\xb7245:EDMM\xb7EDMMFRK\xb7000\xb7135
DISPLAY:EDMM\xb7EDMMWLD\xb7105\xb7245:EDMM\xb7EDMMWLD\xb7105\xb7245:EDMM\xb7EDMMALB\xb7135\xb7245
DISPLAY:EDMM\xb7EDMMWLD\xb7245\xb7315:EDMM\xb7EDMMWLD\xb7245\xb7315:EDMM\xb7EDMMALB\xb7245\xb7315
DISPLAY:EDMM\xb7EDUUDON14\xb7315\xb7355:EDMM\xb7EDUUDON14\xb7315\xb7355:EDMM\xb7EDUUISA14\xb7315\xb7355
DISPLAY:EDMM\xb7EDUUDON24\xb7355\xb7365:EDMM\xb7EDUUDON24\xb7355\xb7365:EDMM\xb7EDUUISA24\xb7355\xb7365
DISPLAY:EDMM\xb7EDUUDON34\xb7365\xb7375:EDMM\xb7EDUUDON34\xb7365\xb7375:EDMM\xb7EDUUISA34\xb7365\xb7375
DISPLAY:EDMM\xb7EDUUDON44\xb7375\xb7660:EDMM\xb7EDUUDON44\xb7375\xb7660:EDMM\xb7EDUUISA44\xb7375\xb7660
DISPLAY:EDMM\xb7EDUUISA14\xb7315\xb7355:EDMM\xb7EDUUISA14\xb7315\xb7355:EDMM\xb7EDUUDON14\xb7315\xb7355
DISPLAY:EDMM\xb7EDUUISA24\xb7355\xb7365:EDMM\xb7EDUUISA24\xb7355\xb7365:EDMM\xb7EDUUDON24\xb7355\xb7365
DISPLAY:EDMM\xb7EDUUISA34\xb7365\xb7375:EDMM\xb7EDUUISA34\xb7365\xb7375:EDMM\xb7EDUUDON34\xb7365\xb7375
DISPLAY:EDMM\xb7EDUUISA44\xb7375\xb7660:EDMM\xb7EDUUISA44\xb7375\xb7660:EDMM\xb7EDUUDON44\xb7375\xb7660

SECTORLINE:180
COORD:N049.26.04.000:E011.49.08.000
COORD:N049.26.30.000:E010.58.00.000
DISPLAY:EDMM\xb7EDMMFRK\xb7135\xb7195:EDMM\xb7EDMMFRK\xb7135\xb7195:EDMM\xb7EDMMALB\xb7135\xb7245
DISPLAY:EDMM\xb7EDMMALB\xb7135\xb7245:EDMM\xb7EDMMALB\xb7135\xb7245:EDMM\xb7EDMMFRK\xb7135\xb7195
DISPLAY:EDMM\xb7EDMMALB\xb7135\xb7245:EDMM\xb7EDMMALB\xb7135\xb7245:EDMM\xb7EDMMBBG\xb7195\xb7245
DISPLAY:EDMM\xb7EDMMALB\xb7245\xb7315:EDMM\xb7EDMMALB\xb7245\xb7315:EDMM\xb7EDMMBBG\xb7245\xb7295
DISPLAY:EDMM\xb7EDMMALB\xb7245\xb7315:EDMM\xb7EDMMALB\xb7245\xb7315:EDMM\xb7EDMMBBG\xb7295\xb7315
DISPLAY:EDMM\xb7EDMMBBG\xb7195\xb7245:EDMM\xb7EDMMBBG\xb7195\xb7245:EDMM\xb7EDMMALB\xb7135\xb7245
DISPLAY:EDMM\xb7EDMMBBG\xb7245\xb7295:EDMM\xb7EDMMBBG\xb7245\xb7295:EDMM\xb7EDMMALB\xb7245\xb7315
DISPLAY:EDMM\xb7EDMMBBG\xb7295\xb7315:EDMM\xb7EDMMBBG\xb7295\xb7315:EDMM\xb7EDMMALB\xb7245\xb7315
DISPLAY:EDMM\xb7EDUUDON14\xb7315\xb7355:EDMM\xb7EDUUDON14\xb7315\xb7355:EDMM\xb7EDUUERL12\xb7315\xb7365
DISPLAY:EDMM\xb7EDUUDON24\xb7355\xb7365:EDMM\xb7EDUUDON24\xb7355\xb7365:EDMM\xb7EDUUERL12\xb7315\xb7365
DISPLAY:EDMM\xb7EDUUDON34\xb7365\xb7375:EDMM\xb7EDUUDON34\xb7365\xb7375:EDMM\xb7EDUUERL22\xb7365\xb7660
DISPLAY:EDMM\xb7EDUUDON44\xb7375\xb7660:EDMM\xb7EDUUDON44\xb7375\xb7660:EDMM\xb7EDUUERL22\xb7365\xb7660
DISPLAY:EDMM\xb7EDUUERL12\xb7315\xb7365:EDMM\xb7EDUUERL12\xb7315\xb7365:EDMM\xb7EDUUDON14\xb7315\xb7355
DISPLAY:EDMM\xb7EDUUERL12\xb7315\xb7365:EDMM\xb7EDUUERL12\xb7315\xb7365:EDMM\xb7EDUUDON24\xb7355\xb7365
DISPLAY:EDMM\xb7EDUUERL22\xb7365\xb7660:EDMM\xb7EDUUERL22\xb7365\xb7660:EDMM\xb7EDUUDON34\xb7365\xb7375
DISPLAY:EDMM\xb7EDUUERL22\xb7365\xb7660:EDMM\xb7EDUUERL22\xb7365\xb7660:EDMM\xb7EDUUDON44\xb7375\xb7660

SECTORLINE:352
COORD:N049.10.43.000:E010.35.16.000
COORD:N049.26.30.000:E010.58.00.000
DISPLAY:EDGG\xb7KTG1\xb7135\xb7245:EDGG\xb7KTG1\xb7135\xb7245:EDMM\xb7EDMMALB\xb7135\xb7245
DISPLAY:EDGG\xb7WUR141\xb7305\xb7325:EDGG\xb7WUR141\xb7305\xb7325:EDMM\xb7EDMMALB\xb7245\xb7315
DISPLAY:EDGG\xb7WUR141\xb7305\xb7325:EDGG\xb7WUR141\xb7305\xb7325:EDMM\xb7EDUUDON14\xb7315\xb7355
DISPLAY:EDGG\xb7WUR24\xb7325\xb7355:EDGG\xb7WUR24\xb7325\xb7355:EDMM\xb7EDUUDON14\xb7315\xb7355
DISPLAY:EDGG\xb7WUR34\xb7355\xb7375:EDGG\xb7WUR34\xb7355\xb7375:EDMM\xb7EDUUDON24\xb7355\xb7365
DISPLAY:EDGG\xb7WUR34\xb7355\xb7375:EDGG\xb7WUR34\xb7355\xb7375:EDMM\xb7EDUUDON34\xb7365\xb7375
DISPLAY:EDGG\xb7WUR44\xb7375\xb7660:EDGG\xb7WUR44\xb7375\xb7660:EDMM\xb7EDUUDON44\xb7375\xb7660
DISPLAY:EDMM\xb7EDMMALB\xb7135\xb7245:EDMM\xb7EDMMALB\xb7135\xb7245:EDGG\xb7KTG1\xb7135\xb7245
DISPLAY:EDMM\xb7EDMMALB\xb7245\xb7305:EDMM\xb7EDMMALB\xb7245\xb7305:EDMM\xb7EDMMALB\xb7245\xb7315
DISPLAY:EDMM\xb7EDMMALB\xb7245\xb7315:EDMM\xb7EDMMALB\xb7245\xb7315:EDGG\xb7WUR141\xb7305\xb7325
DISPLAY:EDMM\xb7EDMMALB\xb7245\xb7315:EDMM\xb7EDMMALB\xb7245\xb7315:EDMM\xb7EDMMALB\xb7245\xb7305
DISPLAY:EDMM\xb7EDUUDON14\xb7315\xb7355:EDMM\xb7EDUUDON14\xb7315\xb7355:EDGG\xb7WUR141\xb7305\xb7325
DISPLAY:EDMM\xb7EDUUDON14\xb7315\xb7355:EDMM\xb7EDUUDON14\xb7315\xb7355:EDGG\xb7WUR24\xb7325\xb7355
DISPLAY:EDMM\xb7EDUUDON24\xb7355\xb7365:EDMM\xb7EDUUDON24\xb7355\xb7365:EDGG\xb7WUR34\xb7355\xb7375
DISPLAY:EDMM\xb7EDUUDON34\xb7365\xb7375:EDMM\xb7EDUUDON34\xb7365\xb7375:EDGG\xb7WUR34\xb7355\xb7375
DISPLAY:EDMM\xb7EDUUDON44\xb7375\xb7660:EDMM\xb7EDUUDON44\xb7375\xb7660:EDGG\xb7WUR44\xb7375\xb7660

SECTORLINE:363
COORD:N049.26.30.000:E010.58.00.000
COORD:N049.39.17.000:E010.45.56.000
DISPLAY:EDGG\xb7WUR141\xb7305\xb7325:EDGG\xb7WUR141\xb7305\xb7325:EDMM\xb7EDMMBBG\xb7295\xb7315
DISPLAY:EDGG\xb7WUR141\xb7305\xb7325:EDGG\xb7WUR141\xb7305\xb7325:EDMM\xb7EDUUERL12\xb7315\xb7365
DISPLAY:EDGG\xb7WUR24\xb7325\xb7355:EDGG\xb7WUR24\xb7325\xb7355:EDMM\xb7EDUUERL12\xb7315\xb7365
DISPLAY:EDGG\xb7WUR34\xb7355\xb7375:EDGG\xb7WUR34\xb7355\xb7375:EDMM\xb7EDUUERL12\xb7315\xb7365
DISPLAY:EDGG\xb7WUR34\xb7355\xb7375:EDGG\xb7WUR34\xb7355\xb7375:EDMM\xb7EDUUERL22\xb7365\xb7660
DISPLAY:EDGG\xb7WUR44\xb7375\xb7660:EDGG\xb7WUR44\xb7375\xb7660:EDMM\xb7EDUUERL22\xb7365\xb7660
DISPLAY:EDMM\xb7EDMMALB\xb7245\xb7305:EDMM\xb7EDMMALB\xb7245\xb7305:EDMM\xb7EDMMBBG\xb7245\xb7295
DISPLAY:EDMM\xb7EDMMALB\xb7245\xb7305:EDMM\xb7EDMMALB\xb7245\xb7305:EDMM\xb7EDMMBBG\xb7295\xb7315
DISPLAY:EDMM\xb7EDMMBBG\xb7245\xb7295:EDMM\xb7EDMMBBG\xb7245\xb7295:EDMM\xb7EDMMALB\xb7245\xb7305
DISPLAY:EDMM\xb7EDMMBBG\xb7295\xb7315:EDMM\xb7EDMMBBG\xb7295\xb7315:EDGG\xb7WUR141\xb7305\xb7325
DISPLAY:EDMM\xb7EDMMBBG\xb7295\xb7315:EDMM\xb7EDMMBBG\xb7295\xb7315:EDMM\xb7EDMMALB\xb7245\xb7305
DISPLAY:EDMM\xb7EDUUERL12\xb7315\xb7365:EDMM\xb7EDUUERL12\xb7315\xb7365:EDGG\xb7WUR141\xb7305\xb7325
DISPLAY:EDMM\xb7EDUUERL12\xb7315\xb7365:EDMM\xb7EDUUERL12\xb7315\xb7365:EDGG\xb7WUR24\xb7325\xb7355
DISPLAY:EDMM\xb7EDUUERL12\xb7315\xb7365:EDMM\xb7EDUUERL12\xb7315\xb7365:EDGG\xb7WUR34\xb7355\xb7375
DISPLAY:EDMM\xb7EDUUERL22\xb7365\xb7660:EDMM\xb7EDUUERL22\xb7365\xb7660:EDGG\xb7WUR34\xb7355\xb7375
DISPLAY:EDMM\xb7EDUUERL22\xb7365\xb7660:EDMM\xb7EDUUERL22\xb7365\xb7660:EDGG\xb7WUR44\xb7375\xb7660

SECTORLINE:365
COORD:N049.10.43.000:E010.35.16.000
COORD:N049.17.00.000:E010.27.30.000
COORD:N049.23.54.000:E010.21.22.000
COORD:N049.30.40.000:E010.30.12.000
COORD:N049.39.17.000:E010.45.56.000
DISPLAY:EDGG\xb7WUR142\xb7245\xb7305:EDGG\xb7WUR142\xb7245\xb7305:EDMM\xb7EDMMALB\xb7245\xb7305
DISPLAY:EDMM\xb7EDMMALB\xb7245\xb7305:EDMM\xb7EDMMALB\xb7245\xb7305:EDGG\xb7WUR142\xb7245\xb7305

SECTORLINE:366
COORD:N049.10.00.000:E011.58.00.000
COORD:N049.26.04.000:E011.49.08.000
DISPLAY:EDMM\xb7EDMMALB\xb7135\xb7245:EDMM\xb7EDMMALB\xb7135\xb7245:EDMM\xb7EDMMRDG\xb7135\xb7245
DISPLAY:EDMM\xb7EDMMALB\xb7245\xb7315:EDMM\xb7EDMMALB\xb7245\xb7315:EDMM\xb7EDMMRDG\xb7245\xb7315
DISPLAY:EDMM\xb7EDMMRDG\xb7135\xb7245:EDMM\xb7EDMMRDG\xb7135\xb7245:EDMM\xb7EDMMALB\xb7135\xb7245
DISPLAY:EDMM\xb7EDMMRDG\xb7245\xb7315:EDMM\xb7EDMMRDG\xb7245\xb7315:EDMM\xb7EDMMALB\xb7245\xb7315

SECTOR:EDMM\xb7EDMMALB\xb7000\xb7105:00000:10500
OWNER:SWA:ALB:WLD:RDG:EGG:ZUG:MMC
BORDER:152:153:154:155:109
DEPAPT:ETSN:ETSI
ARRAPT:ETSN:ETSI

SECTOR:EDMM\xb7EDMMALB\xb7105\xb7135:10500:13500
OWNER:ALB:WLD:RDG:EGG:ZUG:MMC
BORDER:152:153:154:155:109

SECTOR:EDMM\xb7EDMMALB\xb7135\xb7245:13500:24500
OWNER:ALB:WLD:RDG:EGG:ZUG:MMC
BORDER:352:180:366:152:153:154:167

SECTOR:EDMM\xb7EDMMALB\xb7245\xb7305:24500:30500
OWNER:ALB:WLD:RDG:EGG:ZUG:MMC:WUR:SLN:DKB:KTG:BAD
BORDER:365:363:352

SECTOR:EDMM\xb7EDMMALB\xb7245\xb7315:24500:31500
OWNER:ALB:WLD:RDG:EGG:ZUG:MMC
BORDER:352:180:366:152:153:154:167

COPX:*:*:RUDNO:ETSI:*:EDMM\xb7EDMMRDG\xb7000\xb7135:EDMM\xb7EDMMALB\xb7000\xb7105:*:9000:RUDNO
COPX:*:*:*:EDMO:*:EDMM\xb7EDMMALB\xb7000\xb7105:EDMM\xb7EDMMTMANL\xb7000\xb7095:*:9300:INDIV
COPX:EDMA:*:MIQ:*:*:EDMM\xb7EDMMTMANH\xb7095\xb7195:EDMM\xb7EDMMALB\xb7135\xb7245:19000:*:MIQ
COPX:*:*:UPALA:EDDN:*:EDMM\xb7EDMMALB\xb7105\xb7135:EDMM\xb7EDMMFRK\xb7000\xb7135:*:13300:UPALA|
COPX:*:*:UPALA:EDDE:*:EDMM\xb7EDMMALB\xb7245\xb7315:EDMM\xb7EDMMBBG\xb7245\xb7295:*:25000:UPALA
COPX:EDDM:*:UPALA:TENLO:*:EDMM\xb7EDMMALB\xb7245\xb7315:EDMM\xb7EDMMBBG\xb7245\xb7295:25000:*:UPALA
COPX:EDDM:*:UPALA:RODOG:*:EDMM\xb7EDMMALB\xb7245\xb7315:EDMM\xb7EDMMBBG\xb7245\xb7295:25000:*:UPALA
COPX:EDDM:*:UPALA:KEMES:*:EDMM\xb7EDMMALB\xb7245\xb7315:EDMM\xb7EDMMBBG\xb7245\xb7295:25000:*:UPALA
COPX:EDDM:*:DOSIS:*:*:EDMM\xb7EDMMALB\xb7245\xb7315:EDMM\xb7EDMMBBG\xb7245\xb7295:25000:*:DOSIS
COPX:EDDM:*:EVIVA:*:*:EDMM\xb7EDMMTMANL\xb7000\xb7095:EDMM\xb7EDMMALB\xb7135\xb7245:19000:*:EVIVA
COPX:EDDM:*:GIVMI:*:*:EDMM\xb7EDMMTMANL\xb7000\xb7095:EDMM\xb7EDMMALB\xb7135\xb7245:19000:*:GIVMI
COPX:EDDM:*:INPUD:*:*:EDMM\xb7EDMMTMANL\xb7000\xb7095:EDMM\xb7EDMMALB\xb7135\xb7245:19000:*:INPUD
COPX:*:*:MIQ:EDMA:*:EDMM\xb7EDMMALB\xb7000\xb7105:EDMM\xb7EDMMTMANL\xb7000\xb7095:*:8000:MIQ
COPX:EDDM:*:GOLMO:*:*:EDMM\xb7EDMMALB\xb7135\xb7245:EDMM\xb7EDMMWLD\xb7105\xb7245:24000:*:GOLMO
COPX:*:*:STAUB:ETSI:*:EDMM\xb7EDMMRDG\xb7000\xb7135:EDMM\xb7EDMMALB\xb7000\xb7105:*:10000:STAUB
COPX:*:*:EVIVA:EDDN:*:EDMM\xb7EDMMNDG\xb7195\xb7245:EDMM\xb7EDMMALB\xb7135\xb7245:*:20000:EVIVA
COPX:*:*:DOSIS:EDDN:*:EDMM\xb7EDMMALB\xb7135\xb7245:EDMM\xb7EDMMFRK\xb7000\xb7135:*:13300:DOSIS|
COPX:EDDM:*:MIQ:*:*:EDMM\xb7EDMMTMANL\xb7000\xb7095:EDMM\xb7EDMMALB\xb7135\xb7245:19000:*:MIQ
COPX:*:*:UPALA:EDDN:*:EDMM\xb7EDMMALB\xb7135\xb7245:EDMM\xb7EDMMFRK\xb7135\xb7195:*:14000:UPALA|
COPX:*:*:*:EDMO:*:EDMM\xb7EDMMALB\xb7135\xb7245:EDMM\xb7EDMMTMANH\xb7095\xb7195:*:19300:INDIV
COPX:ETSI:*:UPALA:*:*:EDMM\xb7EDMMALB\xb7135\xb7245:EDMM\xb7EDMMBBG\xb7195\xb7245:20000:*:UPALA
COPX:EDMO:*:BESNI:*:*:EDMM\xb7EDMMNDG\xb7195\xb7245:EDMM\xb7EDMMALB\xb7135\xb7245:20000:*:BESNI
COPX:*:*:MAH:EDDN:*:EDMM\xb7EDMMNDG\xb7195\xb7245:EDMM\xb7EDMMALB\xb7135\xb7245:*:20000:MAH
COPX:*:*:AKINI:EDDN:*:EDMM\xb7EDMMRDG\xb7135\xb7245:EDMM\xb7EDMMALB\xb7135\xb7245:*:21000:AKINI
FIR_COPX:*:*:DAMJA:EDDF:*:EDMM\xb7EDMMALB\xb7135\xb7245:EDGG\xb7KTG1\xb7135\xb7245:*:24000:DAMJA
FIR_COPX:*:*:ASPAT:EDDF:*:EDMM\xb7EDMMALB\xb7135\xb7245:EDGG\xb7KTG1\xb7135\xb7245:*:24000:DEBHI@
COPX:*:*:UPALA:EDDE:*:EDMM\xb7EDMMALB\xb7245\xb7305:EDMM\xb7EDMMBBG\xb7245\xb7295:*:25000:UPALA
COPX:EDDM:*:DOSIS:*:*:EDMM\xb7EDMMALB\xb7245\xb7305:EDMM\xb7EDMMBBG\xb7245\xb7295:25000:*:DOSIS
COPX:EDDM:*:UPALA:*:*:EDMM\xb7EDMMALB\xb7245\xb7305:EDMM\xb7EDMMBBG\xb7245\xb7295:25000:*:UPALA
FIR_COPX:LKKV:*:ALAXA:*:*:EDMM\xb7EDMMALB\xb7245\xb7305:EDGG\xb7WUR142\xb7245\xb7305:26000:*:ALAXA
FIR_COPX:EDQD:*:ALAXA:*:*:EDMM\xb7EDMMALB\xb7245\xb7305:EDGG\xb7WUR142\xb7245\xb7305:26000:*:ALAXA
FIR_COPX:EDQM:*:ALAXA:*:*:EDMM\xb7EDMMALB\xb7245\xb7305:EDGG\xb7WUR142\xb7245\xb7305:26000:*:ALAXA
FIR_COPX:LKKV:*:AMOSA:*:*:EDMM\xb7EDMMALB\xb7245\xb7305:EDGG\xb7WUR142\xb7245\xb7305:26000:*:AMOSA
FIR_COPX:EDQD:*:AMOSA:*:*:EDMM\xb7EDMMALB\xb7245\xb7305:EDGG\xb7WUR142\xb7245\xb7305:26000:*:AMOSA
FIR_COPX:EDQM:*:AMOSA:*:*:EDMM\xb7EDMMALB\xb7245\xb7305:EDGG\xb7WUR142\xb7245\xb7305:26000:*:AMOSA
FIR_COPX:*:*:PETIX:EDSB:*:EDMM\xb7EDMMALB\xb7245\xb7305:EDGG\xb7WUR142\xb7245\xb7305:*:28000:PETIX
FIR_COPX:*:*:PETIX: EDDR:*:EDMM\xb7EDMMALB\xb7245\xb7305:EDGG\xb7WUR142\xb7245\xb7305:*:28000:PETIX
FIR_COPX:*:*:PETIX: EDRZ:*:EDMM\xb7EDMMALB\xb7245\xb7305:EDGG\xb7WUR142\xb7245\xb7305:*:28000:PETIX
COPX:*:*:UPALA:EDDP:*:EDMM\xb7EDMMALB\xb7245\xb7305:EDMM\xb7EDMMBBG\xb7245\xb7295:*:29000:UPALA
FIR_COPX:EDDM:*:INBED:*:*:EDMM\xb7EDMMALB\xb7245\xb7305:EDGG\xb7WUR142\xb7245\xb7305:30000:*:INBED^
FIR_COPX:EDMA:*:INBED:*:*:EDMM\xb7EDMMALB\xb7245\xb7305:EDGG\xb7WUR142\xb7245\xb7305:30000:*:INBED^
FIR_COPX:EDMO:*:INBED:*:*:EDMM\xb7EDMMALB\xb7245\xb7305:EDGG\xb7WUR142\xb7245\xb7305:30000:*:INBED^
FIR_COPX:EDMS:*:INBED:*:*:EDMM\xb7EDMMALB\xb7245\xb7305:EDGG\xb7WUR142\xb7245\xb7305:30000:*:INBED 
COPX:LOWL:*:AKINI:*:*:EDMM\xb7EDMMRDG\xb7245\xb7315:EDMM\xb7EDMMALB\xb7245\xb7305:30000:*:AKINI
COPX:LOWS:*:AKINI:*:*:EDMM\xb7EDMMRDG\xb7245\xb7315:EDMM\xb7EDMMALB\xb7245\xb7305:30000:*:AKINI
COPX:AKINI:*:TALAL:EDDF:*:EDMM\xb7EDUUDON14\xb7315\xb7355:EDMM\xb7EDMMALB\xb7245\xb7315:*:31300:TALAL
COPX:AKINI:*:TALAL:ETOU:*:EDMM\xb7EDUUDON14\xb7315\xb7355:EDMM\xb7EDMMALB\xb7245\xb7315:*:31300:TALAL
COPX:AKINI:*:TALAL:EDFE:*:EDMM\xb7EDUUDON14\xb7315\xb7355:EDMM\xb7EDMMALB\xb7245\xb7315:*:31300:TALAL
COPX:AKINI:*:TALAL:EDFZ :*:EDMM\xb7EDUUDON14\xb7315\xb7355:EDMM\xb7EDMMALB\xb7245\xb7315:*:31300:TALAL
COPX:ERNAS:*:TALAL:EDDF:*:EDMM\xb7EDUUDON14\xb7315\xb7355:EDMM\xb7EDMMALB\xb7245\xb7315:*:31300:TALAL
COPX:ERNAS:*:TALAL:ETOU:*:EDMM\xb7EDUUDON14\xb7315\xb7355:EDMM\xb7EDMMALB\xb7245\xb7315:*:31300:TALAL
COPX:ERNAS:*:TALAL:EDFE:*:EDMM\xb7EDUUDON14\xb7315\xb7355:EDMM\xb7EDMMALB\xb7245\xb7315:*:31300:TALAL
COPX:ERNAS:*:TALAL:EDFZ :*:EDMM\xb7EDUUDON14\xb7315\xb7355:EDMM\xb7EDMMALB\xb7245\xb7315:*:31300:TALAL
COPX:*:*:UPALA:EDDP:*:EDMM\xb7EDUUDON14\xb7315\xb7355:EDMM\xb7EDMMALB\xb7245\xb7315:*:31300:10UPALA
COPX:*:*:UPALA:EDAC:*:EDMM\xb7EDUUDON14\xb7315\xb7355:EDMM\xb7EDMMALB\xb7245\xb7315:*:31300:10UPALA
COPX:*:*:UPALA:EDDE:*:EDMM\xb7EDUUDON14\xb7315\xb7355:EDMM\xb7EDMMALB\xb7245\xb7315:*:31300:10UPALA
COPX:*:*:UPALA:EDFQ:*:EDMM\xb7EDUUDON14\xb7315\xb7355:EDMM\xb7EDMMALB\xb7245\xb7315:*:31300:10UPALA
COPX:*:*:UPALA:EDVK:*:EDMM\xb7EDUUDON14\xb7315\xb7355:EDMM\xb7EDMMALB\xb7245\xb7315:*:31300:10UPALA
COPX:*:*:UNKUL:EDSB:*:EDMM\xb7EDUUDON14\xb7315\xb7355:EDMM\xb7EDMMALB\xb7245\xb7315:*:31300:UPALA
COPX:*:*:UNKUL:EDDR:*:EDMM\xb7EDUUDON14\xb7315\xb7355:EDMM\xb7EDMMALB\xb7245\xb7315:*:31300:UPALA
COPX:*:*:UNKUL:EDRZ:*:EDMM\xb7EDUUDON14\xb7315\xb7355:EDMM\xb7EDMMALB\xb7245\xb7315:*:31300:UPALA
COPX:*:*:ERMEL:EDDF:*:EDMM\xb7EDUUDON14\xb7315\xb7355:EDMM\xb7EDMMALB\xb7245\xb7315:*:31300:10ERMEL
COPX:*:*:ERNAS:EDDF:*:EDMM\xb7EDUUDON14\xb7315\xb7355:EDMM\xb7EDMMALB\xb7245\xb7315:*:31300:GOLMO
        ";

        let ese = Ese::parse(ese_bytes).unwrap();
        let alb_0_105 = ese
            .sectors
            .get("EDMM\u{b7}EDMMALB\u{b7}000\u{b7}105")
            .unwrap();
        assert_eq!(
            alb_0_105.id,
            "EDMM\u{b7}EDMMALB\u{b7}000\u{b7}105".to_string()
        );
        assert_eq!(alb_0_105.top, 10500);
        assert_eq!(alb_0_105.bottom, 0);
        assert_eq!(
            ese.agreements
                .iter()
                .filter(|agreement| agreement.entry_sector
                    == *"EDMM\u{b7}EDMMALB\u{b7}000\u{b7}105")
                .collect::<Vec<_>>(),
            vec![
                &Agreement {
                    previous_fix: None,
                    departure_runway: None,
                    fix: Some("RUDNO".to_string()),
                    subsequent_fix: Some("ETSI".to_string()),
                    arrival_runway: None,
                    exit_sector: "EDMM\u{b7}EDMMRDG\u{b7}000\u{b7}135".to_string(),
                    entry_sector: "EDMM\u{b7}EDMMALB\u{b7}000\u{b7}105".to_string(),
                    climb_level: None,
                    descent_level: Some(9000),
                    description: "RUDNO".to_string(),
                },
                &Agreement {
                    previous_fix: None,
                    departure_runway: None,
                    fix: Some("STAUB".to_string()),
                    subsequent_fix: Some("ETSI".to_string()),
                    arrival_runway: None,
                    exit_sector: "EDMM\u{b7}EDMMRDG\u{b7}000\u{b7}135".to_string(),
                    entry_sector: "EDMM\u{b7}EDMMALB\u{b7}000\u{b7}105".to_string(),
                    climb_level: None,
                    descent_level: Some(10000),
                    description: "STAUB".to_string(),
                },
            ]
        );
        assert_eq!(
            ese.agreements
                .iter()
                .filter(|agreement| agreement.exit_sector == *"EDMM\u{b7}EDMMALB\u{b7}000\u{b7}105")
                .collect::<Vec<_>>(),
            vec![
                &Agreement {
                    previous_fix: None,
                    departure_runway: None,
                    fix: None,
                    subsequent_fix: Some("EDMO".to_string()),
                    arrival_runway: None,
                    exit_sector: "EDMM\u{b7}EDMMALB\u{b7}000\u{b7}105".to_string(),
                    entry_sector: "EDMM\u{b7}EDMMTMANL\u{b7}000\u{b7}095".to_string(),
                    climb_level: None,
                    descent_level: Some(9300),
                    description: "INDIV".to_string(),
                },
                &Agreement {
                    previous_fix: None,
                    departure_runway: None,
                    fix: Some("MIQ".to_string()),
                    subsequent_fix: Some("EDMA".to_string()),
                    arrival_runway: None,
                    exit_sector: "EDMM\u{b7}EDMMALB\u{b7}000\u{b7}105".to_string(),
                    entry_sector: "EDMM\u{b7}EDMMTMANL\u{b7}000\u{b7}095".to_string(),
                    climb_level: None,
                    descent_level: Some(8000),
                    description: "MIQ".to_string(),
                },
            ]
        );
        assert_eq!(
            alb_0_105.border,
            vec![
                SectorLine {
                    points: line_string![
                        (
                            x: 11.794_166_666_666_667,
                            y: 48.6675,
                        ),
                        (
                            x: 11.966_666_666_666_667,
                            y: 49.166_666_666_666_664,
                        )
                    ]
                },
                SectorLine {
                    points: line_string![
                        (
                            x: 11.794_166_666_666_667,
                            y: 48.6675,
                        ),
                        (
                            x: 11.511_666_666_666_667,
                            y: 48.667_777_777_777_77,
                        ),
                        (
                            x: 11.320_833_333_333_333,
                            y: 48.667_777_777_777_77,
                        )
                    ]
                },
                SectorLine {
                    points: line_string![
                        (
                            x: 11.320_833_333_333_333,
                            y: 48.667_777_777_777_77,
                        ),
                        (
                            x: 10.673_611_111_111_11,
                            y: 49.119_444_444_444_45,
                        )
                    ]
                },
                SectorLine {
                    points: line_string![
                        (
                            x: 10.673_611_111_111_11,
                            y: 49.119_444_444_444_45,
                        ),
                        (
                            x: 11.1325,
                            y: 49.138_055_555_555_55,
                        )
                    ]
                },
                SectorLine {
                    points: line_string![
                        (
                            x: 11.1325,
                            y: 49.138_055_555_555_55,
                        ),
                        (
                            x: 11.966_666_666_666_667,
                            y: 49.166_666_666_666_664,
                        )
                    ]
                }
            ]
        );
    }

    #[test]
    fn test_ese_sids_stars() {
        let ese_bytes = b"
[POSITIONS]
EDDM_ATIS:Muenchen ATIS:123.130:MX::EDDM:ATIS:::0000:0000
EDMM_ALB_CTR:Muenchen Radar:129.100:ALB:ALB:EDMM:CTR:::2354:2367:N049.02.24.501:E012.31.35.850
EDMM_TEG_CTR:Muenchen Radar:133.680:TEG:TEG:EDMM:CTR:::2354:2367:N048.10.49.419:E011.48.59.530
EDXX_FIS_CTR:Langen Information:128.950:GIXX:FIS:EDXX:CTR:::2001:2577:N049.26.51.334:E010.13.06.336:N052.28.08.891:E010.52.12.796
EGBB_ATIS:Birmingham ATIS:136.030:BBI:B:EGBB:ATIS:-:-::

[SIDSSTARS]
STAR:EDJA:06:KPT1C:KPT JA450 JA430 JA060 FIMPE
SID:EDDM:26R:GIVMI1N:DM060 DM063 GIVMI
";

        let ese = Ese::parse(ese_bytes).unwrap();
        assert_eq!(
            *ese.sids_stars,
            vec![
                SidStar::Star(STAR {
                    name: "KPT1C".to_string(),
                    airport: "EDJA".to_string(),
                    runway: Some("06".to_string()),
                    waypoints: vec![
                        "KPT".to_string(),
                        "JA450".to_string(),
                        "JA430".to_string(),
                        "JA060".to_string(),
                        "FIMPE".to_string()
                    ]
                }),
                SidStar::Sid(SID {
                    name: "GIVMI1N".to_string(),
                    airport: "EDDM".to_string(),
                    runway: Some("26R".to_string()),
                    waypoints: vec![
                        "DM060".to_string(),
                        "DM063".to_string(),
                        "GIVMI".to_string()
                    ]
                })
            ]
        );
    }
}
