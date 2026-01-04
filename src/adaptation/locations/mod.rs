pub mod airways;

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::OnceLock;

use geo::{point, Destination as _, Geodesic, Point};
use multimap::MultiMap;
use regex::Regex;
use serde::Serialize;
use tracing::{trace, warn};
use uom::si::f64::Length;
use uom::si::length::{meter, nautical_mile};

use crate::adaptation::locations::airways::AirwayGraph;
use crate::{
    ese::{Ese, SidStar},
    sct::{self, Sct},
    Location,
};

#[derive(Clone, Debug, Serialize)]
pub struct Fix {
    pub designator: String,
    pub coordinate: Point,
}
// FIXME format! performance? maybe use fixed point decimals, 6 decimals seems to be common (ca 1.1m)
// but our data is bad enough that we have to use .2
impl Hash for Fix {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.designator.hash(state);

        quantize(self.coordinate.x()).hash(state);
        quantize(self.coordinate.y()).hash(state);

        trace!(
            "hashed: {self:?} with {}: {}",
            self.designator,
            state.finish()
        );
    }
}
impl PartialEq for Fix {
    fn eq(&self, other: &Self) -> bool {
        let res = self.designator == other.designator
            && quantize(self.coordinate.x()) == quantize(other.coordinate.x())
            && quantize(self.coordinate.y()) == quantize(other.coordinate.y());

        trace!("{} == {}: {}", self.designator, other.designator, res);

        res
    }
}
impl Eq for Fix {}

#[derive(Clone, Debug, Serialize)]
pub struct GraphPosition(pub Point);

impl GraphPosition {
    fn quantize(&self) -> (i64, i64) {
        let lat = (self.0.y() * 1_000_000.0).round() as i64;
        let lon = (self.0.y() * 1_000_000.0).round() as i64;
        (lat, lon)
    }
}

impl PartialEq for GraphPosition {
    fn eq(&self, other: &Self) -> bool {
        self.quantize() == other.quantize()
    }
}

impl Eq for GraphPosition {}

const DECIMALS: u32 = 2;

fn quantize(v: f64) -> i64 {
    let factor = 10_i64.pow(DECIMALS) as f64;
    (v * factor).round() as i64
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct NDB {
    pub designator: String,
    pub frequency: String,
    pub coordinate: Point,
}
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct VOR {
    pub designator: String,
    pub frequency: String,
    pub coordinate: Point,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Runway {
    pub designators: (String, String),
    pub headings: (u32, u32),
    pub location: (Point, Point),
    pub aerodrome: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Airport {
    pub designator: String,
    pub coordinate: Point,
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

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct STAR {
    pub name: String,
    pub airport: String,
    pub runway: Option<String>,
    pub waypoints: Vec<Fix>,
}

impl PartialEq<STAR> for String {
    fn eq(&self, other: &STAR) -> bool {
        other.name == *self
    }
}
impl PartialEq<String> for STAR {
    fn eq(&self, other: &String) -> bool {
        self.name == *other
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Locations {
    pub fixes: MultiMap<String, Fix>,
    pub vors: MultiMap<String, VOR>,
    pub ndbs: MultiMap<String, NDB>,
    pub airports: HashMap<String, Airport>,
    pub airways: AirwayGraph,
    pub sids: HashMap<String, MultiMap<String, SID>>,
    pub stars: HashMap<String, MultiMap<String, STAR>>,
}

fn coord_regex() -> &'static Regex {
    static COORD_RE: OnceLock<Regex> = OnceLock::new();
    COORD_RE.get_or_init(|| Regex::new(r"^(\d{1,6})(N|S)(\d{2,7})(E|W)$").unwrap())
}

fn range_bearing_regex() -> &'static Regex {
    static RANGE_BEARING_RE: OnceLock<Regex> = OnceLock::new();
    RANGE_BEARING_RE.get_or_init(|| Regex::new(r"^([0-9A-Z]{2,5})(\d{3})(\d{3})$").unwrap())
}

impl Locations {
    pub(super) fn from_euroscope(
        sct: Sct,
        ese: Ese,
        // airways: FixAirwayMap,
        airways2: AirwayGraph,
    ) -> Self {
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
            // airways,
            airways: airways2,
            sids: HashMap::new(),
            stars: HashMap::new(),
        };
        ese.sids_stars
            .into_iter()
            .for_each(|sid_star| match sid_star {
                SidStar::Sid(sid) => {
                    let adap_sid = SID {
                        waypoints: sid
                            .waypoints
                            .into_iter()
                            .filter_map(|wpt| {
                                let fix = locations.convert_designator(&wpt);
                                if fix.is_none() {
                                    warn!(
                                        "SID {} {} {}: waypoint {wpt} not found",
                                        sid.airport,
                                        sid.name,
                                        sid.runway.as_deref().unwrap_or("")
                                    );
                                }
                                fix
                            })
                            .collect(),
                        name: sid.name.clone(),
                        airport: sid.airport.clone(),
                        runway: sid.runway,
                    };
                    locations
                        .sids
                        .entry(sid.airport)
                        .and_modify(|airport_map| {
                            airport_map.insert(sid.name.clone(), adap_sid.clone());
                        })
                        .or_insert_with(|| MultiMap::from_iter([(sid.name, adap_sid)]));
                }
                SidStar::Star(star) => {
                    let adap_star = STAR {
                        waypoints: star
                            .waypoints
                            .into_iter()
                            .filter_map(|wpt| {
                                let fix = locations.convert_designator(&wpt);
                                if fix.is_none() {
                                    warn!(
                                        "STAR {} {} {}: waypoint {wpt} not found",
                                        star.airport,
                                        star.name,
                                        star.runway.as_deref().unwrap_or("")
                                    );
                                }
                                fix
                            })
                            .collect(),
                        name: star.name.clone(),
                        airport: star.airport.clone(),
                        runway: star.runway,
                    };
                    locations
                        .stars
                        .entry(star.airport.clone())
                        .and_modify(|airport_map| {
                            airport_map.insert(star.name.clone(), adap_star.clone());
                        })
                        .or_insert_with(|| MultiMap::from_iter([(star.name.clone(), adap_star)]));
                }
            });

        locations
    }

    pub fn convert_location(&self, loc: &Location) -> Option<Point> {
        match loc {
            Location::Coordinate(c) => Some(*c),
            Location::Fix(wpt) => self.convert_designator(wpt).map(|f| f.coordinate),
        }
    }

    fn convert_range_bearing(&self, designator: &str) -> Option<Fix> {
        range_bearing_regex()
            .captures(designator)
            .and_then(|captures| {
                let fix = &captures[1];
                // TODO magnetic
                let bearing: f64 = captures[2].parse().unwrap();
                let range = Length::new::<nautical_mile>(captures[3].parse::<f64>().unwrap());

                self.convert_fix(fix).map(|f| Fix {
                    designator: designator.to_string(),
                    coordinate: Geodesic.destination(f.coordinate, bearing, range.get::<meter>()),
                })
            })
    }

    fn convert_coordinate(designator: &str) -> Option<Fix> {
        coord_regex().captures(designator).and_then(|captures| {
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
            if normalised_lng_str.len().saturating_sub(normalised_lat_str.len()) != 1 {
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
            Some(Fix {
                designator: format!("{normalised_lat_str}{n_s}{normalised_lng_str}{w_e}"),
                coordinate: point! {
                    x: if w_e == "E" { 1.0 } else { -1.0 } * lng,
                    y: if n_s == "N" { 1.0 } else { -1.0 } * lat,
                }
            })
        })
    }

    fn convert_rwy(&self, designator: &str) -> Option<Fix> {
        matches!(designator.len(), 6..=7)
            .then(|| {
                designator
                    .split_at_checked(4)
                    .and_then(|(ad_designator, rwy_designator)| {
                        self.airports.get(ad_designator).and_then(|airport| {
                            airport.runways.iter().find_map(|rwy| {
                                if rwy.designators.0 == rwy_designator {
                                    Some(Fix {
                                        designator: format!(
                                            "{}{}",
                                            airport.designator, rwy.designators.0
                                        ),
                                        coordinate: rwy.location.0,
                                    })
                                } else if rwy.designators.1 == rwy_designator {
                                    Some(Fix {
                                        designator: format!(
                                            "{}{}",
                                            airport.designator, rwy.designators.1
                                        ),
                                        coordinate: rwy.location.1,
                                    })
                                } else {
                                    None
                                }
                            })
                        })
                    })
            })
            .flatten()
    }

    fn convert_fix(&self, designator: &str) -> Option<Fix> {
        self.vors
            .get(designator)
            .map(|vor| Fix {
                designator: vor.designator.clone(),
                coordinate: vor.coordinate,
            })
            .or(self.ndbs.get(designator).map(|ndb| Fix {
                designator: ndb.designator.clone(),
                coordinate: ndb.coordinate,
            }))
            .or(self.fixes.get(designator).cloned())
            .or(self.airports.get(designator).map(|airport| Fix {
                designator: airport.designator.clone(),
                coordinate: airport.coordinate,
            }))
            .or(self.convert_rwy(designator))
    }

    pub fn convert_designator(&self, designator: &str) -> Option<Fix> {
        self.convert_fix(designator)
            .or(Self::convert_coordinate(designator))
            .or(self.convert_range_bearing(designator))
    }

    pub fn contains_designator(&self, designator: &str) -> bool {
        self.vors.contains_key(designator)
            || self.ndbs.contains_key(designator)
            || self.fixes.contains_key(designator)
            || self.airports.contains_key(designator)
            || self.convert_rwy(designator).is_some()
    }
}

#[cfg(test)]
mod test {
    use geo::point;

    use crate::adaptation::locations::{Airport, Fix, Locations, Runway, NDB, VOR};

    #[test]
    fn test_get_by_wpt() {
        let locs = Locations {
            fixes: [(
                "ARMUT".to_string(),
                Fix {
                    designator: "ARMUT".to_string(),
                    coordinate: point! {
                        x: 12.323_332_777_777_777,
                        y: 49.722_499_722_222_224,
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
                        coordinate: point! {
                            x: 11.597_502_777_777_779,
                            y: 48.570_225,
                        },
                    },
                ),
                (
                    "SI".to_string(),
                    NDB {
                        designator: "SI".to_string(),
                        frequency: "410.000".to_string(),
                        coordinate: point! {
                            x: 12.987_674_722_222_222,
                            y: 47.818_607_777_777_78,
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
                    coordinate: point! {
                        x: 11.816_535_833_333_335,
                        y: 48.180_393_888_888_89,
                    },
                },
            )]
            .into_iter()
            .collect(),
            airports: [(
                "EDDM".to_string(),
                Airport {
                    designator: "EDDM".to_string(),
                    runways: vec![
                        Runway {
                            designators: ("08R".to_string(), "26L".to_string()),
                            headings: (80, 260),
                            location: (
                                point! {
                                  x: 11.751_016_944_444_444,
                                  y: 48.340_668_888_888_89
                                },
                                point! {
                                  x: 11.804_613_888_888_89,
                                  y: 48.344_796_944_444_45
                                },
                            ),
                            aerodrome: "EDDM".to_string(),
                        },
                        Runway {
                            designators: ("08L".to_string(), "26R".to_string()),
                            headings: (80, 260),
                            location: (
                                point! {
                                  x: 11.767_549_722_222_222,
                                  y: 48.362_766_944_444_445
                                },
                                point! {
                                  x: 11.821_171_944_444_444,
                                  y: 48.366_885_833_333_335
                                },
                            ),
                            aerodrome: "EDDM".to_string(),
                        },
                    ],
                    coordinate: point! {
                        x: 11.786_085_833_333_333,
                        y: 48.353_782_777_777_78,
                    },
                },
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        };

        assert_eq!(
            locs.convert_designator("MIQ").unwrap(),
            Fix {
                designator: "MIQ".to_string(),
                coordinate: point! {
                    x: 11.597_502_777_777_779,
                    y: 48.570_225,
                }
            }
        );
        assert_eq!(
            locs.convert_designator("OTT").unwrap(),
            Fix {
                designator: "OTT".to_string(),
                coordinate: point! {
                    x: 11.816_535_833_333_335,
                    y: 48.180_393_888_888_89,
                }
            }
        );
        assert_eq!(
            locs.convert_designator("EDDM").unwrap(),
            Fix {
                designator: "EDDM".to_string(),
                coordinate: point! {
                    x: 11.786_085_833_333_333,
                    y: 48.353_782_777_777_78,
                }
            }
        );
        assert_eq!(
            locs.convert_designator("EDDM26R").unwrap(),
            Fix {
                designator: "EDDM26R".to_string(),
                coordinate: point! {
                    x: 11.821_171_944_444_444,
                    y: 48.366_885_833_333_335,
                }
            }
        );
        assert_eq!(
            locs.convert_designator("ARMUT").unwrap(),
            Fix {
                designator: "ARMUT".to_string(),
                coordinate: point! {
                    x: 12.323_332_777_777_777,
                    y: 49.722_499_722_222_224,
                }
            }
        );
        assert_eq!(locs.convert_designator("OZE"), None);
        assert_eq!(
            locs.convert_designator("46N078W").unwrap(),
            Fix {
                designator: "46N078W".to_string(),
                coordinate: point! { x: -78.0, y: 46.0 }
            }
        );
        assert_eq!(
            locs.convert_designator("4620N05805W").unwrap(),
            Fix {
                designator: "4620N05805W".to_string(),
                coordinate: point! {
                    x: -58.083_333_333_333_336,
                    y: 46.333_333_333_333_336,
                }
            }
        );
        assert_eq!(
            locs.convert_designator("462013N0580503W").unwrap(),
            Fix {
                designator: "462013N0580503W".to_string(),
                coordinate: point! {
                    x: -58.084_166_666_666_67,
                    y: 46.336_944_444_444_45,
                }
            }
        );
        assert_eq!(
            locs.convert_designator("4N40W").unwrap(),
            Fix {
                designator: "04N040W".to_string(),
                coordinate: point! { x: -40.0, y: 4.0 }
            }
        );
        assert_eq!(
            locs.convert_designator("04N40W").unwrap(),
            Fix {
                designator: "04N040W".to_string(),
                coordinate: point! { x: -40.0, y: 4.0 }
            }
        );
        assert_eq!(locs.convert_designator("4N04W"), None);
        assert_eq!(locs.convert_designator("04N04W"), None);
        assert_eq!(
            locs.convert_designator("400N4000W").unwrap(),
            Fix {
                designator: "0400N04000W".to_string(),
                coordinate: point! { x: -40.0, y: 4.0 }
            }
        );
        assert_eq!(
            locs.convert_designator("0400N4000W").unwrap(),
            Fix {
                designator: "0400N04000W".to_string(),
                coordinate: point! { x: -40.0, y: 4.0 }
            }
        );
        assert_eq!(locs.convert_designator("400N0400W"), None);
        assert_eq!(locs.convert_designator("0400N0400W"), None);
        assert_eq!(
            locs.convert_designator("ARMUT070005").unwrap(),
            Fix {
                designator: "ARMUT070005".to_string(),
                coordinate: point! {
                    x: 12.444_077_899_400_547,
                    y: 49.750_911_853_173,
                }
            }
        );
        assert_eq!(
            locs.convert_designator("MIQ270060").unwrap(),
            Fix {
                designator: "MIQ270060".to_string(),
                coordinate: point! {
                    x: 10.091_991_772_311_63,
                    y: 48.560_381_643_538_1,
                }
            }
        );
        assert_eq!(
            locs.convert_designator("SI123456").unwrap(),
            Fix {
                designator: "SI123456".to_string(),
                coordinate: point! {
                    x: 21.728_708_973_319_613,
                    y: 43.327_844_333_714_84,
                }
            }
        );
    }
}
