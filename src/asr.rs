use std::io;

use geo::Coord;
use pest::{iterators::Pair, Parser};
use pest_derive::Parser;
use serde::Serialize;
use thiserror::Error;
use tracing::warn;

use crate::TwoKeyMap;

use super::read_to_string;

#[derive(Parser)]
#[grammar = "pest/asr.pest"]
pub struct AsrParser;

#[derive(Error, Debug)]
pub enum AsrError {
    #[error("failed to parse .asr file: {0}")]
    Parse(#[from] pest::error::Error<Rule>),
    #[error("failed to read .asr file: {0}")]
    FileRead(#[from] io::Error),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Leader {
    Miles(u8),
    Minutes(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SimulationMode {
    Radar,
    Ground,
    // ...
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum DisplayType {
    Radar,
    GroundRadar,
}

enum AsrData {
    Above(Option<u32>),
    Below(Option<u32>),
    DisablePanning(bool),
    DisableZooming(bool),
    DisplayRotation(f64),
    DisplayType(DisplayType),
    DisplayTypeGeoReferenced(bool),
    DisplayTypeNeedRadarContent(bool),
    HistoryDots(u8),
    Leader(Leader),
    SectorFile(String),
    SectorTitle(String),
    ShowLeader(bool),
    ShowC(bool),
    ShowStandby(bool),
    SimulationMode(SimulationMode),
    TagFamily(String),
    TurnLeader(bool),
    WindowArea((Coord, Coord)),
    PluginSetting((String, String, String)),
    Runway((String, String, String, AsrMapRunwayType)),
    Airport((String, AsrMapFixType)),
    Vor((String, AsrMapNavaidType)),
    Ndb((String, AsrMapNavaidType)),
    Fix((String, AsrMapFixType)),
    LowAirway((String, AsrMapAirwayType)),
    HighAirway((String, AsrMapAirwayType)),
    Sid(String),
    Star(String),
    FreeText((String, String)),
    Geo(String),
    GroundNetwork((String, AsrMapGroundNetworkType)),
    Region(String),
    ArtccBoundary(String),
    ArtccLowBoundary(String),
    ArtccHighBoundary(String),
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum AsrMapFixType {
    Name,
    Symbol,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum AsrMapNavaidType {
    Name,
    Symbol,
    Frequency,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum AsrMapRunwayType {
    Name,
    Centerline,
    // TODO extended centerline types
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum AsrMapAirwayType {
    Name,
    Line,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum AsrMapGroundNetworkType {
    Exit,
    Taxiway,
    TerminalTaxiway,
}

#[derive(Default, Clone, Debug, Serialize, PartialEq, Eq)]
pub struct AsrMap {
    pub runways: Vec<(String, String, String, AsrMapRunwayType)>,
    pub airports: Vec<(String, AsrMapFixType)>,
    pub vors: Vec<(String, AsrMapNavaidType)>,
    pub ndbs: Vec<(String, AsrMapNavaidType)>,
    pub fixes: Vec<(String, AsrMapFixType)>,
    pub low_airways: Vec<(String, AsrMapAirwayType)>,
    pub high_airways: Vec<(String, AsrMapAirwayType)>,
    pub sids: Vec<String>,
    pub stars: Vec<String>,
    pub free_text: Vec<(String, String)>,
    pub geo: Vec<String>,
    pub ground_network: Vec<(String, AsrMapGroundNetworkType)>,
    pub regions: Vec<String>,
    pub artcc_boundary: Vec<String>,
    pub artcc_low_boundary: Vec<String>,
    pub artcc_high_boundary: Vec<String>,
}
impl From<Vec<AsrData>> for AsrMap {
    fn from(data: Vec<AsrData>) -> Self {
        data.into_iter().fold(AsrMap::default(), |mut acc, data| {
            match data {
                AsrData::Runway(rwy) => acc.runways.push(rwy),
                AsrData::Airport(airport) => acc.airports.push(airport),
                AsrData::Vor(v) => acc.vors.push(v),
                AsrData::Ndb(n) => acc.ndbs.push(n),
                AsrData::Fix(f) => acc.fixes.push(f),
                AsrData::LowAirway(la) => acc.low_airways.push(la),
                AsrData::HighAirway(ha) => acc.high_airways.push(ha),
                AsrData::Sid(sid) => acc.sids.push(sid),
                AsrData::Star(star) => acc.stars.push(star),
                AsrData::FreeText(ft) => acc.free_text.push(ft),
                AsrData::Geo(geo) => acc.geo.push(geo),
                AsrData::GroundNetwork(gn) => acc.ground_network.push(gn),
                AsrData::Region(r) => acc.regions.push(r),
                AsrData::ArtccBoundary(ab) => acc.artcc_boundary.push(ab),
                AsrData::ArtccLowBoundary(alb) => acc.artcc_low_boundary.push(alb),
                AsrData::ArtccHighBoundary(ahb) => acc.artcc_high_boundary.push(ahb),
                _ => (),
            };
            acc
        })
    }
}

/// Data of Euroscope .asr files, these settings are not necessarily all respected in this client
#[derive(Clone, Debug, Serialize)]
pub struct Asr {
    // ABOVE – xxxxx. The value if you choose not to display aircraft above xxxxx feet altitude (your ceiling level). Zero indicates no filter at all.
    pub above: Option<u32>,
    // BELOW – xxxxx. The value if you choose not to display aircraft below xxxxx feet altitude (your floor level). Zero indicates no filter at all.
    pub below: Option<u32>,
    /// DISABLEPANNING
    pub disable_panning: bool,
    /// DISABLEZOOOMING
    pub disable_zooming: bool,
    /// DisplayRotation
    pub display_rotation: f64,
    // DisplayTypeName – The name of the screen type. The default value is ‘Standard ES radar screen‘. Other may be created by the plug-ins.
    pub display_type: DisplayType,
    // DisplayTypeGeoReferenced – It indicates if coordinates are latitude/longitude pairs or just pixels.
    pub display_type_geo_referenced: bool,
    // DisplayTypeNeedRadarContent – It indicates that background SCT file elements are drawn for the screen or not.
    pub display_type_need_radar_content: bool,
    // HISTORYDOTS – The number of history trails appearing for each aircraft.
    pub history_dots: u8,
    // LEADER – The length of the leader line. Positive values are interpreted as NM, negative as MIN.
    pub leader: Leader,
    /// SECTORFILE – The path of your current sector file this ASR is used for. When you open an ASR it will look if the sector file is loaded or not. If not then it loads the appropriate one.
    pub sector_file: String,
    /// SECTORTITLE – Just a quick access to the title to show in the popup list.
    pub sector_title: String,
    /// SHOWLEADER – Indicates if the leader line should be shown as default or not.
    pub show_leader: bool,
    /// SHOWC – (value if 1 if checked or 0 if unchecked) “Show squawk C aircraft” option.
    pub show_c: bool,
    /// SHOWSB – “Show squawk STBY aircraft” option.
    pub show_standby: bool,
    /// SIMULATION_MODE – The ID of the simulation mode (professional radar, easy radar and the two ground modes).ExecStart=/nix/store/r9nb4ap2ivjc15adbw177bjm5nz3axj7-unit-script-wg-quick-wgfsg-start/bin/wg-quick-wgfsg-start
    pub simulation_mode: SimulationMode,
    /// TAGFAMILY – The name of the tag family used (generally MATIAS (built in)).
    pub tag_family: String,
    /// TURNLEADER – It indicates a route following leader line.
    pub turn_leader: bool,
    /// WINDOWAREA – param1:param2:param3:param4 – The geographic coordinates in degrees of the bottom left corner and of the top right corner of the scope. It is important that even if you do not change any settings, just zoom in and out and pan, this value is most likely to be updated. In this way it is quite normal that you will be prompted at nearly all ASR close to decide weather to save or cancel the update of the area.
    pub window_area: (Coord, Coord),
    /// plugin name, key -> value (Euroscope example: PLUGIN:TopSky plugin:HideMapData:TWR)
    pub plugin_settings: TwoKeyMap<String, String, String>,
    /// Elements to show on the map if ASR is selected
    pub map: AsrMap,
}
// ?? individual sector file elements – Then follows the list of all your checked items in the display dialog. You can not save the SECTORLINE and SECTOR elements as they can be switched on just for debugging purposes and not for next session display.

pub type AsrResult = Result<Asr, AsrError>;

fn to_bool(data: &str) -> Option<bool> {
    match data {
        "0" => Some(false),
        "1" => Some(true),
        _ => None,
    }
}

fn parse_setting(pair: Pair<Rule>) -> Option<AsrData> {
    let rule = pair.as_rule();
    let mut inner = pair.into_inner();
    match rule {
        Rule::display_type => {
            match inner.next().unwrap().as_str() {
                "Ground Radar display" => Some(AsrData::DisplayType(DisplayType::GroundRadar)),
                "Standard ES radar screen" => Some(AsrData::DisplayType(DisplayType::Radar)),
                // default to Radar for unknown types
                _ => Some(AsrData::DisplayType(DisplayType::Radar)),
            }
        }
        Rule::display_type_need_radar_content => {
            to_bool(inner.next().unwrap().as_str()).map(AsrData::DisplayTypeNeedRadarContent)
        }
        Rule::display_type_geo_referenced => {
            to_bool(inner.next().unwrap().as_str()).map(AsrData::DisplayTypeGeoReferenced)
        }
        Rule::sector_file => Some(AsrData::SectorFile(
            inner.next().unwrap().as_str().to_string(),
        )),
        Rule::sector_title => Some(AsrData::SectorTitle(
            inner.next().unwrap().as_str().to_string(),
        )),
        Rule::show_c => to_bool(inner.next().unwrap().as_str()).map(AsrData::ShowC),
        Rule::show_standby => to_bool(inner.next().unwrap().as_str()).map(AsrData::ShowStandby),
        Rule::above => {
            let filter = inner.next().unwrap().as_str().parse().unwrap();
            Some(if filter == 0 {
                AsrData::Above(None)
            } else {
                AsrData::Above(Some(filter))
            })
        }
        Rule::below => {
            let filter = inner.next().unwrap().as_str().parse().unwrap();
            Some(if filter == 0 {
                AsrData::Below(None)
            } else {
                AsrData::Below(Some(filter))
            })
        }
        Rule::leader => {
            let leader = inner.next().unwrap().as_str().parse::<i8>().unwrap();
            Some(if leader > 0 {
                #[allow(clippy::cast_sign_loss)]
                AsrData::Leader(Leader::Miles(leader as u8))
            } else {
                AsrData::Leader(Leader::Minutes(leader.unsigned_abs()))
            })
        }
        Rule::show_leader => to_bool(inner.next().unwrap().as_str()).map(AsrData::ShowLeader),
        Rule::turn_leader => to_bool(inner.next().unwrap().as_str()).map(AsrData::TurnLeader),
        Rule::history_dots => Some(AsrData::HistoryDots(
            inner.next().unwrap().as_str().parse().unwrap(),
        )),
        Rule::simulation_mode => Some(AsrData::SimulationMode(
            match inner.next().unwrap().as_str() {
                "1" => SimulationMode::Radar,
                "4" => SimulationMode::Ground,
                _ => SimulationMode::Radar,
            },
        )),
        Rule::disable_panning => {
            to_bool(inner.next().unwrap().as_str()).map(AsrData::DisablePanning)
        }
        Rule::disable_zooming => {
            to_bool(inner.next().unwrap().as_str()).map(AsrData::DisableZooming)
        }
        Rule::display_rotation => Some(AsrData::DisplayRotation(
            inner.next().unwrap().as_str().parse().unwrap(),
        )),
        Rule::tag_family => Some(AsrData::TagFamily(
            inner.next().unwrap().as_str().to_string(),
        )),
        Rule::window_area => {
            let lat1 = inner.next().unwrap().as_str().parse().unwrap();
            let lng1 = inner.next().unwrap().as_str().parse().unwrap();
            let lat2 = inner.next().unwrap().as_str().parse().unwrap();
            let lng2 = inner.next().unwrap().as_str().parse().unwrap();
            Some(AsrData::WindowArea((
                Coord { x: lng1, y: lat1 },
                Coord { x: lng2, y: lat2 },
            )))
        }
        Rule::plugin => {
            let plugin = inner.next().unwrap().as_str().to_string();
            let key = inner.next().unwrap().as_str().to_string();
            let value = inner.next().unwrap().as_str().to_string();
            Some(AsrData::PluginSetting((plugin, key, value)))
        }
        Rule::runways => {
            let airport = inner.next().unwrap().as_str().to_string();
            let desig_a = inner.next().unwrap().as_str().to_string();
            let desig_b = inner.next().unwrap().as_str().to_string();
            let val = inner.next().unwrap();
            match val.as_rule() {
                Rule::centerline => Some(AsrData::Runway((
                    airport,
                    desig_a,
                    desig_b,
                    AsrMapRunwayType::Centerline,
                ))),
                Rule::name => Some(AsrData::Runway((
                    airport,
                    desig_a,
                    desig_b,
                    AsrMapRunwayType::Name,
                ))),
                Rule::ext_centerline => {
                    warn!(
                        "extended centerline not implemented: {airport} {desig_a}-{desig_b}, {}",
                        val.as_str()
                    );
                    None
                }

                rule => unreachable!("{rule:?}"),
            }
        }
        Rule::airports => {
            let name = inner.next().unwrap().as_str().to_string();
            match inner.next().unwrap().as_rule() {
                Rule::symbol => Some(AsrData::Airport((name, AsrMapFixType::Symbol))),
                Rule::name => Some(AsrData::Airport((name, AsrMapFixType::Name))),
                rule => unreachable!("{rule:?}"),
            }
        }
        Rule::fixes => {
            let name = inner.next().unwrap().as_str().to_string();
            match inner.next().unwrap().as_rule() {
                Rule::symbol => Some(AsrData::Fix((name, AsrMapFixType::Symbol))),
                Rule::name => Some(AsrData::Fix((name, AsrMapFixType::Name))),
                rule => unreachable!("{rule:?}"),
            }
        }
        Rule::ndbs => {
            let name = inner.next().unwrap().as_str().to_string();
            match inner.next().unwrap().as_rule() {
                Rule::symbol => Some(AsrData::Ndb((name, AsrMapNavaidType::Symbol))),
                Rule::name => Some(AsrData::Ndb((name, AsrMapNavaidType::Name))),
                Rule::frequency => Some(AsrData::Ndb((name, AsrMapNavaidType::Frequency))),
                rule => unreachable!("{rule:?}"),
            }
        }
        Rule::vors => {
            let name = inner.next().unwrap().as_str().to_string();
            match inner.next().unwrap().as_rule() {
                Rule::symbol => Some(AsrData::Vor((name, AsrMapNavaidType::Symbol))),
                Rule::name => Some(AsrData::Vor((name, AsrMapNavaidType::Name))),
                Rule::frequency => Some(AsrData::Vor((name, AsrMapNavaidType::Frequency))),
                rule => unreachable!("{rule:?}"),
            }
        }
        Rule::sids => Some(AsrData::Sid(inner.next().unwrap().as_str().to_string())),
        Rule::stars => Some(AsrData::Star(inner.next().unwrap().as_str().to_string())),
        Rule::low_airways => {
            let name = inner.next().unwrap().as_str().to_string();
            match inner.next().unwrap().as_rule() {
                Rule::line => Some(AsrData::LowAirway((name, AsrMapAirwayType::Line))),
                Rule::name => Some(AsrData::LowAirway((name, AsrMapAirwayType::Name))),
                rule => unreachable!("{rule:?}"),
            }
        }
        Rule::high_airways => {
            let name = inner.next().unwrap().as_str().to_string();
            match inner.next().unwrap().as_rule() {
                Rule::line => Some(AsrData::HighAirway((name, AsrMapAirwayType::Line))),
                Rule::name => Some(AsrData::HighAirway((name, AsrMapAirwayType::Name))),
                rule => unreachable!("{rule:?}"),
            }
        }
        Rule::free_text => Some(AsrData::FreeText((
            inner.next().unwrap().as_str().to_string(),
            inner.next().unwrap().as_str().to_string(),
        ))),
        Rule::artcc_boundary => Some(AsrData::ArtccBoundary(
            inner.next().unwrap().as_str().to_string(),
        )),
        Rule::artcc_high_boundary => Some(AsrData::ArtccHighBoundary(
            inner.next().unwrap().as_str().to_string(),
        )),
        Rule::artcc_low_boundary => Some(AsrData::ArtccLowBoundary(
            inner.next().unwrap().as_str().to_string(),
        )),
        Rule::geo => Some(AsrData::Geo(inner.next().unwrap().as_str().to_string())),
        Rule::ground_network => {
            let name = inner.next().unwrap().as_str().to_string();
            match inner.next().unwrap().as_rule() {
                Rule::exit => Some(AsrData::GroundNetwork((
                    name,
                    AsrMapGroundNetworkType::Exit,
                ))),
                Rule::taxiway => Some(AsrData::GroundNetwork((
                    name,
                    AsrMapGroundNetworkType::Taxiway,
                ))),
                Rule::terminal_taxiway => Some(AsrData::GroundNetwork((
                    name,
                    AsrMapGroundNetworkType::TerminalTaxiway,
                ))),
                rule => unreachable!("{rule:?}"),
            }
        }
        Rule::regions => Some(AsrData::Region(inner.next().unwrap().as_str().to_string())),
        Rule::EOI => None,
        rule => unreachable!("Unhandled rule: {rule:?}"),
    }
}

impl Asr {
    pub fn parse(content: &[u8]) -> AsrResult {
        let unparsed_file = read_to_string(content)?;
        let sections = AsrParser::parse(Rule::asr, &unparsed_file).map(|mut pairs| {
            pairs
                .next()
                .unwrap()
                .into_inner()
                .filter_map(parse_setting)
                .collect::<Vec<_>>()
        })?;

        let above = sections
            .iter()
            .find_map(|data| {
                if let AsrData::Above(above) = data {
                    Some(*above)
                } else {
                    None
                }
            })
            .unwrap_or(None);
        let below = sections
            .iter()
            .find_map(|data| {
                if let AsrData::Below(below) = data {
                    Some(*below)
                } else {
                    None
                }
            })
            .unwrap_or(None);
        let disable_panning = sections
            .iter()
            .find_map(|data| {
                if let AsrData::DisablePanning(val) = data {
                    Some(*val)
                } else {
                    None
                }
            })
            .unwrap_or(false);
        let disable_zooming = sections
            .iter()
            .find_map(|data| {
                if let AsrData::DisableZooming(val) = data {
                    Some(*val)
                } else {
                    None
                }
            })
            .unwrap_or(false);
        let display_rotation = sections
            .iter()
            .find_map(|data| {
                if let AsrData::DisplayRotation(val) = data {
                    Some(*val)
                } else {
                    None
                }
            })
            .unwrap_or(0.0);
        let display_type = sections
            .iter()
            .find_map(|data| {
                if let AsrData::DisplayType(val) = data {
                    Some(*val)
                } else {
                    None
                }
            })
            .unwrap_or(DisplayType::Radar);
        let display_type_geo_referenced = sections
            .iter()
            .find_map(|data| {
                if let AsrData::DisplayTypeGeoReferenced(val) = data {
                    Some(*val)
                } else {
                    None
                }
            })
            .unwrap_or(true);
        let display_type_need_radar_content = sections
            .iter()
            .find_map(|data| {
                if let AsrData::DisplayTypeNeedRadarContent(val) = data {
                    Some(*val)
                } else {
                    None
                }
            })
            .unwrap_or(true);
        let history_dots = sections
            .iter()
            .find_map(|data| {
                if let AsrData::HistoryDots(val) = data {
                    Some(*val)
                } else {
                    None
                }
            })
            .unwrap_or(5);
        let leader = sections
            .iter()
            .find_map(|data| {
                if let AsrData::Leader(val) = data {
                    Some(*val)
                } else {
                    None
                }
            })
            .unwrap_or(Leader::Minutes(3));
        let sector_file = sections
            .iter()
            .find_map(|data| {
                if let AsrData::SectorFile(val) = data {
                    Some(val.clone())
                } else {
                    None
                }
            })
            .unwrap_or(String::new());
        let sector_title = sections
            .iter()
            .find_map(|data| {
                if let AsrData::SectorTitle(val) = data {
                    Some(val.clone())
                } else {
                    None
                }
            })
            .unwrap_or(String::new());
        let show_leader = sections
            .iter()
            .find_map(|data| {
                if let AsrData::ShowLeader(val) = data {
                    Some(*val)
                } else {
                    None
                }
            })
            .unwrap_or(false);
        let show_c = sections
            .iter()
            .find_map(|data| {
                if let AsrData::ShowC(val) = data {
                    Some(*val)
                } else {
                    None
                }
            })
            .unwrap_or(true);
        let show_standby = sections
            .iter()
            .find_map(|data| {
                if let AsrData::ShowStandby(val) = data {
                    Some(*val)
                } else {
                    None
                }
            })
            .unwrap_or(false);
        let simulation_mode = sections
            .iter()
            .find_map(|data| {
                if let AsrData::SimulationMode(val) = data {
                    Some(*val)
                } else {
                    None
                }
            })
            .unwrap_or(SimulationMode::Radar);
        let tag_family = sections
            .iter()
            .find_map(|data| {
                if let AsrData::TagFamily(val) = data {
                    Some(val.clone())
                } else {
                    None
                }
            })
            .unwrap_or("Matias (built in)".to_string());
        let turn_leader = sections
            .iter()
            .find_map(|data| {
                if let AsrData::TurnLeader(val) = data {
                    Some(*val)
                } else {
                    None
                }
            })
            .unwrap_or(false);
        let window_area = sections
            .iter()
            .find_map(|data| {
                if let AsrData::WindowArea(val) = data {
                    Some(*val)
                } else {
                    None
                }
            })
            .unwrap_or((
                Coord {
                    x: 6.678_287,
                    y: 46.529_122,
                },
                Coord {
                    x: 16.599_900,
                    y: 50.105_536,
                },
            ));
        let plugin_settings = TwoKeyMap(
            sections
                .iter()
                .filter_map(|data| {
                    if let AsrData::PluginSetting((plugin, key, value)) = data {
                        Some(((plugin.clone(), key.clone()), value.clone()))
                    } else {
                        None
                    }
                })
                .collect(),
        );
        let map = AsrMap::from(sections);
        Ok(Asr {
            above,
            below,
            disable_panning,
            disable_zooming,
            display_rotation,
            display_type,
            display_type_geo_referenced,
            display_type_need_radar_content,
            history_dots,
            leader,
            sector_file,
            sector_title,
            show_leader,
            show_c,
            show_standby,
            simulation_mode,
            tag_family,
            turn_leader,
            window_area,
            plugin_settings,
            map,
        })
    }
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, fs};

    use geo::Coord;
    use pretty_assertions_sorted::assert_eq_sorted;

    use crate::{
        asr::{
            AsrMap, AsrMapFixType, AsrMapNavaidType, AsrMapRunwayType, DisplayType, Leader,
            SimulationMode,
        },
        TwoKeyMap,
    };

    use super::Asr;

    #[test]
    fn test() {
        let asr_contents = fs::read("./fixtures/EDDM_APP.asr").unwrap();
        let asr = Asr::parse(&asr_contents).unwrap();
        assert_eq!(asr.display_type, DisplayType::Radar);
        assert!(asr.display_type_need_radar_content);
        assert!(asr.display_type_geo_referenced);
        assert_eq!(asr.sector_file, String::new());
        assert_eq!(asr.sector_title, String::new());
        // TODO runways
        assert!(asr.show_c);
        assert!(!asr.show_standby);
        assert_eq!(asr.below, None);
        assert_eq!(asr.above, None);
        assert_eq!(asr.leader, Leader::Miles(3));
        assert!(!asr.show_leader);
        assert!(!asr.turn_leader);
        assert_eq!(asr.history_dots, 0);
        assert_eq!(asr.simulation_mode, SimulationMode::Radar);
        assert!(!asr.disable_panning);
        assert!(!asr.disable_zooming);
        assert!((asr.display_rotation - 0.0).abs() < f64::EPSILON);
        assert_eq!(asr.tag_family, "iCAS2-APP".to_string());
        assert_eq!(
            asr.window_area,
            (
                Coord {
                    x: 9.936_633,
                    y: 47.687_116,
                },
                Coord {
                    x: 13.635_539,
                    y: 49.020_449,
                }
            )
        );
        assert_eq_sorted!(
            asr.map,
            AsrMap {
                runways: vec![
                    (
                        "EDDM".to_string(),
                        "08L".to_string(),
                        "26R".to_string(),
                        AsrMapRunwayType::Centerline
                    ),
                    (
                        "EDDM".to_string(),
                        "08R".to_string(),
                        "26L".to_string(),
                        AsrMapRunwayType::Centerline
                    ),
                    (
                        "EDMA".to_string(),
                        "07".to_string(),
                        "25".to_string(),
                        AsrMapRunwayType::Centerline
                    ),
                    (
                        "EDMO".to_string(),
                        "04".to_string(),
                        "22".to_string(),
                        AsrMapRunwayType::Centerline
                    ),
                ],
                vors: vec![
                    ("LAM".to_string(), AsrMapNavaidType::Name),
                    ("LAM".to_string(), AsrMapNavaidType::Symbol)
                ],
                ndbs: vec![
                    ("BHX".to_string(), AsrMapNavaidType::Name),
                    ("BHX".to_string(), AsrMapNavaidType::Symbol)
                ],
                fixes: vec![
                    ("UPTON".to_string(), AsrMapFixType::Name),
                    ("UPTON".to_string(), AsrMapFixType::Symbol)
                ],
                free_text: vec![("Airspace Bases".to_string(), "FL95".to_string())],
                geo: vec!["Merseyside Coastline".to_string()],
                regions: vec!["Coastline".to_string()],
                artcc_low_boundary: vec!["Channel Islands CTA".to_string()],
                artcc_high_boundary: vec!["EGPX Scottish FIR".to_string()],
                ..Default::default()
            }
        );
        assert_eq!(
            asr.plugin_settings,
            TwoKeyMap(HashMap::from([
                (
                    ("EsCenterLines".to_string(), "Active".to_string()),
                    "2".to_string()
                ),
                (
                    ("TopSky plugin".to_string(), "ShowMapData".to_string()),
                    "APP,EDDM_APP".to_string()
                ),
            ]))
        );
    }
}
