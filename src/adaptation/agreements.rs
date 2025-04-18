use tracing::debug;

use crate::ese::{Agreement, Ese};

pub(super) fn extract_agreements(ese: &Ese) -> (Vec<Agreement>, Vec<Agreement>) {
    ese.agreements
        .iter()
        .filter(|&agreement| {
            let to_drop = agreement.climb_level.is_none() && agreement.descent_level.is_none();
            if to_drop {
                debug!("Dropping agreement, no level specified: {agreement:?}");
            }

            !to_drop
        })
        .cloned()
        .partition(|agreement| agreement.climb_level.is_some())
}
