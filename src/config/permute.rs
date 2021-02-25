//! Config for thread permutation.

use super::err;
use crate::run::{self, permute};

/// String representations of checking strategies
pub mod string {
    /// String representation of the random permute strategy.
    pub const RANDOM: &str = "random";
    /// String representation of the static permute strategy.
    pub const STATIC: &str = "static";
    /// String representations of all checking strategies.
    ///
    /// This is unrolled into a single slice to make use in clap easier than
    /// programmatically generating it would allow.
    pub const ALL: &[&str] = &[RANDOM, STATIC];
}

/// Enumeration of thread permutation methods.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Strategy {
    /// Randomly permute thread-automaton assignments on each rotation.
    Random,
    /// Never permute.
    Static,
}

/// The default permutation method is random permutation.
impl Default for Strategy {
    fn default() -> Self {
        Self::Random
    }
}

/// Tries to parse a [Strategy] from a string.
impl std::str::FromStr for Strategy {
    type Err = err::Error;

    fn from_str(s: &str) -> err::Result<Self> {
        match s {
            string::RANDOM => Ok(Self::Random),
            string::STATIC => Ok(Self::Static),
            s => Err(err::Error::BadPermuteStrategy(s.to_owned())),
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
                Self::Random => string::RANDOM,
                Self::Static => string::STATIC,
            }
        )
    }
}

impl Strategy {
    /// Gets an iterator of all available strategies.
    ///
    /// # Examples
    ///
    /// ```
    /// use phenolphthalein::config::permute::Strategy;
    /// let set: std::collections::HashSet<Strategy> = Strategy::all().collect();
    ///
    /// assert!(set.contains(&Strategy::Random));
    /// assert!(set.contains(&Strategy::Static));
    /// ```
    pub fn all() -> impl Iterator<Item = Self> {
        vec![Self::Random, Self::Static].into_iter()
    }

    pub fn to_permuter<T: run::permute::HasTid>(&self) -> Box<dyn run::Permuter<T>> {
        match self {
            Self::Random => Box::new(rand::thread_rng()),
            Self::Static => Box::new(permute::Nop {}),
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
