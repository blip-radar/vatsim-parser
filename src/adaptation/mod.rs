pub mod colours;
pub mod locations;
pub mod maps;
pub mod sectors;
pub mod settings;

use std::{collections::HashMap, fs, io};

use bevy_reflect::Reflect;
use sectors::{Sector, Volume};
use serde::Serialize;
use thiserror::Error;

use crate::{
    airway::{parse_airway_txt, AirwayError},
    ese::{self, Ese, EseError},
    prf::Prf,
    sct::{Sct, SctError},
    symbology::{Symbology, SymbologyError},
    topsky::{symbol::SymbolDef, Topsky, TopskyError},
};

use self::{colours::Colours, locations::Locations, maps::MapFolders, settings::Settings};

#[derive(Clone, Debug, Default, Reflect, Serialize, PartialEq, Eq)]
pub enum HorizontalAlignment {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Clone, Debug, Default, Reflect, Serialize, PartialEq, Eq)]
pub enum VerticalAlignment {
    Top,
    #[default]
    Center,
    Bottom,
}

#[derive(Clone, Debug, Default, Reflect, Serialize, PartialEq, Eq)]
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
    // TODO vis points?
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
    #[error("failed to read file: {0}")]
    FileRead(#[from] io::Error),
}

pub type AdaptationResult = Result<Adaptation, AdaptationError>;
#[derive(Clone, Debug, Default, Serialize)]
pub struct Adaptation {
    pub locations: Locations,
    // TODO id -> pos? something else might be more useful/efficient (freq, prefix, suffix)?
    pub positions: HashMap<String, Position>,
    pub volumes: HashMap<String, Volume>,
    pub sectors: HashMap<String, Sector>,
    pub maps: MapFolders,
    // TODO
    // pub areas,
    // TODO convert to svg?
    pub symbols: HashMap<String, SymbolDef>,
    pub colours: Colours,
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
}

impl Adaptation {
    pub fn from_prf(prf: Prf) -> AdaptationResult {
        // TODO parallelise/asyncify where able
        let sct = Sct::parse(&fs::read(prf.sct_path())?)?;
        let ese = Ese::parse(&fs::read(prf.ese_path())?)?;
        let airways = parse_airway_txt(&fs::read(prf.airways_path())?)?;
        let (volumes, sectors) = Sector::from_ese(&ese);
        let positions = Position::from_ese_positions(ese.positions.clone());
        let locations = Locations::from_euroscope(sct, ese, airways);
        let symbology = Symbology::parse(&fs::read(prf.symbology_path())?)?;
        let squawks = prf
            .squawks_path()
            .and_then(|path| fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok());
        let topsky = prf.topsky_path().and_then(|path| {
            Topsky::parse(path).map(Some).unwrap_or_else(|e| {
                eprintln!("Topsky: {e}");
                None
            })
        });
        let settings = Settings::from_euroscope(&topsky, &squawks);
        let colours = Colours::from_euroscope(&symbology, &topsky, &settings);
        Ok(Adaptation {
            positions,
            volumes,
            sectors,
            maps: topsky
                .as_ref()
                .map(|topsky| maps::from_topsky(topsky, &settings, &colours, &locations))
                .unwrap_or_default(),
            locations,
            symbols: topsky.map(|t| t.symbols).unwrap_or_default(),
            colours,
            settings,
        })
    }
}
