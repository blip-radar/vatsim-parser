pub mod airways;

use std::collections::HashMap;

use geo_types::Coord;
use multimap::MultiMap;
use regex::Regex;
use serde::Serialize;

use crate::{
    ese::{Ese, SidStar},
    sct::{self, Sct},
    Location, TwoKeyMultiMap,
};

use self::airways::FixAirwayMap;

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Fix {
    pub designator: String,
    pub coordinate: Coord,
}
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct NDB {
    pub designator: String,
    pub frequency: String,
    pub coordinate: Coord,
}
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct VOR {
    pub designator: String,
    pub frequency: String,
    pub coordinate: Coord,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Runway {
    pub designators: (String, String),
    pub headings: (u32, u32),
    pub location: (Coord, Coord),
    pub aerodrome: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Airport {
    pub designator: String,
    pub coordinate: Coord,
    pub runways: Vec<Runway>,
}
impl Airport {
    fn from_sct_airports(
        airports: Vec<sct::Airport>,
        runways: Vec<Runway>,
    ) -> HashMap<String, Airport> {
        airports
            .into_iter()
            .map(|ap| {
                (
                    ap.designator.clone(),
                    Airport {
                        coordinate: ap.coordinate,
                        runways: runways
                            .iter()
                            .filter(|r| r.aerodrome == ap.designator)
                            .cloned()
                            .collect(),
                        designator: ap.designator,
                    },
                )
            })
            .collect()
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct SID {
    pub name: String,
    pub airport: String,
    pub runway: String,
    pub waypoints: Vec<Fix>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct STAR {
    pub name: String,
    pub airport: String,
    pub runway: String,
    pub waypoints: Vec<Fix>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Locations {
    pub fixes: MultiMap<String, Fix>,
    pub vors: MultiMap<String, VOR>,
    pub ndbs: MultiMap<String, NDB>,
    pub airports: HashMap<String, Airport>,
    pub airways: FixAirwayMap,
    pub sids: TwoKeyMultiMap<String, String, SID>,
    pub stars: TwoKeyMultiMap<String, String, STAR>,
}

impl Locations {
    pub(super) fn from_euroscope(sct: Sct, ese: Ese, airways: FixAirwayMap) -> Self {
        let fixes = sct.fixes.into_iter().fold(MultiMap::new(), |mut acc, fix| {
            acc.insert(fix.designator.clone(), fix);
            acc
        });
        let vors = sct.vors.into_iter().fold(MultiMap::new(), |mut acc, vor| {
            acc.insert(vor.designator.clone(), vor);
            acc
        });
        let ndbs = sct.ndbs.into_iter().fold(MultiMap::new(), |mut acc, ndb| {
            acc.insert(ndb.designator.clone(), ndb);
            acc
        });
        let mut locations = Locations {
            fixes,
            vors,
            ndbs,
            airports: Airport::from_sct_airports(sct.airports, sct.runways),
            airways,
            sids: TwoKeyMultiMap(MultiMap::new()),
            stars: TwoKeyMultiMap(MultiMap::new()),
        };
        ese.sids_stars
            .into_iter()
            .for_each(|sid_star| match sid_star {
                SidStar::Sid(sid) => locations.sids.0.insert(
                    (sid.airport.clone(), sid.name.clone()),
                    SID {
                        waypoints: sid
                            .waypoints
                            .into_iter()
                            .filter_map(|wpt| {
                                if let Some(coordinate) = locations.convert_designator(&wpt) {
                                    Some(Fix {
                                        coordinate,
                                        designator: wpt,
                                    })
                                } else {
                                    eprintln!("Waypoint {wpt} not found in SID {}", sid.name);
                                    None
                                }
                            })
                            .collect(),
                        name: sid.name,
                        airport: sid.airport,
                        runway: sid.runway,
                    },
                ),
                SidStar::Star(star) => locations.stars.0.insert(
                    (star.airport.clone(), star.name.clone()),
                    STAR {
                        waypoints: star
                            .waypoints
                            .into_iter()
                            .filter_map(|wpt| {
                                if let Some(coordinate) = locations.convert_designator(&wpt) {
                                    Some(Fix {
                                        coordinate,
                                        designator: wpt,
                                    })
                                } else {
                                    eprintln!(
                                        "STAR {} {} {}: waypoint {wpt} not found",
                                        star.airport, star.name, star.runway
                                    );
                                    None
                                }
                            })
                            .collect(),
                        name: star.name,
                        airport: star.airport,
                        runway: star.runway,
                    },
                ),
            });

        locations
    }

    pub(crate) fn convert_location(&self, loc: &Location) -> Option<Coord> {
        match loc {
            Location::Coordinate(c) => Some(*c),
            Location::Fix(wpt) => self.convert_designator(wpt),
        }
    }

    fn convert_coordinate(&self, designator: &str) -> Option<Coord> {
        let regex = Regex::new(r"^(\d{1,6})(N|S)(\d{2,7})(E|W)$").unwrap();
        regex.captures(designator).and_then(|captures| {
            let lat_str = &captures[1];
            let lng_str = &captures[3];
            let normalised_lat_str = if matches!(lat_str.len(), 1 | 3 | 5) {
                // invalid syntax
                if lat_str.starts_with('0') {
                    eprintln!("Coordinate waypoints must not be abbreviated and start with a 0: {designator} (lat_str)");
                    return None;
                }
                format!("0{lat_str}")
            } else {
                lat_str.to_string()
            };
            let normalised_lng_str = if matches!(lng_str.len(), 2 | 4 | 6) {
                // invalid syntax
                if lng_str.starts_with('0') {
                    eprintln!("Coordinate waypoints must not be abbreviated and start with a 0: {designator} (lng_str)");
                    return None;
                }
                format!("0{lng_str}")
            } else {
                lng_str.to_string()
            };
            if normalised_lng_str.len() - normalised_lat_str.len() != 1 {
                eprintln!("Coordinate waypoints must have the same precision in lat/lon: {designator}");
                return None;
            }
            let lat: f64 = match normalised_lat_str.len() {
                2 => normalised_lat_str.parse().unwrap(),
                4 => normalised_lat_str[0..2].parse::<f64>().unwrap() + normalised_lat_str[2..4].parse::<f64>().unwrap() / 60.0,
                6 => normalised_lat_str[0..2].parse::<f64>().unwrap() + normalised_lat_str[2..4].parse::<f64>().unwrap() / 60.0
                    + normalised_lat_str[4..6].parse::<f64>().unwrap() / 3600.0,
                _ => unreachable!(),
            };
            let lng: f64 = match normalised_lng_str.len() {
                3 => normalised_lng_str.parse().unwrap(),
                5 => normalised_lng_str[0..3].parse::<f64>().unwrap() + normalised_lng_str[3..5].parse::<f64>().unwrap() / 60.0,
                7 => normalised_lng_str[0..3].parse::<f64>().unwrap() + normalised_lng_str[3..5].parse::<f64>().unwrap() / 60.0
                    + normalised_lng_str[5..7].parse::<f64>().unwrap() / 3600.0,
                _ => unreachable!(),
            };
            let n_s = &captures[2];
            let w_e = &captures[4];
            Some(Coord {
                y: if n_s == "N" { 1.0 } else { -1.0 } * lat,
                x: if w_e == "E" { 1.0 } else { -1.0 } * lng,
            })
        })
    }

    pub fn convert_designator(&self, designator: &str) -> Option<Coord> {
        self.vors
            .get(designator)
            .map(|vor| vor.coordinate)
            .or(self.ndbs.get(designator).map(|ndb| ndb.coordinate))
            .or(self.fixes.get(designator).map(|fix| fix.coordinate))
            .or(self
                .airports
                .get(designator)
                .map(|airport| airport.coordinate))
            .or(self.convert_coordinate(designator))
    }

    pub fn contains_designator(&self, designator: &str) -> bool {
        self.vors.contains_key(designator)
            || self.ndbs.contains_key(designator)
            || self.fixes.contains_key(designator)
            || self.airports.contains_key(designator)
    }
}

#[cfg(test)]
mod test {
    use geo_types::Coord;

    use crate::adaptation::locations::{Airport, Fix, Locations, NDB, VOR};

    #[test]
    fn test_get_by_wpt() {
        let locs = Locations {
            fixes: [(
                "ARMUT".to_string(),
                Fix {
                    designator: "ARMUT".to_string(),
                    coordinate: Coord {
                        y: 49.722_499_722_222_224,
                        x: 12.323_332_777_777_777,
                    },
                },
            )]
            .into_iter()
            .collect(),
            ndbs: [(
                "MIQ".to_string(),
                NDB {
                    designator: "MIQ".to_string(),
                    frequency: "426.000".to_string(),
                    coordinate: Coord {
                        y: 48.570_225,
                        x: 11.597_502_777_777_779,
                    },
                },
            )]
            .into_iter()
            .collect(),
            vors: [(
                "OTT".to_string(),
                VOR {
                    designator: "OTT".to_string(),
                    frequency: "112.300".to_string(),
                    coordinate: Coord {
                        y: 48.180_393_888_888_89,
                        x: 11.816_535_833_333_335,
                    },
                },
            )]
            .into_iter()
            .collect(),
            airports: [(
                "EDDM".to_string(),
                Airport {
                    designator: "EDDM".to_string(),
                    runways: vec![],
                    coordinate: Coord {
                        y: 48.353_782_777_777_78,
                        x: 11.786_085_833_333_333,
                    },
                },
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        };

        assert_eq!(
            locs.convert_designator("MIQ").unwrap(),
            Coord {
                y: 48.570_225,
                x: 11.597_502_777_777_779,
            }
        );
        assert_eq!(
            locs.convert_designator("OTT").unwrap(),
            Coord {
                y: 48.180_393_888_888_89,
                x: 11.816_535_833_333_335
            }
        );
        assert_eq!(
            locs.convert_designator("EDDM").unwrap(),
            Coord {
                y: 48.353_782_777_777_78,
                x: 11.786_085_833_333_333
            }
        );
        assert_eq!(
            locs.convert_designator("ARMUT").unwrap(),
            Coord {
                y: 49.722_499_722_222_224,
                x: 12.323_332_777_777_777
            }
        );
        assert_eq!(locs.convert_designator("OZE"), None);
        assert_eq!(
            locs.convert_designator("46N078W").unwrap(),
            Coord { y: 46.0, x: -78.0 }
        );
        assert_eq!(
            locs.convert_designator("4620N05805W").unwrap(),
            Coord {
                y: 46.333333333333336,
                x: -58.083333333333336
            }
        );
        assert_eq!(
            locs.convert_designator("462013N0580503W").unwrap(),
            Coord {
                y: 46.33694444444445,
                x: -58.08416666666667
            }
        );
        assert_eq!(
            locs.convert_designator("4N40W").unwrap(),
            Coord { y: 4.0, x: -40.0 }
        );
        assert_eq!(
            locs.convert_designator("04N40W").unwrap(),
            Coord { y: 4.0, x: -40.0 }
        );
        assert_eq!(locs.convert_designator("4N04W"), None);
        assert_eq!(locs.convert_designator("04N04W"), None);
        assert_eq!(
            locs.convert_designator("400N4000W").unwrap(),
            Coord { y: 4.0, x: -40.0 }
        );
        assert_eq!(
            locs.convert_designator("0400N4000W").unwrap(),
            Coord { y: 4.0, x: -40.0 }
        );
        assert_eq!(locs.convert_designator("400N0400W"), None);
        assert_eq!(locs.convert_designator("0400N0400W"), None);
        // TODO test runways, r/b
    }
}
