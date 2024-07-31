use std::collections::HashMap;
use std::io;
use std::{env::args_os, fs};

use geojson::feature::Id;
use geojson::{Feature, FeatureCollection};
use serde::Serialize;
use serde_json::Map;
use vatsim_parser::adaptation::maps::active::RunwayIdentifier;
use vatsim_parser::adaptation::sectors::Sector;
use vatsim_parser::ese::Ese;

#[derive(Serialize)]
struct OpenDataSector {
    description: String,
    volumes: Vec<String>,
    position_priority: Vec<Vec<String>>,
    runway_filter: Vec<Vec<RunwayIdentifier>>,
}

fn main() {
    tracing_subscriber::fmt().with_writer(io::stderr).init();
    let ese_path = args_os().nth(1).expect("missing argument: path to .ese");
    let geojson_path = args_os()
        .nth(2)
        .expect("missing argument: path to .geojson output");
    let sectors_path = args_os()
        .nth(3)
        .expect("missing argument: path to sectors.json output");
    let filter = args_os()
        .nth(4)
        .expect("missing argument: string filter prefix for volumes")
        .into_string()
        .unwrap();
    let ese = Ese::parse(&fs::read(ese_path).unwrap()).expect("unsuccessful parse");
    let (volumes, sectors) = Sector::from_ese(&ese);
    let open_data_sectors = sectors
        .into_iter()
        .filter(|(_id, s)| s.volumes.iter().any(|v| v.starts_with(&filter)))
        .map(|(id, s)| {
            (
                id.clone(),
                OpenDataSector {
                    description: "".to_string(),
                    volumes: s.volumes,
                    position_priority: s.position_priority.into_iter().map(|p| vec![p]).collect(),
                    runway_filter: s.runway_filter,
                },
            )
        })
        .collect::<HashMap<_, _>>();
    let feature_collection = FeatureCollection::from_iter(
        volumes
            .into_iter()
            .filter(|(id, _v)| id.starts_with(&filter))
            .map(|(id, v)| Feature {
                id: Some(Id::String(id)),
                geometry: Some((&v.lateral_border).into()),
                properties: Some(Map::from_iter(vec![
                    ("lower_level".to_string(), v.lower_level.into()),
                    ("upper_level".to_string(), v.upper_level.into()),
                ])),
                ..Default::default()
            }),
    );

    fs::write(geojson_path, feature_collection.to_string()).expect("could not write .geojson");
    fs::write(
        sectors_path,
        serde_json::to_string_pretty(&open_data_sectors).unwrap(),
    )
    .expect("could not write sectors.json");
}
