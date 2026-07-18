use std::collections::HashMap;

use tracing::debug;

use crate::ese::{Constraint, Ese};

pub(super) fn extract_constraints(
    ese: &Ese,
) -> (HashMap<String, Constraint>, HashMap<String, Constraint>) {
    let (departure, destination): (Vec<Constraint>, Vec<Constraint>) = ese
        .constraints
        .iter()
        .filter(|&constraint| {
            let to_drop = constraint.climb_level.is_none() && constraint.descent_level.is_none();
            if to_drop {
                debug!("Dropping constraint, no level specified: {constraint:?}");
            }

            !to_drop
        })
        .cloned()
        .partition(|constraint| constraint.climb_level.is_some());

    let keyed = |v: Vec<Constraint>| v.into_iter().map(|c| (c.key(), c)).collect();
    (keyed(departure), keyed(destination))
}
