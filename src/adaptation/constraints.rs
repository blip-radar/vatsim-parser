use tracing::debug;

use crate::ese::{Constraint, Ese};

pub(super) fn extract_constraints(ese: &Ese) -> (Vec<Constraint>, Vec<Constraint>) {
    ese.constraints
        .iter()
        .filter(|&constraint| {
            let to_drop = constraint.climb_level.is_none() && constraint.descent_level.is_none();
            if to_drop {
                debug!("Dropping constraint, no level specified: {constraint:?}");
            }

            !to_drop
        })
        .cloned()
        .partition(|constraint| constraint.climb_level.is_some())
}
