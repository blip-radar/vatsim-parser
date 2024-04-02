use std::collections::HashMap;
use std::io;

use pest::{iterators::Pair, Parser};
use pest_derive::Parser;
use serde::Serialize;
use thiserror::Error;

use crate::Color;

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
    pub airports: Vec<Airport>,
    pub fixes: Vec<Fix>,
    pub ndbs: Vec<NDB>,
    pub vors: Vec<VOR>,
    pub runways: Vec<Runway>,
}

pub type SctResult = Result<Sct, SctError>;

#[derive(Debug)]
enum Section {
    Airport(Vec<Airport>),
    Fixes(Vec<Fix>),
    NDBs(Vec<NDB>),
    VORs(Vec<VOR>),
    Runways(Vec<Runway>),
    Unsupported,
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum SectionName {
    Airport,
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
    let coordinate = parse_coordinate(location.nth(1).unwrap());

    Airport {
        designator,
        coordinate,
    }
}

fn parse_fix(pair: Pair<Rule>) -> Option<Fix> {
    match pair.as_rule() {
        Rule::fix => {
            let mut fix = pair.into_inner();
            let designator = fix.next().unwrap().as_str().to_string();
            let coordinate = parse_coordinate(fix.next().unwrap());

            Some(Fix {
                designator,
                coordinate,
            })
        }
        _ => {
            eprintln!("broken fix: {:?}", pair);
            None
        }
    }
}

fn parse_ndb(pair: Pair<Rule>) -> NDB {
    let mut location = pair.into_inner();
    let designator = location.next().unwrap().as_str().to_string();
    let frequency = location.next().unwrap().as_str().to_string();
    let coordinate = parse_coordinate(location.next().unwrap());

    NDB {
        designator,
        frequency,
        coordinate,
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

fn parse_section(pair: Pair<Rule>, colors: &mut HashMap<String, Color>) -> (SectionName, Section) {

    

    match pair.as_rule() {
        Rule::airport_section => (
            SectionName::Airport,
            Section::Airport(pair.into_inner().filter_map(|pair| {
                if let Rule::color_definition = pair.as_rule() {
                    store_color(colors, pair);
                    None
                } else {
                    Some(parse_airport(pair))
                }
            }).collect()),
        ),
        
        Rule::fixes_section => (
            SectionName::Fixes,
            Section::Fixes(pair.into_inner().filter_map(|pair| {
                if let Rule::color_definition = pair.as_rule() {
                    store_color(colors, pair);
                    None
                } else {
                    parse_fix(pair)
                }
            }).collect()),
        ),
        
        Rule::ndb_section => (
            SectionName::NDBs,
            Section::NDBs(pair.into_inner().filter_map(|pair| {
                if let Rule::color_definition = pair.as_rule() {
                    store_color(colors, pair);
                    None
                } else {
                    Some(parse_ndb(pair))
                }
            }).collect()),
        ),

        Rule::vor_section => (
            SectionName::VORs,
            Section::VORs(pair.into_inner().filter_map(|pair| {
                if let Rule::color_definition = pair.as_rule() {
                    store_color(colors, pair);
                    None
                } else {
                    Some(parse_vor(pair))
                }
            }).collect()),
        ),

        Rule::runway_section => (
            SectionName::Runways,
            Section::Runways(pair.into_inner().filter_map(|pair| {
                if let Rule::color_definition = pair.as_rule() {
                    store_color(colors, pair);
                    None
                } else {
                    Some(parse_runway(pair))
                }
            }).collect()),
        ),

        _ => (SectionName::Unsupported, Section::Unsupported),
    }
}

impl Sct {
    pub fn parse(content: &[u8]) -> SctResult {
        let unparsed_file = read_to_string(content)?;
        let mut colors = HashMap::new();
        let mut sections = SctParser::parse(Rule::sct, &unparsed_file).map(|mut pairs| {
            pairs
                .next()
                .unwrap()
                .into_inner()
                .filter_map(|pair| {
                    if let Rule::color_definition = pair.as_rule() {
                        store_color(&mut colors, pair);
                        None
                    } else {
                        Some(parse_section(pair, &mut colors))
                    }
                })
                .collect::<HashMap<_, _>>()
        })?;
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

        Ok(Sct {
            airports,
            fixes,
            ndbs,
            vors,
            runways,
        })
    }
}

#[cfg(test)]
mod test {
    use crate::sct::{Airport, Fix, Runway, Sct, NDB, VOR};
    use crate::Coordinate;

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

[LOW AIRWAY]
A361       N048.56.21.001 E000.57.11.001 N048.47.26.199 E000.31.49.000
A361       N049.01.42.700 E001.12.50.601 N048.56.21.001 E000.57.11.001
A4         N048.37.03.248 E017.32.28.201 N048.42.56.998 E017.23.09.999
A4         N048.17.25.569 E018.03.02.300 N048.37.03.248 E017.32.28.201
A4         N048.42.56.998 E017.23.09.999 N048.51.11.520 E017.10.04.238
        ";
        let sct = Sct::parse(sct_bytes);
        assert_eq!(
            sct.as_ref().unwrap().ndbs,
            vec![
                NDB {
                    designator: "MIQ".to_string(),
                    frequency: "426.000".to_string(),
                    coordinate: Coordinate {
                        lat: 48.570225,
                        lng: 11.597502777777779,
                    }
                },
                NDB {
                    designator: "RTT".to_string(),
                    frequency: "303.000".to_string(),
                    coordinate: Coordinate {
                        lat: 47.43092194444444,
                        lng: 11.940052777777778
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
                        lat: 49.502918888888885,
                        lng: 11.035
                    }
                },
                VOR {
                    designator: "OTT".to_string(),
                    frequency: "112.300".to_string(),
                    coordinate: Coordinate {
                        lat: 48.18039388888889,
                        lng: 11.816535833333335
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
                        lat: 48.35378277777778,
                        lng: 11.786085833333333
                    }
                },
                Airport {
                    designator: "EDNX".to_string(),
                    coordinate: Coordinate {
                        lat: 48.238999722222225,
                        lng: 11.559166944444446
                    }
                },
                Airport {
                    designator: "LIPB".to_string(),
                    coordinate: Coordinate {
                        lat: 46.46027777777778,
                        lng: 11.32638888888889
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
                        lat: 49.51833305555555,
                        lng: 8.445
                    }
                },
                Fix {
                    designator: "ARMUT".to_string(),
                    coordinate: Coordinate {
                        lat: 49.722499722222224,
                        lng: 12.323332777777777
                    }
                },
                Fix {
                    designator: "GEDSO".to_string(),
                    coordinate: Coordinate {
                        lat: 47.080555833333335,
                        lng: 11.870277777777778
                    }
                },
                Fix {
                    designator: "INBED".to_string(),
                    coordinate: Coordinate {
                        lat: 49.3875,
                        lng: 10.941666944444444
                    }
                },
                Fix {
                    designator: "NAXAV".to_string(),
                    coordinate: Coordinate {
                        lat: 46.463855833333334,
                        lng: 11.322182777777778
                    }
                },
                Fix {
                    designator: "UNKUL".to_string(),
                    coordinate: Coordinate {
                        lat: 49.13722194444444,
                        lng: 11.459721944444444
                    }
                },
                Fix {
                    designator: "VEMUT".to_string(),
                    coordinate: Coordinate {
                        lat: 49.81074388888889,
                        lng: 12.461246944444444
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
                            lat: 48.34066888888889,
                            lng: 11.751016944444444
                        },
                        Coordinate {
                            lat: 48.34479694444445,
                            lng: 11.80461388888889
                        }
                    ),
                    aerodrome: "EDDM".to_string()
                },
                Runway {
                    designators: ("08L".to_string(), "26R".to_string()),
                    headings: (80, 260),
                    location: (
                        Coordinate {
                            lat: 48.362766944444445,
                            lng: 11.767549722222222
                        },
                        Coordinate {
                            lat: 48.366885833333335,
                            lng: 11.821171944444444
                        }
                    ),
                    aerodrome: "EDDM".to_string()
                },
                Runway {
                    designators: ("07".to_string(), "25".to_string()),
                    headings: (71, 251),
                    location: (
                        Coordinate {
                            lat: 48.23825277777778,
                            lng: 11.55391388888889
                        },
                        Coordinate {
                            lat: 48.24010777777778,
                            lng: 11.56444388888889
                        }
                    ),
                    aerodrome: "EDNX".to_string()
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

        assert!(sct.is_ok())
    }
}
