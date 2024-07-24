use bevy_reflect::Reflect;
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Hash, Reflect, Serialize)]
pub struct RunwayIdentifier {
    pub icao: String,
    pub designator: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Reflect, Serialize)]
pub struct ActiveIds {
    pub own: Option<Vec<String>>,
    pub own_excludes: Option<Vec<String>>,
    pub online: Option<Vec<String>>,
    pub online_excludes: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Reflect, Serialize)]
pub struct ActiveRunways {
    pub arrival: Option<Vec<RunwayIdentifier>>,
    pub arrival_excludes: Option<Vec<RunwayIdentifier>>,
    pub departure: Option<Vec<RunwayIdentifier>>,
    pub departure_excludes: Option<Vec<RunwayIdentifier>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Reflect, Serialize)]
pub struct ActiveAreas {
    pub areas: Vec<String>,
    pub area_excludes: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Reflect, Serialize)]
pub enum ActiveMapOperator {
    Same,
    Opposite,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum Active {
    True,
    Schedule,
    Aup(Vec<String>),
    Notam(String, Vec<String>),
    Area(ActiveAreas),
    Id(ActiveIds),
    Callsign(ActiveIds),
    Runway(ActiveRunways),
    /// Same or Opposite as Map in Folder, Name
    Map(ActiveMapOperator, String, String),
}
