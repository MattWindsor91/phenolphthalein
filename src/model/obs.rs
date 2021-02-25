use super::outcome;
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
    /// The number of the cycle where this observation first occurred.
    pub iteration: usize,
    /// The number of times this state has occurred.
    pub occurs: usize,
    /// The result of asking the test to check this state.
    pub outcome: outcome::Outcome,
}

impl Obs {
    /// Computes the Obs resulting from increasing this Obs's
    /// occurs count by 1.
    pub fn inc(&self) -> Obs {
        Obs {
            occurs: self.occurs.saturating_add(1),
            ..*self
        }
    }
}

/// A final report of observations coming from a test run.
pub struct Report {
    /// The overall outcome of checks performed on states on this run.
    pub outcome: Option<outcome::Outcome>,

    /// The full state observation set.
    pub obs: Set,
}
