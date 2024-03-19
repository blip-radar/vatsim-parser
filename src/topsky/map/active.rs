use bevy_reflect::Reflect;
use pest::iterators::Pair;
use serde::Serialize;

use crate::topsky::Rule;

use super::Runway;

#[derive(Clone, Debug, PartialEq, Eq, Reflect, Serialize)]
pub enum ActiveIdType {
    Wildcard,
    Defined(Vec<String>),
}
impl ActiveIdType {
    fn parse(pair: Pair<Rule>) -> Self {
        match pair.as_rule() {
            Rule::wildcard => Self::Wildcard,
            Rule::names => Self::Defined(
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
pub struct ActiveIds {
    pub own: ActiveIdType,
    pub own_excludes: ActiveIdType,
    pub online: ActiveIdType,
    pub online_excludes: ActiveIdType,
}

impl ActiveIds {
    fn parse(pair: Pair<Rule>) -> Self {
        let mut active = pair.into_inner();
        let own = ActiveIdType::parse(active.next().unwrap());
        let own_excludes = ActiveIdType::parse(active.next().unwrap());
        let online = ActiveIdType::parse(active.next().unwrap());
        let online_excludes = ActiveIdType::parse(active.next().unwrap());
        Self {
            own,
            own_excludes,
            online,
            online_excludes,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Reflect, Serialize)]
pub enum ActiveRunwaysType {
    Wildcard,
    Active(Vec<Runway>),
}
impl ActiveRunwaysType {
    fn parse(pair: Pair<Rule>) -> Self {
        match pair.as_rule() {
            Rule::wildcard => ActiveRunwaysType::Wildcard,
            Rule::runways => {
                ActiveRunwaysType::Active(pair.into_inner().map(Runway::parse).collect())
            }
            rule => {
                eprintln!("{rule:?}");
                unreachable!()
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Reflect, Serialize)]
pub struct ActiveRunways {
    pub arrival: ActiveRunwaysType,
    pub arrival_excludes: ActiveRunwaysType,
    pub departure: ActiveRunwaysType,
    pub departure_excludes: ActiveRunwaysType,
}

impl ActiveRunways {
    fn parse(pair: Pair<Rule>) -> Self {
        let mut active = pair.into_inner();
        let arrival = ActiveRunwaysType::parse(active.next().unwrap());
        let departure = ActiveRunwaysType::parse(active.next().unwrap());
        Self {
            arrival,
            arrival_excludes: ActiveRunwaysType::Wildcard,
            departure,
            departure_excludes: ActiveRunwaysType::Wildcard,
        }
    }

    fn parse_with_excludes(pair: Pair<Rule>) -> Self {
        let mut active = pair.into_inner();
        let arrival = ActiveRunwaysType::parse(active.next().unwrap());
        let arrival_excludes = ActiveRunwaysType::parse(active.next().unwrap());
        let departure = ActiveRunwaysType::parse(active.next().unwrap());
        let departure_excludes = ActiveRunwaysType::parse(active.next().unwrap());
        Self {
            arrival,
            arrival_excludes,
            departure,
            departure_excludes,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Reflect, Serialize)]
pub enum Active {
    True,
    Schedule,
    Id(ActiveIds),
    Runway(ActiveRunways),
}

impl Active {
    pub(super) fn parse(pair: Pair<Rule>) -> Self {
        let active = pair.into_inner().next().unwrap();
        match active.as_rule() {
            Rule::active_always => Self::True,
            Rule::active_id => Self::Id(ActiveIds::parse(active)), // TODO
            Rule::active_sched => Self::Schedule,                  // TODO
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

