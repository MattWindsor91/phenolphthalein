//! Config for the tester's synchronisation methods.

use super::err;
use crate::run::sync;
use serde::{Deserialize, Serialize};

/// String representations of each strategy.
pub mod string {
    /// Name of the `Spinner` synchronisation strategy.
    pub const SPINNER: &str = "spinner";
    /// Name of the `Barrier` synchronisation strategy.
    pub const BARRIER: &str = "barrier";
    /// Names of all synchronisation strategies.
    pub const ALL: &[&str] = &[SPINNER, BARRIER];
}

/// Enumeration of synchronisation strategy exported by the phenolphthalein
/// toplevel.
#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum Strategy {
    /// Represents the spinner synchronisation strategy.
    #[serde(rename = "spinner")]
    Spinner,
    /// Represents the barrier synchronisation strategy.
    #[serde(rename = "barrier")]
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
                Self::Barrier => string::BARRIER,
            }
        )
    }
}

impl Strategy {
    pub fn all() -> impl Iterator<Item = Self> {
        vec![Self::Spinner, Self::Barrier].into_iter()
    }

    /// Gets the correct factory method for the synchronisation primitive
    /// requested in this argument set.
    pub fn to_factory(&self) -> sync::Factory {
        match self {
            Self::Barrier => sync::make_barrier,
            Self::Spinner => sync::make_spinner,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests that the ALL constant reflects the result of getting strings for
    /// each strategy in turn.
    #[test]
    fn test_all_strings_in_sync() {
        let got_set: std::collections::HashSet<String> =
            string::ALL.into_iter().map(|x| x.to_string()).collect();
        let want_set: std::collections::HashSet<String> =
            Strategy::all().map(|x| Strategy::to_string(&x)).collect();
        assert_eq!(got_set, want_set)
    }
}