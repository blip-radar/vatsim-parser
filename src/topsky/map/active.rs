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
pub enum Active {
    True,
    Schedule,
    Area(String),
    Id(ActiveIds),
    Runway(ActiveRunways),
}

impl Active {
    pub(super) fn parse(pair: Pair<Rule>) -> Self {
        let active = pair.into_inner().next().unwrap();
        match active.as_rule() {
            Rule::active_always => Self::True,
            Rule::active_id => Self::Id(ActiveIds::parse(active)),
            Rule::active_area => {
                Self::Area(active.into_inner().next().unwrap().as_str().to_string())
            }
            Rule::active_sched => Self::Schedule, // TODO
            Rule::active_rwy => Self::Runway(ActiveRunways::parse(active)),
            Rule::active_rwy_with_excludes => {
                Self::Runway(ActiveRunways::parse_with_excludes(active))
            }
            rule => {
                eprintln!("{rule:?}");
                unreachable!()
            }
        }
    }
}
