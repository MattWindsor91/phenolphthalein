//! Model types for checks.

/// The result of running a checker.
///
/// Outcomes are ordered such that `max` on an iterator of outcomes will return
/// the correct final outcome (`None` if the outcomes are empty, `Unknown` if
/// any were unknown, `Passed` if all are passes, and `Failed` otherwise).
#[derive(Copy, Clone, Debug, PartialOrd, PartialEq, Eq, Ord)]
pub enum Outcome {
    /// The observation passed its check.
    Passed,
    /// The observation failed its check.
    Failed,
    /// The observation has no determined outcome.
    Unknown,
}

impl Outcome {
    /// Converts a pass/fail Boolean to an Outcome.
    pub fn from_pass_bool(is_pass: bool) -> Self {
        if is_pass {
            Self::Passed
        } else {
            Self::Failed
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
        let v = vec![Outcome::Passed, Outcome::Passed, Outcome::Passed];
        assert_eq!(v.into_iter().max(), Some(Outcome::Passed))
    }

    #[test]
    /// `max` of an iterator of v should return a fail.
    fn test_max_failed() {
        let v = vec![Outcome::Failed, Outcome::Failed, Outcome::Failed];
        assert_eq!(v.into_iter().max(), Some(Outcome::Failed))
    }

    #[test]
    /// `max` of a mixed determinate iterator should return a fail.
    fn test_max_mixed() {
        let v = vec![Outcome::Passed, Outcome::Failed, Outcome::Passed];
        assert_eq!(v.into_iter().max(), Some(Outcome::Failed))
    }

    #[test]
    /// `max` of an iterator with one unknown should return an unknown.
    fn test_max_unknown() {
        let v = vec![Outcome::Unknown, Outcome::Failed, Outcome::Passed];
        assert_eq!(v.into_iter().max(), Some(Outcome::Unknown))
    }
}
