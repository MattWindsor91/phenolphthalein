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

/// Function pointers are trivial checkers.
impl<E> Checker<E> for fn(&E) -> model::Outcome {
    fn check(&self, env: &E) -> model::Outcome {
        (self)(env)
    }
}

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

/// Type alias of functions that return fully wrapped synchronisers.
pub type Factory<'a, S, E> = fn(&S) -> Box<dyn Checker<E> + 'a>;

/// Constructs a checker for any environment type that just returns [model::Outcome::Unknown].
///
/// This gains nothing over just using [model::Outcome::Unknown] as a checker, except that it is
/// the right shape to be a [Factory].
pub fn make_unknown<'a, T, E>(_: &T) -> Box<dyn Checker<E> + 'a> {
    Box::new(model::Outcome::Unknown)
}

/// Lifts an optional item `opt` into a boxed checker using `maker`, if it
/// exists; returns a boxed checker that constantly returns
/// [model::Outcome::Unknown] otherwise.
///
/// # Examples
///
/// ```
/// use phenolphthalein::api::abs::check::{Checker, Factory, make_optional};
/// use phenolphthalein::model::Outcome;
/// let mut opt = Some(Outcome::Pass);
/// let checker = make_optional(|x| Box::new(*x), &opt);
/// assert_eq!(Outcome::Pass, checker.check(&()));
/// opt.take();
/// let checker = make_optional(|x| Box::new(*x), &opt);
/// assert_eq!(Outcome::Unknown, checker.check(&()));
/// ```
pub fn make_optional<'a, E, T>(
    maker: impl Fn(&T) -> Box<dyn Checker<E> + 'a>,
    opt: &Option<T>,
) -> Box<dyn Checker<E> + 'a> {
    if let Some(precursor) = opt {
        maker(precursor)
    } else {
        Box::new(model::Outcome::Unknown)
    }
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
