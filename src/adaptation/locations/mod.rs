pub mod airways;

use std::collections::HashMap;

use geo::{Coord, GeodesicDestination, Point};
use multimap::MultiMap;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;
use tracing::warn;
use uom::si::f64::Length;
use uom::si::length::{meter, nautical_mile};

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
        runways: &[Runway],
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
    pub runway: Option<String>,
    pub waypoints: Vec<Fix>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct STAR {
    pub name: String,
    pub airport: String,
    pub runway: Option<String>,
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

static COORD_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(\d{1,6})(N|S)(\d{2,7})(E|W)$").unwrap());
static RANGE_BEARING_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^([0-9A-Z]{2,5})(\d{3})(\d{3})$").unwrap());
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
            airports: Airport::from_sct_airports(sct.airports, &sct.runways),
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
                                    warn!("Waypoint {wpt} not found in SID {}", sid.name);
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
                                    warn!(
                                        "STAR {} {} {}: waypoint {wpt} not found",
                                        star.airport,
                                        star.name,
                                        star.runway.as_deref().unwrap_or("")
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

    fn convert_range_bearing(&self, designator: &str) -> Option<Coord> {
        RANGE_BEARING_RE.captures(designator).and_then(|captures| {
            let fix = &captures[1];
            // TODO magnetic
            let bearing: f64 = captures[2].parse().unwrap();
            let range = Length::new::<nautical_mile>(captures[3].parse::<f64>().unwrap());

            self.convert_fix(fix).map(|c| {
                Point::from(c)
                    .geodesic_destination(bearing, range.get::<meter>())
                    .0
            })
        })
    }

    fn convert_coordinate(designator: &str) -> Option<Coord> {
        COORD_RE.captures(designator).and_then(|captures| {
            let lat_str = &captures[1];
            let lng_str = &captures[3];
            let normalised_lat_str = if matches!(lat_str.len(), 1 | 3 | 5) {
                // invalid syntax
                if lat_str.starts_with('0') {
                    warn!("Coordinate waypoints must not be abbreviated and start with a 0: {designator} (lat_str)");
                    return None;
                }
                format!("0{lat_str}")
            } else {
                lat_str.to_string()
            };
            let normalised_lng_str = if matches!(lng_str.len(), 2 | 4 | 6) {
                // invalid syntax
                if lng_str.starts_with('0') {
                    warn!("Coordinate waypoints must not be abbreviated and start with a 0: {designator} (lng_str)");
                    return None;
                }
                format!("0{lng_str}")
            } else {
                lng_str.to_string()
            };
            if normalised_lng_str.len() - normalised_lat_str.len() != 1 {
                warn!("Coordinate waypoints must have the same precision in lat/lon: {designator}");
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

    fn convert_fix(&self, designator: &str) -> Option<Coord> {
        self.vors
            .get(designator)
            .map(|vor| vor.coordinate)
            .or(self.ndbs.get(designator).map(|ndb| ndb.coordinate))
            .or(self.fixes.get(designator).map(|fix| fix.coordinate))
            .or(self
                .airports
                .get(designator)
                .map(|airport| airport.coordinate))
    }

    pub fn convert_designator(&self, designator: &str) -> Option<Coord> {
        self.convert_fix(designator)
            .or(Self::convert_coordinate(designator))
            .or(self.convert_range_bearing(designator))
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
    use geo::Coord;

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
            ndbs: [
                (
                    "MIQ".to_string(),
                    NDB {
                        designator: "MIQ".to_string(),
                        frequency: "426.000".to_string(),
                        coordinate: Coord {
                            y: 48.570_225,
                            x: 11.597_502_777_777_779,
                        },
                    },
                ),
                (
                    "SI".to_string(),
                    NDB {
                        designator: "SI".to_string(),
                        frequency: "410.000".to_string(),
                        coordinate: Coord {
                            y: 47.818_607_777_777_78,
                            x: 12.987_674_722_222_222,
                        },
                    },
                ),
            ]
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
                y: 46.333_333_333_333_336,
                x: -58.083_333_333_333_336
            }
        );
        assert_eq!(
            locs.convert_designator("462013N0580503W").unwrap(),
            Coord {
                y: 46.336_944_444_444_45,
                x: -58.084_166_666_666_67
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
        assert_eq!(
            locs.convert_designator("ARMUT070005").unwrap(),
            Coord {
                y: 49.750_911_853_173,
                x: 12.444_077_899_400_547,
            }
        );
        assert_eq!(
            locs.convert_designator("MIQ270060").unwrap(),
            Coord {
                y: 48.560_381_643_538_1,
                x: 10.091_991_772_311_63
            }
        );
        assert_eq!(
            locs.convert_designator("SI123456").unwrap(),
            Coord {
                y: 43.327_844_333_714_84,
                x: 21.728_708_973_319_613,
            }
        );
        // TODO test runways
    }
}
