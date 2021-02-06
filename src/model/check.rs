//! Model types for checks.

/// Trait of things that can check an environment.
///
/// Checkers are expected to be movable across thread boundaries, unlike
/// `Fn(E) -> Outcome`.
pub trait Checker<E>: Sync + Send {
    /// Checks the current state of the environment.
    fn check(&self, env: &E) -> Outcome;
}

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

/// `Outcome`s can be trivial `Checker`s; they always return themselves.
impl<E> Checker<E> for Outcome {
    fn check(&self, _env: &E) -> Outcome {
        *self
    }
}

#[cfg(test)]
mod test {
    use super::{Checker, Outcome};

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

    #[test]
    /// Outcomes return themselves when used as checks.
    fn test_outcome_as_check() {
        for x in [Outcome::Unknown, Outcome::Failed, Outcome::Passed].iter() {
            assert_eq!(*x, x.check(&()))
        }
    }
}
