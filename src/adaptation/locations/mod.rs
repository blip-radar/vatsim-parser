pub mod airways;

use std::collections::HashMap;

use geo_types::Coord;
use multimap::MultiMap;
use serde::Serialize;

use crate::{
    sct::{self, Sct},
    Location,
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
pub struct Locations {
    pub fixes: MultiMap<String, Fix>,
    pub vors: MultiMap<String, VOR>,
    pub ndbs: MultiMap<String, NDB>,
    pub airports: HashMap<String, Airport>,
    pub airways: FixAirwayMap,
    // TODO
    pub sids: Vec<String>,
    pub stars: Vec<String>,
}

impl Locations {
    pub(super) fn from_euroscope(sct: Sct, airways: FixAirwayMap) -> Self {
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
        Locations {
            fixes,
            vors,
            ndbs,
            airports: Airport::from_sct_airports(sct.airports, sct.runways),
            airways,
            sids: vec![],
            stars: vec![],
        }
    }

    pub(crate) fn convert_location(&self, loc: &Location) -> Option<Coord> {
        match loc {
            Location::Coordinate(c) => Some(*c),
            Location::Fix(wpt) => self.convert_designator(wpt),
        }
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
        // TODO test runways, r/b, "coord wpts"
    }
}
