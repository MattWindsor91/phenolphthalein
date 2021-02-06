//! The abstract test API.
//!
//! This module contains the various traits that the actual language APIs
//! implement.
use crate::{err, model};

/// Trait of top-level tests.
///
/// Each test can spawn multiple entry points into itself.
pub trait Test<'a> {
    /// The type of entry point into the test.
    type Entry: Entry;

    /// Spawns a new entry point into the test.
    fn spawn(&self) -> Self::Entry;
}

/// Trait of cloneable entry points into tests.
pub trait Entry: Clone {
    /* NOTE(@MattWindsor91): this will likely need a lifetime adding to it
    eventually; I think its lack of one thus far is a quirk of how
    dlopen Containers manage lifetimes. */

    /// Every test entry has an associated environment type, which implements
    /// a fairly basic API for inspection and resetting.
    type Env: Env;

    /// Test entries must also have an associated checker type, for checking
    /// environments uphold test conditions.
    type Checker: Checker<Env = Self::Env>;

    /// Makes a manifest using information taken from the test entry point.
    fn make_manifest(&self) -> err::Result<model::manifest::Manifest>;

    /// Runs the entry point given a thread ID and handle to the environment.
    fn run(&self, tid: usize, e: &mut Self::Env);

    /// Gets a checker for this entry point's environments.
    fn checker(&self) -> Self::Checker;
}

/// Type of functions that can check an environment.
pub trait Checker: Sync + Send + Clone {
    // The type of the environment this checker checks.
    type Env: Env;

    /// Checks the current state of the environment.
    fn check(&self, env: &Self::Env) -> model::check::Outcome;
}

/// Trait of medium-level handles to an observable test environment.
///
/// This trait currently mainly exists to hide parts of the actual environment
/// that aren't thread-safe to run, but may be more useful later on.
pub trait Env: Sized + Clone {
    /// Constructs an environment for the given manifest.
    fn for_manifest(m: &model::manifest::Manifest) -> err::Result<Self>;

    /// Gets the atomic 32-bit integer in slot i.
    /// Assumes that the implementation does range checking and returns a
    /// valid but undefined result if i is out of bounds.
    fn get_atomic_i32(&self, i: usize) -> i32;

    /// Gets the non-atomic 32-bit integer in slot i.
    /// Assumes that the implementation does range checking and returns a
    /// valid but undefined result if i is out of bounds.
    fn get_i32(&self, i: usize) -> i32;

    /// Sets the atomic 32-bit integer in slot i to value v.
    fn set_atomic_i32(&mut self, i: usize, v: i32);

    /// Sets the non-atomic 32-bit integer in slot i to value v.
    fn set_i32(&mut self, i: usize, v: i32);
}
