//! Models for states.

use super::outcome;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt::Display};

/// An observed state.
///
/// `State` is an ordered map; the order should be consistent between each state
/// observed in a test.
pub type State = BTreeMap<String, Value>;

/// A value in a state.
///
/// Values are marked non-exhaustive as phenolphthalein may add new value types
/// in future.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "type", content = "value")]
#[non_exhaustive]
pub enum Value {
    /// A 32-bit signed integer.
    I32(i32),
}

/// We display values, by default, without any type annotation.
impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::I32(v) => v,
            }
        )
    }
}

/// A record of information about an observed stae.
///
/// An observation aggregates the various times a tester has seen a particular
/// state.
#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct Info {
    /// The number of the cycle where this observation first occurred.
    pub iteration: usize,
    /// The number of times this state has occurred.
    pub occurs: usize,
    /// The result of asking the test to check this state.
    pub outcome: outcome::Outcome,
}

impl Info {
    /// Creates a new [Info] with the given outcome and iteration, and with
    /// an occurs count of 1.
    #[must_use]
    pub fn new(outcome: outcome::Outcome, iteration: usize) -> Self {
        Self {
            occurs: 1,
            outcome,
            iteration,
        }
    }

    /// Computes the [Info] resulting from increasing this [Info]'s
    /// occurs count by 1.
    #[must_use]
    pub fn inc(&self) -> Info {
        Info {
            occurs: self.occurs.saturating_add(1),
            ..*self
        }
    }
}
