//! Config for the tester's iteration counts, periods, and so on.

use crate::run::halt;
use serde::{Deserialize, Serialize};
use std::num::NonZeroUsize;

/// The default number of iterations in total.
const DEFAULT_ITERATIONS: usize = 1_000_000;
/// The default number of iterations after which the
const DEFAULT_PERIOD: usize = 100_000;

/// The strategy used to handle iteration-based rotations and exits.
#[non_exhaustive]
#[derive(Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum Strategy {
    /// No halting based on iterations.
    #[serde(rename = "no-halt")]
    NoHalt,
    /// Exit after the given number of iterations.
    #[serde(rename = "exit")]
    Exit { iterations: NonZeroUsize },
    /// Exit after the given number of iterations, and rotate after each
    /// multiple of the given period.
    #[serde(rename = "exit-and-rotate")]
    ExitAndRotate {
        iterations: NonZeroUsize,
        period: NonZeroUsize,
    },
}

/// The default strategy uses the constants [DEFAULT_ITERATIONS] and
/// [DEFAULT_PERIOD].
impl Default for Strategy {
    fn default() -> Self {
        Self::from_ints(DEFAULT_ITERATIONS, DEFAULT_PERIOD)
    }
}

impl Strategy {
    /// Gets any halting rules implied by this iteration strategy.
    pub fn halt_rules(&self) -> impl Iterator<Item = halt::Rule> {
        let i_rule = self
            .iterations()
            .map(|x| halt::Condition::EveryNIterations(x).exit());
        let p_rule = self
            .period()
            .map(|x| halt::Condition::EveryNIterations(x).rotate());

        i_rule.into_iter().chain(p_rule.into_iter())
    }

    /// Gets the number of iterations defined by this strategy.
    pub fn iterations(&self) -> Option<NonZeroUsize> {
        match self {
            Self::Exit { iterations } => Some(*iterations),
            Strategy::ExitAndRotate { iterations, .. } => Some(*iterations),
            _ => None,
        }
    }

    /// Gets the rotation period defined by this strategy.
    pub fn period(&self) -> Option<NonZeroUsize> {
        match self {
            Strategy::ExitAndRotate { period, .. } => Some(*period),
            _ => None,
        }
    }

    /// Parses a strategy from a pair of iteration and period integers.
    pub const fn from_ints(iterations: usize, period: usize) -> Self {
        match (NonZeroUsize::new(iterations), NonZeroUsize::new(period)) {
            (None, _) => Self::NoHalt,
            (Some(iterations), None) => Self::Exit { iterations },
            (Some(iterations), Some(period)) => Self::ExitAndRotate { iterations, period },
        }
    }
}
