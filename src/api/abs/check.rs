//! The checker API.

use crate::model;

/// Trait of things that can check an environment.
///
/// Checkers are expected to be movable across thread boundaries, unlike
/// `Fn(E) -> Outcome`.
pub trait Checker<E>: Sync + Send {
    /// Checks the current state of the environment.
    fn check(&self, env: &E) -> model::Outcome;
}

/// Type alias of functions that return fully wrapped synchronisers.
pub type Factory<'a, S, E> = fn(&S) -> Box<dyn Checker<E> + 'a>;

/// `Outcome`s can be trivial `Checker`s; they always return themselves.
///
/// # Examples
///
/// ```
/// use phenolphthalein::api::abs::check::Checker;
/// use phenolphthalein::model::Outcome;
/// assert_eq!(Outcome::Pass, Outcome::Pass.check(&()));
/// assert_eq!(Outcome::Fail, Outcome::Fail.check(&()));
/// assert_eq!(Outcome::Unknown, Outcome::Unknown.check(&()));
/// ```
impl<E> Checker<E> for model::Outcome {
    fn check(&self, _env: &E) -> Self {
        *self
    }
}

/// Constructs a checker for any environment type that just returns [model::Outcome::Unknown].
///
/// This gains nothing over just using [model::Outcome::Unknown] as a checker, except that it is
/// the right shape to be a [Factory].
pub fn unknown_factory<'a, T, E>(_: &T) -> Box<dyn Checker<E> + 'a> {
    Box::new(model::Outcome::Unknown)
}

#[cfg(test)]
mod test {
    use super::Checker;
    use crate::model::Outcome;

    #[test]
    /// Outcomes return themselves when used as checks.
    fn test_outcome_as_check() {
        for x in [Outcome::Unknown, Outcome::Fail, Outcome::Pass].iter() {
            assert_eq!(*x, x.check(&()))
        }
    }
}
