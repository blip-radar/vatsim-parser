use super::Rule;
use pest::iterators::Pair;

use crate::adaptation::maps::active::{
    Active, ActiveAreas, ActiveIds, ActiveMapOperator, ActiveRunways, Runway,
};

impl Runway {
    fn parse(pair: Pair<Rule>) -> Self {
        let mut rwy = pair.into_inner();
        let icao = rwy.next().unwrap().as_str().to_string();
        let designator = rwy.next().unwrap().as_str().to_string();
        Self { icao, designator }
    }
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
            rule => unreachable!("{rule:?}"),
        }
    }
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
            rule => unreachable!("{rule:?}"),
        }
    }
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
                    op => unreachable!("Unknown active_map operator: {op}"),
                };
                Self::Map(
                    op,
                    active_map.next().unwrap().as_str().to_string(),
                    active_map.next().unwrap().as_str().to_string(),
                )
            }
            rule => unreachable!("{rule:?}"),
        }
    }
}
