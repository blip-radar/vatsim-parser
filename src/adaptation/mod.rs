pub mod colours;
pub mod constraints;
pub mod icao;
pub mod line_styles;
pub mod locations;
pub mod maps;
pub mod sct_items;
pub mod sectors;
pub mod settings;
pub mod symbols;

use std::path::PathBuf;
use std::{collections::HashMap, fmt::Write as _, io, path::Path};

use bevy_reflect::Reflect;
use constraints::extract_constraints;
use fs_err::read;
use geo::Coord;
use geo::Point;
use icao::AircraftMap;
use icao::Airline;
use icao::Airport;
use itertools::Itertools;
use jrsonnet_evaluator::manifest::escape_string_json;
use jrsonnet_evaluator::{FileImportResolver, StateBuilder};
use line_styles::{line_styles_from_topsky, Dash};
use sct_items::SctItems;
use sectors::{Sector, Volume};
use serde::{Deserialize, Serialize};
use symbols::Symbols;
use thiserror::Error;
use tracing::trace;
use tracing::warn;

use crate::airway::parse_airway_txt;
use crate::ese::Constraint;
use crate::prf::PrfError;
use crate::{
    airway::AirwayError,
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

#[derive(Clone, Copy, Debug, Default, Reflect, Serialize, Deserialize, PartialEq, Eq)]
pub enum HorizontalAlignment {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, Default, Reflect, Serialize, Deserialize, PartialEq, Eq)]
pub enum VerticalAlignment {
    Top,
    #[default]
    Center,
    Bottom,
}

#[derive(Clone, Copy, Debug, Default, Reflect, Serialize, Deserialize, PartialEq, Eq)]
pub struct Alignment {
    pub horizontal: HorizontalAlignment,
    pub vertical: VerticalAlignment,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    #[error("Prf: {0}")]
    Prf(#[from] PrfError),
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
    #[error("Failed to serialize/deserialize JSON: {0}")]
    JSON(#[from] serde_json::Error),
    #[error("Failed to serialize/deserialize TOML: {0}")]
    TOML(#[from] toml::de::Error),
    #[error("Failed to format: {0}")]
    Formatting(#[from] std::fmt::Error),
    #[error("Jsonnet: {0}")]
    Jsonnet(String),
    #[error("failed to read file: {0}")]
    FileRead(#[from] io::Error),
}

fn normalise_path(base: &Path, to_normalise: PathBuf) -> Result<PathBuf, AdaptationError> {
    Ok(base.join(to_normalise).canonicalize()?)
}

#[derive(Clone, Debug, Deserialize)]
pub struct AdaptationSetup {
    pub prf: PathBuf,
    pub overlays: Vec<PathBuf>,
}
impl AdaptationSetup {
    pub fn parse(adaptation_toml: &Path) -> Result<Self, AdaptationError> {
        let mut adaptation_setup: AdaptationSetup = toml::from_slice(&read(adaptation_toml)?)?;
        let adaptation_toml_parent = adaptation_toml.parent().expect("Cannot be root/prefix");
        adaptation_setup.prf = normalise_path(adaptation_toml_parent, adaptation_setup.prf)?;
        adaptation_setup.overlays = adaptation_setup
            .overlays
            .into_iter()
            .map(|overlay| normalise_path(adaptation_toml_parent, overlay))
            .try_collect()?;
        trace!("{adaptation_setup:?}");

        Ok(adaptation_setup)
    }
}

pub type AdaptationResult = Result<Adaptation, AdaptationError>;
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Adaptation {
    pub name: String,
    pub locations: Locations,
    // TODO id -> pos? something else might be more useful/efficient (freq, prefix, suffix)?
    pub positions: HashMap<String, Position>,
    pub volumes: HashMap<String, Volume>,
    pub sectors: HashMap<String, Sector>,
    pub departure_constraints: Vec<Constraint>,
    pub destination_constraints: Vec<Constraint>,
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
        let (departure_constraints, destination_constraints) = extract_constraints(&ese);
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
        let settings = Settings::from_euroscope(&symbology, topsky.as_ref(), squawks.as_ref(), prf);
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
            departure_constraints,
            destination_constraints,
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

    /// Apply jsonnet overlays to this adaptation using jsonnet's native merging
    /// Each .jsonnet file will be merged using jsonnet's + operator
    pub fn apply_jsonnet_overlays<P: AsRef<Path>>(
        self,
        jsonnet_paths: impl Iterator<Item = P>,
    ) -> Result<Self, AdaptationError> {
        let mut jsonnet_paths = jsonnet_paths.peekable();
        if jsonnet_paths.peek().is_none() {
            return Ok(self);
        }

        let mut jsonnet = serde_json::to_string(&self)?;

        for path in jsonnet_paths {
            write!(
                jsonnet,
                "\n + (import {})",
                escape_string_json(&path.as_ref().display().to_string())
            )?;
        }
        trace!("jsonnet_file: {jsonnet}");

        // Create a State with file import resolver for overlay files
        let mut state_builder = StateBuilder::default();
        state_builder.import_resolver(FileImportResolver::new(vec![std::env::current_dir()?]));
        let state = state_builder.build();

        let merged_jsonnet_val = state
            .evaluate_snippet("combined.jsonnet", jsonnet)
            .map_err(|e| AdaptationError::Jsonnet(e.to_string()))?;

        let json_string = merged_jsonnet_val
            .to_string()
            .map_err(|e| AdaptationError::Jsonnet(e.to_string()))?;

        Ok(serde_json::from_str(&json_string)?)
    }

    /// Create adaptation from .prf and apply .jsonnet overlays
    pub fn from_prf_with_overlays<P: AsRef<Path>>(
        prf: &Prf,
        jsonnet_paths: impl Iterator<Item = P>,
    ) -> AdaptationResult {
        Self::from_prf(prf)?.apply_jsonnet_overlays(jsonnet_paths)
    }

    /// Create adaptation from adaptation.toml
    pub fn from_adaptation_toml<P: AsRef<Path>>(adaptation_toml: &P) -> AdaptationResult {
        let adaptation_setup = AdaptationSetup::parse(adaptation_toml.as_ref())?;

        Self::from_prf_with_overlays(
            &Prf::parse(&adaptation_setup.prf, &read(&adaptation_setup.prf)?)?,
            adaptation_setup.overlays.iter(),
        )
    }
}

pub trait Quantize {
    // Quantize coordinates to 6 decimal places.
    const FACTOR: f64 = 1e6;

    fn q(x: f64) -> i64 {
        (x * Self::FACTOR).round() as i64
    }

    fn quantize(&self) -> (i64, i64);
}

impl Quantize for Point {
    fn quantize(&self) -> (i64, i64) {
        self.0.quantize()
    }
}

impl Quantize for Coord {
    fn quantize(&self) -> (i64, i64) {
        (Self::q(self.x), Self::q(self.y))
    }
}

#[cfg(test)]
mod tests {
    use test_log::test;

    use crate::{adaptation::Adaptation, prf::Prf};
    use std::{collections::HashMap, fs, path::Path};

    #[test]
    fn test_adaptation_toml() {
        let adaptation_res =
            Adaptation::from_adaptation_toml(&Path::new("fixtures/adaptation.toml"));

        assert!(adaptation_res.is_ok(), "{}", adaptation_res.unwrap_err());
        let adaptation = adaptation_res.unwrap();

        assert_eq!(
            adaptation.settings.ssr.special_use_codes,
            HashMap::from([("7000".to_string(), "V".to_string())])
        );
        assert!(adaptation.settings.track.vector.enabled);
    }

    #[test]
    fn test_jsonnet_overlays() {
        let prf_path = Path::new("fixtures/iCAS2.prf");

        let contents = fs::read(prf_path).unwrap();
        let prf = Prf::parse(prf_path, &contents).unwrap();

        let adaptation_res = Adaptation::from_prf_with_overlays(
            &prf,
            [Path::new("fixtures/overlay.jsonnet")].into_iter(),
        );

        assert!(adaptation_res.is_ok(), "{}", adaptation_res.unwrap_err());
        let adaptation = adaptation_res.unwrap();

        assert_eq!(
            adaptation.settings.ssr.special_use_codes,
            HashMap::from([("7000".to_string(), "V".to_string())])
        );
        assert!(adaptation.settings.track.vector.enabled);
    }
}
