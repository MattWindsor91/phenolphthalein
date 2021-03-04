//! The abstract test API.
//!
//! This module contains the various traits that the actual language APIs
//! implement.
use crate::{err, model};

pub mod check;

pub use check::Checker;

/// Trait of top-level tests.
///
/// Each test can spawn multiple entry points into itself.
pub trait Test<'a> {
    /// The type of entry point into the test.
    type Entry: Entry<'a>;

    /// Spawns a new entry point into the test.
    fn spawn(&self) -> Self::Entry;
}

/// Trait of cloneable entry points into tests.
pub trait Entry<'a>: Clone + 'a {
    /// Every test entry has an associated environment type, which implements
    /// a fairly basic API for inspection and resetting.
    type Env: Env + 'a;

    /// Makes a manifest using information taken from the test entry point.
    fn make_manifest(&self) -> err::Result<model::manifest::Manifest>;

    /// Runs the entry point given a thread ID and handle to the environment.
    fn run(&self, tid: usize, e: &Self::Env);

    /// Gets a checker for this entry point's environments.
    fn checker(&self) -> Box<dyn check::Checker<Self::Env> + 'a>;
}

pub fn option_checker<'a, E, T>(
    maker: fn(T) -> Box<dyn check::Checker<E> + 'a>,
    opt: Option<T>,
) -> Box<dyn check::Checker<E> + 'a> {
    if let Some(precursor) = opt {
        maker(precursor)
    } else {
        Box::new(model::Outcome::Unknown)
    }
}

/// Trait of medium-level handles to an observable test environment.
///
/// This trait currently mainly exists to hide parts of the actual environment
/// that aren't thread-safe to run, but may be more useful later on.
pub trait Env: Sized {
    /// Constructs an environment for the given slot reservations.
    fn of_reservations(r: model::slot::ReservationSet) -> err::Result<Self>;

    /// Gets the 32-bit integer in the given slot.
    /// Assumes that the implementation does range checking and returns a
    /// valid but undefined result if i is out of bounds.
    fn get_i32(&self, slot: model::slot::Slot) -> i32;

    /// Sets the 32-bit integer in the given slot to value v.
    fn set_i32(&mut self, slot: model::slot::Slot, v: i32);
}

#[cfg(test)]
pub mod test_helpers {
    use crate::{
        err,
        model::slot::{Reservation, ReservationSet, Slot},
    };
    use std::iter::once;

    pub fn test_i32_get_set<E: super::Env>(is_atomic: bool) -> err::Result<()> {
        let slot = Slot {
            index: 0,
            is_atomic,
        };
        let reservation = ReservationSet {
            i32s: Reservation::of_slots(once(slot).into_iter()),
        };
        let mut env = E::of_reservations(reservation)?;

        assert_eq!(0, env.get_i32(slot));
        env.set_i32(slot, 42);
        assert_eq!(42, env.get_i32(slot));

        Ok(())
    }
}
