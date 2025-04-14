use tracing::debug;

use crate::ese::{Agreement, Ese};

pub(super) fn extract_agreements(ese: &Ese) -> (Vec<Agreement>, Vec<Agreement>) {
    ese.sectors
        .iter()
        .flat_map(|(_id, sector)| {
            sector
                .copns
                .iter()
                .chain(sector.fir_copns.iter())
                .chain(sector.copxs.iter())
                .chain(sector.fir_copxs.iter())
                .cloned()
        })
        .filter(|agreement| {
            let to_drop = agreement.climb_level.is_none() && agreement.descent_level.is_none();
            if to_drop {
                debug!("Dropping agreement, no level specified: {agreement:?}");
            }

            !to_drop
        })
        .partition(|agreement| agreement.climb_level.is_some())
}
