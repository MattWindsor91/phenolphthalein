//! Support for configuring how the tester approaches state checks.

use super::err;
use crate::{api::abs, model::outcome, run::halt};
use serde::{de::Visitor, Deserialize, Serialize};

/// String representations of checking strategies.
pub mod string {
    /// String representation of the disable check strategy.
    pub const DISABLE: &str = "disable";
    /// String representation of the report check strategy.
    pub const REPORT: &str = "report";
    /// String representation of the prefix of the exit-on check strategy.
    /// This gets prepended to outcome names to form strategies.
    pub const EXIT_ON_PREFIX: &str = "exit-on-";
    /// String representations of all checking strategies.
    ///
    /// This is unrolled into a single slice to make use in clap easier than
    /// programmatically generating it would allow.
    pub const ALL: &[&str] = &[
        DISABLE,
        REPORT,
        "exit-on-pass",
        "exit-on-fail",
        "exit-on-unknown",
    ];
}

/// Enumeration of test checking strategies.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Strategy {
    /// All checks are off.
    Disable,
    /// Checks are on, but only influence the final report.
    Report,
    /// Checks are on, and the test will halt when it sees the first state with
    /// the given outcome.
    ExitOn(outcome::Outcome),
}

/// The default strategy is reporting only.
impl Default for Strategy {
    fn default() -> Self {
        Self::Report
    }
}

/// Tries to parse a [Strategy] from a string.
///
/// # Examples
///
/// ```
/// use phenolphthalein::{config::check::Strategy, model::Outcome};
/// assert_eq!(str::parse::<Strategy>("disable"), Ok(Strategy::Disable));
/// assert_eq!(str::parse::<Strategy>("Report"), Ok(Strategy::Report));
/// assert_eq!(str::parse::<Strategy>("EXIT-ON-PASS"), Ok(Strategy::ExitOn(Outcome::Pass)));
/// ```
impl std::str::FromStr for Strategy {
    type Err = err::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        if let Some(outcome) = s.strip_prefix(string::EXIT_ON_PREFIX) {
            Ok(Self::ExitOn(
                outcome.parse().map_err(Self::Err::BadCheckOutcome)?,
            ))
        } else {
            match &*s {
                string::DISABLE => Ok(Self::Disable),
                string::REPORT => Ok(Self::Report),
                _ => Err(Self::Err::BadCheckStrategy(s)),
            }
        }
    }
}

/// Formats a [Strategy] by applying the inverse of [FromStr].
impl std::fmt::Display for Strategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disable => write!(f, "{}", string::DISABLE),
            Self::Report => write!(f, "{}", string::REPORT),
            Self::ExitOn(outcome) => write!(f, "{}{}", string::EXIT_ON_PREFIX, outcome),
        }
    }
}

/// Serialize by stringification.
impl Serialize for Strategy {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

/// Deserialize by parsing.
impl<'de> Deserialize<'de> for Strategy {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(StrategyVisitor)
    }
}

struct StrategyVisitor;

impl<'de> Visitor<'de> for StrategyVisitor {
    type Value = Strategy;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "one of: {}", string::ALL.join(", "))
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> std::result::Result<Self::Value, E> {
        Ok(v.parse().map_err(E::custom)?)
    }
}

impl Strategy {
    /// Gets an iterator of all available strategies.
    ///
    /// # Examples
    ///
    /// ```
    /// use phenolphthalein::{config::check::Strategy, model::Outcome};
    /// let set: std::collections::HashSet<Strategy> = Strategy::all().collect();
    ///
    /// assert!(set.contains(&Strategy::Disable));
    /// assert!(set.contains(&Strategy::Report));
    /// assert!(set.contains(&Strategy::ExitOn(Outcome::Pass)));
    /// assert!(set.contains(&Strategy::ExitOn(Outcome::Fail)));
    /// assert!(set.contains(&Strategy::ExitOn(Outcome::Unknown)));
    /// ```
    pub fn all() -> impl Iterator<Item = Self> {
        vec![Self::Disable, Self::Report]
            .into_iter()
            .chain(outcome::Outcome::all().map(Self::ExitOn))
    }

    /// Retrieves any test halt rules implied by this checking strategy.
    pub fn halt_rules(&self) -> impl Iterator<Item = halt::Rule> {
        self.halt_outcome()
            .map(|x| halt::Condition::OnOutcome(x).exit())
            .into_iter()
    }

    /// Retrieves any outcome being halted upon by this checking strategy.
    pub fn halt_outcome(&self) -> Option<outcome::Outcome> {
        match self {
            Self::ExitOn(outcome) => Some(*outcome),
            _ => None,
        }
    }

    /// Gets whether checking is disabled in this strategy.
    ///
    /// # Examples
    ///
    /// ```
    /// use phenolphthalein::{model::Outcome, config::check::Strategy};
    ///
    /// assert!(Strategy::Disable.is_disabled());
    /// assert!(!Strategy::Report.is_disabled());
    /// assert!(!Strategy::ExitOn(Outcome::Pass).is_disabled());
    /// ```
    pub fn is_disabled(&self) -> bool {
        matches!(self, Self::Disable)
    }

    pub fn to_factory<'a, T: abs::Entry<'a>>(&self) -> abs::check::Factory<'a, T, T::Env> {
        if self.is_disabled() {
            abs::check::make_unknown
        } else {
            abs::Entry::checker
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
