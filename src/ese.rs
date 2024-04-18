use std::collections::HashMap;
use std::io;

use bevy_reflect::Reflect;
use pest::{iterators::Pair, Parser};
use pest_derive::Parser;
use serde::Serialize;
use thiserror::Error;

use super::{read_to_string, Coordinate, DegMinSec};

#[derive(Parser)]
#[grammar = "ese.pest"]
pub struct EseParser;

#[derive(Error, Debug)]
pub enum EseError {
    #[error("failed to parse .ese file: {0:?}")]
    Parse(#[from] pest::error::Error<Rule>),
    #[error("failed to read .ese file: {0:?}")]
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
    pub visibility_points: Vec<Coordinate>,
}

#[derive(Clone, Debug, Reflect, Serialize, PartialEq)]
pub struct MSAW {
    pub altitude: u32,
    pub points: Vec<Coordinate>,
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

#[derive(Clone, Debug, Reflect, Serialize, PartialEq)]
pub struct SectorLine {
    pub points: Vec<Coordinate>,
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

#[derive(Clone, Debug, Reflect, Serialize, PartialEq)]
pub struct Sector {
    pub id: String,
    pub bottom: u32,
    pub top: u32,
    pub border: Vec<SectorLine>,
    pub owner_priority: Vec<String>,
    pub departure_airports: Vec<String>,
    pub arrival_airports: Vec<String>,
    pub copns: Vec<Cop>,
    pub copxs: Vec<Cop>,
    pub fir_copns: Vec<Cop>,
    pub fir_copxs: Vec<Cop>,
    // TODO?
    // pub alt_owner_priority: Vec<String>,
    // pub guest: Vec<String>,
    // TODO
    // airport, runway
    // pub active: (String, String),
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

        (
            border,
            Self {
                id,
                bottom,
                top,
                owner_priority,
                departure_airports,
                arrival_airports,
                // replaced later on
                border: vec![],
                copns: vec![],
                copxs: vec![],
                fir_copns: vec![],
                fir_copxs: vec![],
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
            rule => {
                eprintln!("{rule:?}");
                unreachable!()
            }
        }
    }
}

fn parse_wildcard_string(pair: Pair<Rule>) -> Option<String> {
    match pair.as_rule() {
        Rule::wildcard => None,
        Rule::text | Rule::designator => Some(pair.as_str().to_string()),
        rule => {
            eprintln!("{rule:?}");
            unreachable!()
        }
    }
}

fn parse_wildcard_u32(pair: Pair<Rule>) -> Option<u32> {
    match pair.as_rule() {
        Rule::wildcard => None,
        Rule::INTEGER => Some(pair.as_str().parse().unwrap()),
        rule => {
            eprintln!("{rule:?}");
            unreachable!()
        }
    }
}

#[derive(Clone, Debug, Reflect, Serialize, PartialEq, Eq)]
pub struct Cop {
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
impl Cop {
    fn parse(pair: Pair<Rule>) -> Self {
        let mut cop = pair.into_inner();
        let previous_fix = parse_wildcard_string(cop.next().unwrap());
        let departure_runway = parse_wildcard_string(cop.next().unwrap());
        let fix = parse_wildcard_string(cop.next().unwrap());
        let subsequent_fix = parse_wildcard_string(cop.next().unwrap());
        let arrival_runway = parse_wildcard_string(cop.next().unwrap());
        let exit_sector = cop.next().unwrap().as_str().to_string();
        let entry_sector = cop.next().unwrap().as_str().to_string();
        let climb_level = parse_wildcard_u32(cop.next().unwrap());
        let descent_level = parse_wildcard_u32(cop.next().unwrap());
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

#[derive(Debug, Default, Reflect, Serialize, PartialEq)]
pub struct Ese {
    pub positions: HashMap<String, Position>,
    pub sectors: HashMap<String, Sector>,
}

pub type EseResult = Result<Ese, EseError>;

#[derive(Debug)]
enum Section {
    Positions(HashMap<String, Position>),
    Sectors(HashMap<String, Sector>),
    Unsupported,
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum SectionName {
    Position,
    Airspace,
    Unsupported,
}

fn parse_coordinate_part(pair: Pair<Rule>) -> DegMinSec {
    let mut coordinate_part = pair.into_inner();
    let hemi = coordinate_part.next().unwrap().as_str();
    let degrees = coordinate_part
        .next()
        .unwrap()
        .as_str()
        .parse::<f64>()
        .unwrap()
        * match hemi {
            "N" | "E" | "n" | "e" => 1.0,
            "S" | "W" | "s" | "w" => -1.0,
            _ => unreachable!(),
        };
    let min = coordinate_part.next().unwrap().as_str().parse().unwrap();
    let sec = coordinate_part.next().unwrap().as_str().parse().unwrap();

    (degrees, min, sec)
}

// TODO generalise this and other similar into trait
fn parse_coordinate(pair: Pair<Rule>) -> Coordinate {
    let mut coordinate = pair.into_inner();
    let lat = parse_coordinate_part(coordinate.next().unwrap());
    let lng = parse_coordinate_part(coordinate.next().unwrap());
    Coordinate::from_deg_min_sec(lat, lng)
}

enum SectorRule {
    Sector((Vec<String>, Sector)),
    SectorLine((String, SectorLine)),
    FirCop(Cop),
    Cop(Cop),
    CircleSectorLine((String, CircleSectorLine)),
    DisplaySectorline,
    MSAW((String, MSAW)),
}

fn parse_airspace(pair: Pair<Rule>) -> SectorRule {
    match pair.as_rule() {
        Rule::sectorline => SectorRule::SectorLine(SectorLine::parse(pair)),
        Rule::sector => SectorRule::Sector(Sector::parse(pair)),
        Rule::cop => SectorRule::Cop(Cop::parse(pair)),
        Rule::fir_cop => SectorRule::FirCop(Cop::parse(pair)),
        Rule::display_sectorline => SectorRule::DisplaySectorline,
        Rule::circle_sectorline => SectorRule::CircleSectorLine(CircleSectorLine::parse(pair)),
        Rule::msaw => SectorRule::MSAW(MSAW::parse(pair)),
        rule => {
            eprintln!("{rule:?}");
            unreachable!()
        }
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
);
fn collect_sectors(
    (mut sectors, mut circle_sector_lines, mut sector_lines, mut borders): SectorsLinesBorders,
    rule: SectorRule,
) -> SectorsLinesBorders {
    match rule {
        SectorRule::Cop(cop) => {
            if let Some(sct) = sectors.get_mut(&cop.entry_sector) {
                sct.copns.push(cop.clone());
            }
            if let Some(sct) = sectors.get_mut(&cop.exit_sector) {
                sct.copxs.push(cop);
            }
        }
        SectorRule::FirCop(cop) => {
            if let Some(sct) = sectors.get_mut(&cop.entry_sector) {
                sct.fir_copns.push(cop.clone());
            }
            if let Some(sct) = sectors.get_mut(&cop.exit_sector) {
                sct.fir_copxs.push(cop);
            }
        }
        SectorRule::SectorLine((id, sector_line)) => {
            if let Some(_overwritten) = sector_lines.insert(id.clone(), sector_line) {
                eprintln!("duplicate sector_line: {id}");
            }
        }
        SectorRule::CircleSectorLine((id, circle_sector_line)) => {
            if let Some(_overwritten) = circle_sector_lines.insert(id.clone(), circle_sector_line) {
                eprintln!("duplicate cirle_sector_line: {id}");
            }
        }
        SectorRule::Sector((border, sector)) => {
            borders.insert(sector.id.clone(), border);
            if let Some(overwritten) = sectors.insert(sector.id.clone(), sector) {
                eprintln!("duplicate sector: {}", overwritten.id);
            }
        }
        // TODO
        SectorRule::MSAW(_) => (),
        SectorRule::DisplaySectorline => (),
    }
    (sectors, circle_sector_lines, sector_lines, borders)
}

fn combine_sectors_with_borders(
    (sectors, _circle_sector_lines, lines, borders): SectorsLinesBorders,
) -> HashMap<String, Sector> {
    // TODO circle_sector_lines
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
        .collect()
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
                            ),
                            collect_sectors,
                        ),
                )
            }),
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
        let sectors = match sections.remove_entry(&SectionName::Airspace) {
            Some((_, Section::Sectors(sectors))) => sectors,
            _ => HashMap::new(),
        };

        Ok(Ese { positions, sectors })
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::{
        ese::{Cop, Ese, Position, SectorLine},
        Coordinate,
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
                        visibility_points: vec![Coordinate {
                            lat: 49.040_139_166_666_66,
                            lng: 12.526_625_000_000_001
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
                        visibility_points: vec![Coordinate {
                            lat: 48.180_394_166_666_666,
                            lng: 11.816_536_111_111_112
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
                            Coordinate {
                                lat: 49.447_592_777_777_77,
                                lng: 10.218_426_666_666_668
                            },
                            Coordinate {
                                lat: 52.469_136_388_888_89,
                                lng: 10.870_221_111_111_112
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
            alb_0_105.copns,
            vec![
                Cop {
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
                Cop {
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
            alb_0_105.copxs,
            vec![
                Cop {
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
                Cop {
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
        assert_eq!(alb_0_105.fir_copns, vec![]);
        assert_eq!(alb_0_105.fir_copxs, vec![]);
        assert_eq!(
            alb_0_105.border,
            vec![
                SectorLine {
                    points: vec![
                        Coordinate {
                            lat: 48.6675,
                            lng: 11.794_166_666_666_667
                        },
                        Coordinate {
                            lat: 49.166_666_666_666_664,
                            lng: 11.966_666_666_666_667
                        }
                    ]
                },
                SectorLine {
                    points: vec![
                        Coordinate {
                            lat: 48.6675,
                            lng: 11.794_166_666_666_667
                        },
                        Coordinate {
                            lat: 48.667_777_777_777_77,
                            lng: 11.511_666_666_666_667
                        },
                        Coordinate {
                            lat: 48.667_777_777_777_77,
                            lng: 11.320_833_333_333_333
                        }
                    ]
                },
                SectorLine {
                    points: vec![
                        Coordinate {
                            lat: 48.667_777_777_777_77,
                            lng: 11.320_833_333_333_333
                        },
                        Coordinate {
                            lat: 49.119_444_444_444_45,
                            lng: 10.673_611_111_111_11
                        }
                    ]
                },
                SectorLine {
                    points: vec![
                        Coordinate {
                            lat: 49.119_444_444_444_45,
                            lng: 10.673_611_111_111_11
                        },
                        Coordinate {
                            lat: 49.138_055_555_555_55,
                            lng: 11.1325
                        }
                    ]
                },
                SectorLine {
                    points: vec![
                        Coordinate {
                            lat: 49.138_055_555_555_55,
                            lng: 11.1325
                        },
                        Coordinate {
                            lat: 49.166_666_666_666_664,
                            lng: 11.966_666_666_666_667
                        }
                    ]
                }
            ]
        );
    }
}
