use bevy_reflect::Reflect;
use pest::iterators::Pair;
use serde::Serialize;

use crate::topsky::Rule;

use super::Runway;

#[derive(Clone, Debug, PartialEq, Eq, Reflect, Serialize)]
pub struct ActiveIds {
    pub own: Option<Vec<String>>,
    pub own_excludes: Option<Vec<String>>,
    pub online: Option<Vec<String>>,
    pub online_excludes: Option<Vec<String>>,
}

impl ActiveIds {
    fn parse(pair: Pair<Rule>) -> Self {
        let mut active = pair.into_inner();
        let own = Self::parse_value(active.next().unwrap());
        let own_excludes = Self::parse_value(active.next().unwrap());
        let online = Self::parse_value(active.next().unwrap());
        let online_excludes = Self::parse_value(active.next().unwrap());
        Self {
            own,
            own_excludes,
            online,
            online_excludes,
        }
    }

    fn parse_value(pair: Pair<Rule>) -> Option<Vec<String>> {
        match pair.as_rule() {
            Rule::wildcard => None,
            Rule::names => Some(
                pair.into_inner()
                    .map(|pair| pair.as_str().to_string())
                    .collect(),
            ),
            rule => {
                eprintln!("{rule:?}");
                unreachable!()
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Reflect, Serialize)]
pub struct ActiveRunways {
    pub arrival: Option<Vec<Runway>>,
    pub arrival_excludes: Option<Vec<Runway>>,
    pub departure: Option<Vec<Runway>>,
    pub departure_excludes: Option<Vec<Runway>>,
}

impl ActiveRunways {
    fn parse(pair: Pair<Rule>) -> Self {
        let mut active = pair.into_inner();
        let arrival = Self::parse_value(active.next().unwrap());
        let departure = Self::parse_value(active.next().unwrap());
        Self {
            arrival,
            arrival_excludes: None,
            departure,
            departure_excludes: None,
        }
    }

    fn parse_with_excludes(pair: Pair<Rule>) -> Self {
        let mut active = pair.into_inner();
        let arrival = Self::parse_value(active.next().unwrap());
        let arrival_excludes = Self::parse_value(active.next().unwrap());
        let departure = Self::parse_value(active.next().unwrap());
        let departure_excludes = Self::parse_value(active.next().unwrap());
        Self {
            arrival,
            arrival_excludes,
            departure,
            departure_excludes,
        }
    }

    fn parse_value(pair: Pair<Rule>) -> Option<Vec<Runway>> {
        match pair.as_rule() {
            Rule::wildcard => None,
            Rule::runways => Some(pair.into_inner().map(Runway::parse).collect()),
            rule => {
                eprintln!("{rule:?}");
                unreachable!()
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Reflect, Serialize)]
pub struct ActiveAreas {
    pub areas: Vec<String>,
    pub area_excludes: Option<Vec<String>>,
}
impl ActiveAreas {
    fn parse(pair: Pair<Rule>) -> Self {
        let mut active = pair.into_inner();
        let areas = active
            .next()
            .unwrap()
            .into_inner()
            .map(|pair| pair.as_str().to_string())
            .collect();
        Self {
            areas,
            area_excludes: None,
        }
    }

    fn parse_with_excludes(pair: Pair<Rule>) -> Self {
        let mut active = pair.into_inner();
        let areas = active
            .next()
            .unwrap()
            .into_inner()
            .map(|pair| pair.as_str().to_string())
            .collect();
        let area_excludes = Some(
            active
                .next()
                .unwrap()
                .into_inner()
                .map(|pair| pair.as_str().to_string())
                .collect(),
        );
        Self {
            areas,
            area_excludes,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Reflect, Serialize)]
pub enum ActiveMapOperator {
    Same,
    Opposite,
}

#[derive(Clone, Debug, PartialEq, Eq, Reflect, Serialize)]
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

impl Active {
    pub(super) fn parse(pair: Pair<Rule>) -> Self {
        let active = pair.into_inner().next().unwrap();
        match active.as_rule() {
            Rule::active_always => Self::True,
            Rule::active_id => Self::Id(ActiveIds::parse(active)),
            Rule::active_callsign => Self::Callsign(ActiveIds::parse(active)),
            Rule::active_aup => Self::Aup(
                active
                    .into_inner()
                    .map(|pair| pair.as_str().to_string())
                    .collect(),
            ),
            Rule::active_notam => {
                let mut active_notam = active.into_inner();
                Self::Notam(
                    active_notam.next().unwrap().as_str().to_string(),
                    active_notam.map(|pair| pair.as_str().to_string()).collect(),
                )
            }
            Rule::active_area => Self::Area(ActiveAreas::parse(active)),
            Rule::active_area_with_excludes => Self::Area(ActiveAreas::parse_with_excludes(active)),
            Rule::active_sched => Self::Schedule, // TODO
            Rule::active_rwy => Self::Runway(ActiveRunways::parse(active)),
            Rule::active_rwy_with_excludes => {
                Self::Runway(ActiveRunways::parse_with_excludes(active))
            }
            Rule::active_map => {
                let mut active_map = active.into_inner();
                let op = match active_map.next().unwrap().as_str() {
                    "!" => ActiveMapOperator::Opposite,
                    "=" => ActiveMapOperator::Same,
                    op => {
                        eprintln!("Unknown active_map operator: {op}");
                        unreachable!()
                    }
                };
                Self::Map(
                    op,
                    active_map.next().unwrap().as_str().to_string(),
                    active_map.next().unwrap().as_str().to_string(),
                )
            }
            rule => {
                eprintln!("{rule:?}");
                unreachable!()
            }
        }
    }
}
