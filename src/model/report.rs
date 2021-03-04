//! The [Report] type.

use super::{outcome, state};
use serde::{Deserialize, Serialize};
use std::vec::Vec;

/// A final report of observations coming from a test run.
#[derive(Serialize, Deserialize)]
pub struct Report {
    /// The overall outcome of checks performed on states on this run.
    ///
    /// This is an option, to disambiguate between an unknown outcome and an
    /// outcome where there were no states in the report.
    pub outcome: Option<outcome::Outcome>,

    /// Reports for each state observed.
    ///
    /// This is a vector to ease serialisation and deserialisation, rather than
    /// for any deep purpose.
    pub states: Vec<State>,
}

impl Report {
    /// Adds a state to the report, updating aggregates accordingly.
    pub fn insert(&mut self, state: State) {
        self.outcome = self.outcome.max(Some(state.info.outcome));
        self.states.push(state);
    }
}

/// A report for a single state, containing both the valuation and metadata.
#[derive(Serialize, Deserialize)]
pub struct State {
    /// The valuation for the state.
    pub state: state::State,

    /// The metadata for the stage.
    #[serde(flatten)]
    pub info: state::Info,
}
