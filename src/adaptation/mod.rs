pub mod colours;
pub mod icao;
pub mod line_styles;
pub mod locations;
pub mod maps;
pub mod sct_items;
pub mod sectors;
pub mod settings;
pub mod symbols;

use std::{collections::HashMap, io};

use bevy_reflect::Reflect;
use geo::Point;
use icao::AircraftMap;
use icao::Airline;
use icao::Airport;
use line_styles::{line_styles_from_topsky, Dash};
use sct_items::SctItems;
use sectors::{Sector, Volume};
use serde::Serialize;
use symbols::Symbols;
use thiserror::Error;
use tracing::warn;

use crate::{
    airway::{parse_airway_txt, AirwayError},
    ese::{self, Ese, EseError},
    icao_aircraft::{parse_aircraft, AircraftError},
    icao_airlines::{parse_airlines, AirlinesError},
    icao_airports::{parse_airports, AirportsError},
    prf::Prf,
    sct::{Sct, SctError},
    symbology::{Symbology, SymbologyError},
    topsky::{Topsky, TopskyError},
};

use self::{colours::Colours, locations::Locations, maps::MapFolders, settings::Settings};

#[derive(Clone, Copy, Debug, Default, Reflect, Serialize, PartialEq, Eq)]
pub enum HorizontalAlignment {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, Default, Reflect, Serialize, PartialEq, Eq)]
pub enum VerticalAlignment {
    Top,
    #[default]
    Center,
    Bottom,
}

#[derive(Clone, Copy, Debug, Default, Reflect, Serialize, PartialEq, Eq)]
pub struct Alignment {
    pub horizontal: HorizontalAlignment,
    pub vertical: VerticalAlignment,
}

#[derive(Clone, Debug, Serialize)]
pub struct Position {
    pub id: String,
    pub name: String,
    pub frequency: String,
    pub prefix: String,
    pub suffix: String,
    pub visibility_points: Vec<Point>,
}
impl Position {
    fn from_ese_positions(positions: HashMap<String, ese::Position>) -> HashMap<String, Position> {
        positions
            .into_iter()
            .map(|(id, pos)| {
                (
                    id,
                    Position {
                        id: pos.identifier,
                        name: pos.name,
                        frequency: pos.frequency,
                        prefix: pos.prefix,
                        suffix: pos.suffix,
                        visibility_points: pos
                            .visibility_points
                            .into_iter()
                            .map(Point::from)
                            .collect(),
                    },
                )
            })
            .collect()
    }
}

#[derive(Error, Debug)]
pub enum AdaptationError {
    #[error("SCT: {0}")]
    Sct(#[from] SctError),
    #[error("ESE: {0}")]
    Ese(#[from] EseError),
    #[error("Symbology: {0}")]
    Symbology(#[from] SymbologyError),
    #[error("Topsky: {0}")]
    Topsky(#[from] TopskyError),
    #[error("airway.txt: {0}")]
    Airways(#[from] AirwayError),
    #[error("ICAO_Aircraft.txt: {0}")]
    Aircraft(#[from] AircraftError),
    #[error("ICAO_Airlines.txt: {0}")]
    Airlines(#[from] AirlinesError),
    #[error("ICAO_Airports.txt: {0}")]
    Airports(#[from] AirportsError),
    #[error("failed to read file: {0}")]
    FileRead(#[from] io::Error),
}

pub type AdaptationResult = Result<Adaptation, AdaptationError>;
#[derive(Clone, Debug, Default, Serialize)]
pub struct Adaptation {
    pub name: String,
    pub locations: Locations,
    // TODO id -> pos? something else might be more useful/efficient (freq, prefix, suffix)?
    pub positions: HashMap<String, Position>,
    pub volumes: HashMap<String, Volume>,
    pub sectors: HashMap<String, Sector>,
    pub maps: MapFolders,
    // TODO
    // pub areas,
    // TODO convert to svg?
    pub symbols: Symbols,
    pub colours: Colours,
    pub line_styles: HashMap<String, Option<Vec<Dash>>>,
    pub settings: Settings,
    // TODO
    // approaches: Vec<String>,
    // missed_approaches: Vec<String>,
    // ground stuff
    // - stands
    // - taxiways
    // - maps
    // - runways
    // surveillance information (radar/mlat/...)
    // mva? map only?
    // msaw
    // stca_blanking
    // cpdlc
    // external/extra_plugin_settings?
    pub aircraft: AircraftMap,
    pub airlines: HashMap<String, Airline>,
    pub airports: HashMap<String, Airport>,
    /// .sct items used for drawing maps and otherwise not usable
    pub sct_items: SctItems,
}

impl Adaptation {
    pub fn from_prf(prf: &Prf) -> AdaptationResult {
        // TODO parallelise/asyncify where able
        let sct = Sct::parse(&fs_err::read(prf.sct_path())?)?;
        let ese = Ese::parse(&fs_err::read(prf.ese_path())?)?;
        let airways = parse_airway_txt(&fs_err::read(prf.airways_path())?)?;
        let name = sct.info.name.clone();
        let (volumes, sectors) = Sector::from_ese(&ese);
        let positions = Position::from_ese_positions(ese.positions.clone());
        let symbology = Symbology::parse(&fs_err::read(prf.symbology_path())?)?;
        let squawks = prf
            .squawks_path()
            .and_then(|path| fs_err::read(path).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok());
        let topsky = prf.topsky_path().and_then(|path| {
            Topsky::parse(&path).map_or_else(
                |e| {
                    warn!("Topsky: {e}");
                    None
                },
                Some,
            )
        });
        let settings = Settings::from_euroscope(&symbology, topsky.as_ref(), squawks.as_ref());
        let colours = Colours::from_euroscope(&symbology, &sct, &topsky, &settings);
        let locations = Locations::from_euroscope(sct.clone(), ese, airways);
        let sct_items = SctItems::from_sct(sct, &locations, &colours, &settings);
        let aircraft = parse_aircraft(&fs_err::read(prf.aircraft_path())?)?;
        let airlines = parse_airlines(&fs_err::read(prf.airlines_path())?)?;
        let airports = parse_airports(&fs_err::read(prf.airports_path())?)?;
        Ok(Adaptation {
            name,
            positions,
            volumes,
            sectors,
            maps: topsky
                .as_ref()
                .map(|topsky| maps::from_topsky(topsky, &settings, &colours, &locations))
                .unwrap_or_default(),
            locations,
            symbols: Symbols::from_euroscope(&symbology, &topsky),
            line_styles: line_styles_from_topsky(&topsky),
            colours,
            settings,
            aircraft,
            airlines,
            airports,
            sct_items,
        })
    }
}
