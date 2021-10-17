//! Outcomes of running checks on observations.

use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};

/// The result of running a checker.
///
/// Outcomes are ordered such that `max` on an iterator of outcomes will return
/// the correct final outcome (`None` if the outcomes are empty, `Unknown` if
/// any were unknown, `Pass` if all are passes, and `Fail` otherwise).
///
/// Outcomes are non-exhaustive, in the unlikely case that we add more.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    /// The observation passed its check.
    Pass,
    /// The observation failed its check.
    Fail,
    /// The observation has no determined outcome.
    Unknown,
}

/// String representations for outcomes.
pub mod string {
    /// String representation for pass outcomes.
    pub const PASS: &str = "pass";
    /// String representation for fail outcomes.
    pub const FAIL: &str = "fail";
    /// String representation for unknown outcomes.
    pub const UNKNOWN: &str = "unknown";
}

/// We can produce a string representation of the outcome.
///
/// # Examples
///
/// ```
/// use phenolphthalein::model::Outcome;
/// assert_eq!(Outcome::Pass.to_string(), "pass");
/// assert_eq!(Outcome::Fail.to_string(), "fail");
/// assert_eq!(Outcome::Unknown.to_string(), "unknown");
/// ```
impl Display for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Pass => string::PASS,
                Self::Fail => string::FAIL,
                Self::Unknown => string::UNKNOWN,
            }
        )
    }
}

/// We can parse a string representation of an outcome.  Parsing is (relatively)
/// case insensitive.
///
/// # Examples
///
/// ```
/// use phenolphthalein::model::Outcome;
/// assert_eq!(str::parse::<Outcome>("pass"), Ok(Outcome::Pass));
/// assert_eq!(str::parse::<Outcome>("Fail"), Ok(Outcome::Fail));
/// assert_eq!(str::parse::<Outcome>("UNKNOWN"), Ok(Outcome::Unknown));
/// ```
impl FromStr for Outcome {
    /// Errors just take ownership of the invalid string.
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lc = s.to_lowercase();
        match &*lc {
            string::PASS => Ok(Self::Pass),
            string::FAIL => Ok(Self::Fail),
            string::UNKNOWN => Ok(Self::Unknown),
            _ => Err(lc),
        }
    }
}

/// The default outcome is `Outcome::Unknown`.
impl Default for Outcome {
    fn default() -> Self {
        Self::Unknown
    }
}

impl Outcome {
    /// Gets an iterator of each [Outcome].
    ///
    /// # Examples
    ///
    /// ```
    /// use phenolphthalein::model::Outcome;
    /// let set: std::collections::HashSet<Outcome> = Outcome::all().collect();
    ///
    /// assert!(set.contains(&Outcome::Pass));
    /// assert!(set.contains(&Outcome::Fail));
    /// assert!(set.contains(&Outcome::Unknown));
    /// ```
    pub fn all() -> impl Iterator<Item = Self> {
        vec![Self::Pass, Self::Fail, Self::Unknown].into_iter()
    }

    /// Converts a pass/fail Boolean to an [Outcome].
    ///
    /// # Examples
    ///
    /// ```
    /// use phenolphthalein::model::outcome::Outcome;
    /// assert_eq!(Outcome::from_pass_bool(true), Outcome::Pass);
    /// assert_eq!(Outcome::from_pass_bool(false), Outcome::Fail);
    /// ```
    #[must_use]
    pub fn from_pass_bool(is_pass: bool) -> Self {
        if is_pass {
            Self::Pass
        } else {
            Self::Fail
        }
    }
}
#[cfg(test)]
mod test {
    use super::Outcome;

    #[test]
    /// `max` of an empty iterator should return `None`.
    fn test_max_empty() {
        let v: std::vec::Vec<Outcome> = vec![];
        assert_eq!(v.into_iter().max(), None)
    }

    #[test]
    /// `max` of an iterator of passes should return a pass.
    fn test_max_passes() {
        let v = vec![Outcome::Pass, Outcome::Pass, Outcome::Pass];
        assert_eq!(v.into_iter().max(), Some(Outcome::Pass))
    }

    #[test]
    /// `max` of an iterator of v should return a fail.
    fn test_max_fail() {
        let v = vec![Outcome::Fail, Outcome::Fail, Outcome::Fail];
        assert_eq!(v.into_iter().max(), Some(Outcome::Fail))
    }

    #[test]
    /// `max` of a mixed determinate iterator should return a fail.
    fn test_max_mixed() {
        let v = vec![Outcome::Pass, Outcome::Fail, Outcome::Pass];
        assert_eq!(v.into_iter().max(), Some(Outcome::Fail))
    }

    #[test]
    /// `max` of an iterator with one unknown should return an unknown.
    fn test_max_unknown() {
        let v = vec![Outcome::Unknown, Outcome::Fail, Outcome::Pass];
        assert_eq!(v.into_iter().max(), Some(Outcome::Unknown))
    }
}
