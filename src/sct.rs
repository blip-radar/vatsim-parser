use std::collections::HashMap;
use std::io;

use pest::{iterators::Pair, Parser};
use pest_derive::Parser;
use serde::Serialize;
use thiserror::Error;

use crate::{Color, Location};

use super::{read_to_string, Coordinate, DegMinSec};

#[derive(Parser)]
#[grammar = "sct.pest"]
pub struct SctParser;

#[derive(Error, Debug)]
pub enum SctError {
    #[error("failed to parse .sct file: {0:?}")]
    Parse(#[from] pest::error::Error<Rule>),
    #[error("failed to read .sct file: {0:?}")]
    FileRead(#[from] io::Error),
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Airport {
    pub designator: String,
    pub coordinate: Coordinate,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Fix {
    pub designator: String,
    pub coordinate: Coordinate,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Airway {
    pub designator: String,
    pub start: Location,
    pub end: Location,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct NDB {
    pub designator: String,
    pub frequency: String,
    pub coordinate: Coordinate,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct VOR {
    pub designator: String,
    pub frequency: String,
    pub coordinate: Coordinate,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Runway {
    pub designators: (String, String),
    pub headings: (u32, u32),
    pub location: (Coordinate, Coordinate),
    pub aerodrome: String,
}

// TODO GEO, REGIONS, ARTCC, SID, STAR, AIRWAY
#[derive(Debug, Serialize, PartialEq)]
pub struct Sct {
    pub info: SctInfo,
    pub colors: HashMap<String, Color>,
    pub airports: Vec<Airport>,
    pub fixes: Vec<Fix>,
    pub ndbs: Vec<NDB>,
    pub vors: Vec<VOR>,
    pub runways: Vec<Runway>,
    pub high_airways: Vec<Airway>,
    pub low_airways: Vec<Airway>,
}

#[derive(Debug, Default, Serialize, PartialEq)]
pub struct SctInfo {
    pub name: String,
    pub default_callsign: String,
    pub default_airport: String,
    pub centre_point: Coordinate,
    pub miles_per_deg_lat: f64,
    pub miles_per_deg_lng: f64,
    pub magnetic_variation: f64,
    pub scale_factor: f64,
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
            "N" | "E" => 1.0,
            "S" | "W" => -1.0,
            _ => unreachable!(),
        };
    let min = coordinate_part.next().unwrap().as_str().parse().unwrap();
    let sec = coordinate_part.next().unwrap().as_str().parse().unwrap();

    (degrees, min, sec)
}

fn parse_coordinate(pair: Pair<Rule>) -> Coordinate {
    let mut coordinate = pair.into_inner();
    let lat = parse_coordinate_part(coordinate.next().unwrap());
    let lng = parse_coordinate_part(coordinate.next().unwrap());
    Coordinate::from_deg_min_sec(lat, lng)
}

fn parse_airport(pair: Pair<Rule>) -> Airport {
    let mut location = pair.into_inner();
    let designator = location.next().unwrap().as_str().to_string();
    let coordinate = parse_coordinate(
        location
            .find(|pair| matches!(pair.as_rule(), Rule::coordinate))
            .unwrap(),
    );

    Airport {
        designator,
        coordinate,
    }
}

fn parse_or_fix(pair: Pair<Rule>) -> Location {
    match pair.as_rule() {
        Rule::coordinate => Location::Coordinate(parse_coordinate(pair)),
        Rule::airway_fix => Location::Fix(pair.into_inner().next().unwrap().as_str().to_string()),
        rule => {
            eprintln!("{rule:?}");
            unreachable!()
        }
    }
}

fn parse_airway(pair: Pair<Rule>) -> Option<Airway> {
    let mut airway = pair.into_inner();
    let designator = airway.next().unwrap().as_str().to_string();

    let (start, end) = if let (Some(start), Some(end)) = (airway.next(), airway.next()) {
        (parse_or_fix(start), parse_or_fix(end))
    } else {
        eprintln!("broken airway (initial parse): {airway:?}");
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
        eprintln!("broken fix: {pair:?}");
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
        eprintln!("broken ndb: {pair:?}");
        None
    }
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

fn parse_info_section(pair: Pair<Rule>, colors: &mut HashMap<String, Color>) -> SctInfo {
    let mut sct_info = SctInfo::default();
    let mut i = 0;
    let mut lat = DegMinSec::default();

    for pair in pair.into_inner() {
        if let Rule::color_definition = pair.as_rule() {
            store_color(colors, pair);
        } else {
            match i {
                0 => sct_info.name = pair.as_str().to_string(),
                1 => sct_info.default_callsign = pair.as_str().to_string(),
                2 => sct_info.default_airport = pair.as_str().to_string(),
                3 => lat = parse_coordinate_part(pair),
                4 => {
                    let lng = parse_coordinate_part(pair);
                    sct_info.centre_point = Coordinate::from_deg_min_sec(lat, lng);
                }
                5 => sct_info.miles_per_deg_lat = pair.as_str().parse().unwrap(),
                6 => sct_info.miles_per_deg_lng = pair.as_str().parse().unwrap(),
                7 => sct_info.magnetic_variation = pair.as_str().parse().unwrap(),
                8 => sct_info.scale_factor = pair.as_str().parse().unwrap(),
                _ => unreachable!(),
            }
            i += 1;
        }
    }
    sct_info
}

fn parse_color_definition(pair: Pair<Rule>) -> (String, Color) {
    let mut pairs = pair.into_inner();
    let color_name = pairs.next().unwrap().as_str().to_string();
    let color_value = Color::from_euroscope(pairs.next().unwrap().as_str().parse().unwrap());
    (color_name, color_value)
}

#[inline]
fn store_color(colors: &mut HashMap<String, Color>, pair: Pair<Rule>) {
    let (color_name, color) = parse_color_definition(pair);
    colors.insert(color_name, color);
}

fn parse_independent_section(
    pair: Pair<Rule>,
    colors: &mut HashMap<String, Color>,
) -> (SectionName, Section) {
    match pair.as_rule() {
        Rule::info_section => (
            SectionName::Info,
            Section::Info(parse_info_section(pair, colors)),
        ),
        Rule::airport_section => (
            SectionName::Airport,
            Section::Airport(
                pair.into_inner()
                    .filter_map(|pair| {
                        if let Rule::color_definition = pair.as_rule() {
                            store_color(colors, pair);
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
                        if let Rule::color_definition = pair.as_rule() {
                            store_color(colors, pair);
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
                        if let Rule::color_definition = pair.as_rule() {
                            store_color(colors, pair);
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
                        if let Rule::color_definition = pair.as_rule() {
                            store_color(colors, pair);
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
                        if let Rule::color_definition = pair.as_rule() {
                            store_color(colors, pair);
                            None
                        } else {
                            Some(parse_runway(pair))
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
                        if let Rule::color_definition = pair.as_rule() {
                            store_color(colors, pair);
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
                        if let Rule::color_definition = pair.as_rule() {
                            store_color(colors, pair);
                            None
                        } else {
                            parse_airway(pair)
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
        let mut colors = HashMap::new();
        let sct_parse = SctParser::parse(Rule::sct, &unparsed_file);
        let mut sections = sct_parse.map(|mut pairs| {
            pairs
                .next()
                .unwrap()
                .into_inner()
                .filter_map(|pair| {
                    if let Rule::color_definition = pair.as_rule() {
                        store_color(&mut colors, pair);
                        None
                    } else {
                        Some(parse_independent_section(pair, &mut colors))
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
        let high_airways = match sections.remove_entry(&SectionName::HighAirway) {
            Some((_, Section::HighAirways(high_airways))) => high_airways,
            _ => vec![],
        };
        let low_airways = match sections.remove_entry(&SectionName::LowAirway) {
            Some((_, Section::LowAirways(low_airways))) => low_airways,
            _ => vec![],
        };

        let sct = Sct {
            info,
            airports,
            colors: colors.clone(),
            fixes,
            ndbs,
            vors,
            runways,
            high_airways,
            low_airways,
        };

        Ok(sct)
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::sct::{Airport, Airway, Fix, Runway, Sct, SctInfo, NDB, VOR};
    use crate::{Color, Coordinate, Location};

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

#define COLOR_APP       16711680
#define COLOR_AirspaceA  8421376
#define prohibitcolor 7697781		; 117,117,117	Prohibited areas

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
EDDM TRAN RNP26R LANDU26                 N048.35.46.899 E012.16.26.140 N048.32.16.501 E011.30.05.529 COLOR_APP
                                         N048.32.16.501 E011.30.05.529 N048.25.44.061 E011.31.15.931 COLOR_APP
                                         N048.25.44.061 E011.31.15.931 N048.29.36.319 E012.22.42.409 COLOR_APP
                                         N048.29.36.319 E012.22.42.409 N048.30.47.361 E012.37.38.952 COLOR_APP

EDDN TRAN ILS10 DN430                    N049.33.11.289 E010.30.33.379 N049.32.37.168 E010.36.38.149 COLOR_APP
                                         N049.32.37.168 E010.36.38.149 N049.32.02.731 E010.42.42.681 COLOR_APP

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

Release line EDMM ARBAX Window           N049.26.33.000 E012.52.22.000 N049.35.18.000 E013.00.24.000 COLOR_Releaseline
                                         N049.35.18.000 E013.00.24.000 N049.27.45.000 E013.17.13.000 COLOR_Releaseline
                                         N049.27.45.000 E013.17.13.000 N049.12.42.000 E013.13.14.000 COLOR_Releaseline
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
RMZ EDMS                                 N048.57.40.000 E012.23.34.000 N048.56.22.000 E012.39.38.000 COLOR_RMZ
                                         N048.56.22.000 E012.39.38.000 N048.50.25.000 E012.38.30.000 COLOR_RMZ
                                         N048.50.25.000 E012.38.30.000 N048.51.43.000 E012.22.28.000 COLOR_RMZ
                                         N048.51.43.000 E012.22.28.000 N048.57.40.000 E012.23.34.000 COLOR_RMZ

RMZ EDNX                                 N048.16.27.000 E011.27.21.000 N048.17.12.000 E011.32.05.000 COLOR_RMZ
                                         N048.17.12.000 E011.32.05.000 N048.16.25.000 E011.32.09.000 COLOR_RMZ
                                         N048.16.25.000 E011.32.09.000 N048.16.54.000 E011.38.27.000 COLOR_RMZ
                                         N048.16.54.000 E011.38.27.000 N048.15.17.000 E011.40.25.000 COLOR_RMZ
                                         N048.15.17.000 E011.40.25.000 N048.14.28.000 E011.40.40.000 COLOR_RMZ
                                         N048.14.28.000 E011.40.40.000 N048.12.14.000 E011.39.45.000 COLOR_RMZ
                                         N048.12.14.000 E011.39.45.000 N048.10.38.000 E011.29.22.000 COLOR_RMZ
                                         N048.10.38.000 E011.29.22.000 N048.16.27.000 E011.27.21.000 COLOR_RMZ


[GEO]

EDDN Groundlayout Holding Points         N049.29.58.736 E011.03.33.028 N049.29.58.942 E011.03.34.353 COLOR_Stopbar
                                         N049.29.56.786 E011.03.52.659 N049.29.57.006 E011.03.54.022 COLOR_Stopbar
                                         N049.29.54.726 E011.04.12.424 N049.29.55.069 E011.04.13.780 COLOR_Stopbar
                                         N049.29.52.460 E011.04.35.712 N049.29.52.803 E011.04.36.718 COLOR_Stopbar
                                         N049.29.49.782 E011.05.07.369 N049.29.49.453 E011.05.08.660 COLOR_Stopbar
                                         N049.29.46.390 E011.05.42.055 N049.29.45.841 E011.05.43.456 COLOR_Stopbar
                                         N049.29.48.890 E011.04.19.445 N049.29.48.739 E011.04.21.004 COLOR_Stopbar
                                         N049.29.47.750 E011.04.38.414 N049.29.47.667 E011.04.39.214 COLOR_Stopbar
                                         N049.29.47.667 E011.04.39.214 N049.29.48.038 E011.04.40.474 COLOR_Stopbar
                                         N049.29.47.475 E011.05.03.614 N049.29.46.871 E011.05.04.269 COLOR_Stopbar
                                         N049.29.43.547 E011.05.35.831 N049.29.42.669 E011.05.35.961 COLOR_Stopbar

EDQC Groundlayout                        N050.15.52.011 E010.59.29.553 N050.15.52.239 E010.59.29.566 COLOR_Stopbar
                                         N050.15.39.345 E011.00.04.860 N050.15.39.411 E011.00.05.139 COLOR_Stopbar

[REGIONS]
REGIONNAME EDDM Groundlayout
COLOR_Stopbar              N048.21.53.621 E011.48.32.152
                           N048.21.54.514 E011.48.32.715
                           N048.21.54.527 E011.48.32.571
                           N048.21.53.676 E011.48.32.042
REGIONNAME EDMO Groundlayout
COLOR_HardSurface1         N048.04.25.470 E011.16.20.093
                           N048.04.24.509 E011.16.21.717
                           N048.05.20.677 E011.17.38.446
                           N048.05.21.638 E011.17.36.829

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
                centre_point: Coordinate {
                    lat: 48.353_782_777_777_78,
                    lng: 11.786_085_833_333_333
                },
                miles_per_deg_lat: 60.0,
                miles_per_deg_lng: 39.0,
                magnetic_variation: -3.0,
                scale_factor: 1.0,
            }
        );
        assert_eq!(
            sct.as_ref().unwrap().colors,
            HashMap::from([
                (
                    "COLOR_APP".to_string(),
                    Color {
                        r: 0,
                        g: 0,
                        b: 255,
                        a: 255
                    }
                ),
                (
                    "COLOR_AirspaceA".to_string(),
                    Color {
                        r: 0,
                        g: 128,
                        b: 128,
                        a: 255
                    }
                ),
                (
                    "prohibitcolor".to_string(),
                    Color {
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
                    coordinate: Coordinate {
                        lat: 48.570_225,
                        lng: 11.597_502_777_777_779,
                    }
                },
                NDB {
                    designator: "RTT".to_string(),
                    frequency: "303.000".to_string(),
                    coordinate: Coordinate {
                        lat: 47.430_921_944_444_44,
                        lng: 11.940_052_777_777_778
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
                    coordinate: Coordinate {
                        lat: 49.502_918_888_888_885,
                        lng: 11.035
                    }
                },
                VOR {
                    designator: "OTT".to_string(),
                    frequency: "112.300".to_string(),
                    coordinate: Coordinate {
                        lat: 48.180_393_888_888_89,
                        lng: 11.816_535_833_333_335
                    }
                }
            ]
        );
        assert_eq!(
            sct.as_ref().unwrap().airports,
            vec![
                Airport {
                    designator: "EDDM".to_string(),
                    coordinate: Coordinate {
                        lat: 48.353_782_777_777_78,
                        lng: 11.786_085_833_333_333
                    }
                },
                Airport {
                    designator: "EDNX".to_string(),
                    coordinate: Coordinate {
                        lat: 48.238_999_722_222_225,
                        lng: 11.559_166_944_444_446
                    }
                },
                Airport {
                    designator: "LIPB".to_string(),
                    coordinate: Coordinate {
                        lat: 46.460_277_777_777_78,
                        lng: 11.326_388_888_888_89
                    }
                }
            ]
        );
        assert_eq!(
            sct.as_ref().unwrap().fixes,
            vec![
                Fix {
                    designator: "(FM-C)".to_string(),
                    coordinate: Coordinate {
                        lat: 49.518_333_055_555_55,
                        lng: 8.445
                    }
                },
                Fix {
                    designator: "ARMUT".to_string(),
                    coordinate: Coordinate {
                        lat: 49.722_499_722_222_224,
                        lng: 12.323_332_777_777_777
                    }
                },
                Fix {
                    designator: "GEDSO".to_string(),
                    coordinate: Coordinate {
                        lat: 47.080_555_833_333_335,
                        lng: 11.870_277_777_777_778
                    }
                },
                Fix {
                    designator: "INBED".to_string(),
                    coordinate: Coordinate {
                        lat: 49.3875,
                        lng: 10.941_666_944_444_444
                    }
                },
                Fix {
                    designator: "NAXAV".to_string(),
                    coordinate: Coordinate {
                        lat: 46.463_855_833_333_334,
                        lng: 11.322_182_777_777_778
                    }
                },
                Fix {
                    designator: "UNKUL".to_string(),
                    coordinate: Coordinate {
                        lat: 49.137_221_944_444_44,
                        lng: 11.459_721_944_444_444
                    }
                },
                Fix {
                    designator: "VEMUT".to_string(),
                    coordinate: Coordinate {
                        lat: 49.810_743_888_888_89,
                        lng: 12.461_246_944_444_444
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
                        Coordinate {
                            lat: 48.340_668_888_888_89,
                            lng: 11.751_016_944_444_444
                        },
                        Coordinate {
                            lat: 48.344_796_944_444_45,
                            lng: 11.804_613_888_888_89
                        }
                    ),
                    aerodrome: "EDDM".to_string()
                },
                Runway {
                    designators: ("08L".to_string(), "26R".to_string()),
                    headings: (80, 260),
                    location: (
                        Coordinate {
                            lat: 48.362_766_944_444_445,
                            lng: 11.767_549_722_222_222
                        },
                        Coordinate {
                            lat: 48.366_885_833_333_335,
                            lng: 11.821_171_944_444_444
                        }
                    ),
                    aerodrome: "EDDM".to_string()
                },
                Runway {
                    designators: ("07".to_string(), "25".to_string()),
                    headings: (71, 251),
                    location: (
                        Coordinate {
                            lat: 48.238_252_777_777_78,
                            lng: 11.553_913_888_888_89
                        },
                        Coordinate {
                            lat: 48.240_107_777_777_78,
                            lng: 11.564_443_888_888_89
                        }
                    ),
                    aerodrome: "EDNX".to_string()
                },
                Runway {
                    designators: ("04".to_string(), "22".to_string()),
                    headings: (37, 217),
                    location: (
                        Coordinate {
                            lat: 54.612_038_888_888_89,
                            lng: -5.879_761_111_111_112
                        },
                        Coordinate {
                            lat: 54.624_855_555_555_555,
                            lng: -5.864_430_555_555_555
                        }
                    ),
                    aerodrome: "EGAC".to_string()
                }
            ]
        );
        assert_eq!(
            sct.as_ref().unwrap().high_airways,
            vec![
                Airway {
                    designator: "B73".to_string(),
                    start: Location::Coordinate(Coordinate {
                        lat: 54.912_777_777_777_78,
                        lng: 18.958_332_777_777_777
                    }),
                    end: Location::Coordinate(Coordinate {
                        lat: 55.603_610_833_333_335,
                        lng: 19.838_305_833_333_333
                    })
                },
                Airway {
                    designator: "B74".to_string(),
                    start: Location::Coordinate(Coordinate {
                        lat: 55.201_388_888_888_89,
                        lng: 19.634_166_944_444_445
                    }),
                    end: Location::Coordinate(Coordinate {
                        lat: 55.603_610_833_333_335,
                        lng: 19.838_305_833_333_333
                    })
                },
                Airway {
                    designator: "B74".to_string(),
                    start: Location::Coordinate(Coordinate {
                        lat: 54.637_777_777_777_78,
                        lng: 19.355_555_833_333_334
                    }),
                    end: Location::Coordinate(Coordinate {
                        lat: 55.201_388_888_888_89,
                        lng: 19.634_166_944_444_445
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
                    start: Location::Coordinate(Coordinate {
                        lat: 48.939_166_944_444_445,
                        lng: 0.953_055_833_333_333_3
                    }),
                    end: Location::Coordinate(Coordinate {
                        lat: 48.790_610_833_333_33,
                        lng: 0.530_277_777_777_777_8
                    })
                },
                Airway {
                    designator: "A361".to_string(),
                    start: Location::Coordinate(Coordinate {
                        lat: 49.028_527_777_777_775,
                        lng: 1.214_055_833_333_333_3
                    }),
                    end: Location::Coordinate(Coordinate {
                        lat: 48.939_166_944_444_445,
                        lng: 0.953_055_833_333_333_3
                    })
                },
                Airway {
                    designator: "A4".to_string(),
                    start: Location::Coordinate(Coordinate {
                        lat: 48.617_568_888_888_89,
                        lng: 17.541_166_944_444_445
                    }),
                    end: Location::Coordinate(Coordinate {
                        lat: 48.715_832_777_777_78,
                        lng: 17.386_110_833_333_333
                    })
                },
                Airway {
                    designator: "A4".to_string(),
                    start: Location::Coordinate(Coordinate {
                        lat: 48.290_435_833_333_33,
                        lng: 18.050_638_888_888_89
                    }),
                    end: Location::Coordinate(Coordinate {
                        lat: 48.617_568_888_888_89,
                        lng: 17.541_166_944_444_445
                    })
                },
                Airway {
                    designator: "A4".to_string(),
                    start: Location::Coordinate(Coordinate {
                        lat: 48.715_832_777_777_78,
                        lng: 17.386_110_833_333_333
                    }),
                    end: Location::Coordinate(Coordinate {
                        lat: 48.8532,
                        lng: 17.167_843_888_888_89
                    })
                },
                Airway {
                    designator: "A5".to_string(),
                    start: Location::Fix("RTT".to_string()),
                    end: Location::Fix("NUB".to_string())
                }
            ]
        );
        // TODO GEO, REGIONS, ARTCC, SID, STAR, AIRWAY
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

#define COLOR_APP       16711680
#define COLOR_AirspaceA  8421376

[SID]

[STAR]
EDDM TRAN RNP26R LANDU26                 N048.35.46.899 E012.16.26.140 N048.32.16.501 E011.30.05.529 COLOR_APP
                                         N048.32.16.501 E011.30.05.529 N048.25.44.061 E011.31.15.931 COLOR_APP
                                         N048.25.44.061 E011.31.15.931 N048.29.36.319 E012.22.42.409 COLOR_APP
                                         N048.29.36.319 E012.22.42.409 N048.30.47.361 E012.37.38.952 COLOR_APP

EDDN TRAN ILS10 DN430                    N049.33.11.289 E010.30.33.379 N049.32.37.168 E010.36.38.149 COLOR_APP
                                         N049.32.37.168 E010.36.38.149 N049.32.02.731 E010.42.42.681 COLOR_APP

EDQD STAR ALL LONLIxZ                    N050.04.29.060 E011.13.34.989 N049.53.50.819 E011.24.28.540
        ";
        let sct = Sct::parse(sct_bytes);

        assert!(sct.is_ok());
    }
}
