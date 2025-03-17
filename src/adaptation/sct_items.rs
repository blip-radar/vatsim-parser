use multimap::MultiMap;
use serde::Serialize;

use crate::sct::{Airway, Artcc, Geo, Region, Sct, Sid, Star};

#[derive(Clone, Debug, Default, Serialize)]
pub struct SctItems {
    pub sids: MultiMap<String, Sid>,
    pub stars: MultiMap<String, Star>,
    pub high_airways: MultiMap<String, Airway>,
    pub low_airways: MultiMap<String, Airway>,
    pub artccs_high: MultiMap<String, Artcc>,
    pub artccs: MultiMap<String, Artcc>,
    pub artccs_low: MultiMap<String, Artcc>,
    pub geo: MultiMap<String, Geo>,
    pub regions: MultiMap<String, Region>,
}

impl SctItems {
    pub fn from_sct(sct: &Sct) -> Self {
        Self {
            sids: sct
                .sids
                .iter()
                .map(|sid| (sid.name.clone(), sid.clone()))
                .collect(),
            stars: sct
                .stars
                .iter()
                .map(|star| (star.name.clone(), star.clone()))
                .collect(),
            high_airways: sct
                .high_airways
                .iter()
                .map(|awy| (awy.designator.clone(), awy.clone()))
                .collect(),
            low_airways: sct
                .low_airways
                .iter()
                .map(|awy| (awy.designator.clone(), awy.clone()))
                .collect(),
            artccs_high: sct
                .artccs_high
                .iter()
                .map(|artcc| (artcc.name.clone(), artcc.clone()))
                .collect(),
            artccs: sct
                .artccs
                .iter()
                .map(|artcc| (artcc.name.clone(), artcc.clone()))
                .collect(),
            artccs_low: sct
                .artccs_low
                .iter()
                .map(|artcc| (artcc.name.clone(), artcc.clone()))
                .collect(),
            geo: sct
                .geo
                .iter()
                .map(|geo| (geo.name.clone(), geo.clone()))
                .collect(),
            regions: sct
                .regions
                .iter()
                .map(|region| (region.name.clone(), region.clone()))
                .collect(),
        }
    }
}
