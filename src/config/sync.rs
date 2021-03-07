//! Config for the tester's synchronisation methods.

use super::err;
use crate::run::sync;
use serde::{Deserialize, Serialize};

/// String representations of each strategy, used in the clap interface.
pub mod string {
    /// Name of the `Spinner` synchronisation strategy.
    pub const SPINNER: &str = "spinner";
    /// Name of the `SpinBarrier` synchronisation strategy.
    pub const SPIN_BARRIER: &str = "spin-barrier";
    /// Name of the `Barrier` synchronisation strategy.
    pub const BARRIER: &str = "barrier";
    /// Names of all synchronisation strategies.
    pub const ALL: &[&str] = &[SPINNER, SPIN_BARRIER, BARRIER];
}

/// Enumeration of synchronisation strategy exported by the phenolphthalein
/// toplevel.
#[derive(Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Strategy {
    /// Represents the spinner synchronisation strategy.
    Spinner,
    /// Represents the spin-barrier synchronisation strategy.
    SpinBarrier,
    /// Represents the barrier synchronisation strategy.
    Barrier,
}

/// The default synchronisation strategy is the spinner.
impl Default for Strategy {
    fn default() -> Self {
        Self::Spinner
    }
}

/// Tries to parse a [Strategy] from a string.
impl std::str::FromStr for Strategy {
    type Err = err::Error;

    fn from_str(s: &str) -> err::Result<Self> {
        match s {
            string::SPINNER => Ok(Self::Spinner),
            string::SPIN_BARRIER => Ok(Self::SpinBarrier),
            string::BARRIER => Ok(Self::Barrier),
            s => Err(err::Error::BadSyncStrategy(s.to_owned())),
        }
    }
}

/// Formats a [Strategy] by applying the inverse of [FromStr].
impl std::fmt::Display for Strategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Spinner => string::SPINNER,
                Self::SpinBarrier => string::SPIN_BARRIER,
                Self::Barrier => string::BARRIER,
            }
        )
    }
}

impl Strategy {
    pub fn all() -> impl Iterator<Item = Self> {
        vec![Self::Spinner, Self::SpinBarrier, Self::Barrier].into_iter()
    }

    /// Gets the correct factory method for the synchronisation primitive
    /// requested in this argument set.
    pub fn to_factory(&self) -> sync::Factory {
        match self {
            Self::Barrier => sync::make_barrier,
            Self::SpinBarrier => sync::make_spin_barrier,
            Self::Spinner => sync::make_spinner,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Tests that the ALL constant reflects the result of getting strings for
    /// each strategy in turn.
    #[test]
    fn test_all_strings_in_sync() {
        let got_set: HashSet<String> = string::ALL.into_iter().map(|x| x.to_string()).collect();
        let want_set = Strategy::all().map(|x| Strategy::to_string(&x)).collect();
        assert_eq!(got_set, want_set)
    }
}
