use super::check;
use std::collections::{BTreeMap, HashMap};

/// An observed state.
///
/// `State` is an ordered map; the order should be consistent between each state
/// in a `Set`.
pub type State = BTreeMap<String, i32>;

/// A set of states and aggregate observations about them.
///
/// `Set` is an unordered map.
pub type Set = HashMap<State, Obs>;

/// Information about an observation.
#[derive(Copy, Clone)]
pub struct Obs {
    /// The number of times this state has occurred.
    pub occurs: usize,
    /// The result of asking the test to check this state.
    pub check_result: check::Outcome,
}

impl Obs {
    /// Computes the Obs resulting from increasing this Obs's
    /// occurs count by 1.
    pub fn inc(&self) -> Obs {
        Obs {
            occurs: self.occurs.saturating_add(1),
            check_result: self.check_result,
        }
    }
}
