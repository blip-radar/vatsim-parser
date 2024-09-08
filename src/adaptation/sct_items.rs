use multimap::MultiMap;
use serde::Serialize;

use crate::sct::{Geo, Region, Sct};

#[derive(Clone, Debug, Default, Serialize)]
pub struct SctItems {
    pub geo: MultiMap<String, Geo>,
    pub regions: MultiMap<String, Region>,
}

impl SctItems {
    pub fn from_sct(sct: &Sct) -> Self {
        Self {
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
