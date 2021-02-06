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
    type Entry: Entry<'a>;

    /// Spawns a new entry point into the test.
    fn spawn(&self) -> Self::Entry;
}

/// Trait of cloneable entry points into tests.
pub trait Entry<'a>: Clone {
    /// Every test entry has an associated environment type, which implements
    /// a fairly basic API for inspection and resetting.
    type Env: Env + 'a;

    /// Makes a manifest using information taken from the test entry point.
    fn make_manifest(&self) -> err::Result<model::manifest::Manifest>;

    /// Runs the entry point given a thread ID and handle to the environment.
    fn run(&self, tid: usize, e: &mut Self::Env);

    /// Gets a checker for this entry point's environments.
    fn checker(&self) -> Box<dyn Checker<Self::Env> + 'a>;
}

/// Type of functions that can check an environment.
pub trait Checker<E>: Sync + Send {
    /// Checks the current state of the environment.
    fn check(&self, env: &E) -> model::check::Outcome;
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
