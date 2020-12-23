use crate::{err, manifest};

/// Trait of handles to an observable test environment.
///
/// This trait currently mainly exists to hide parts of the actual environment
/// that aren't thread-safe to run, but may be more useful later on.
pub trait Env: Sized + Clone {
    /// Constructs an environment for the given manifest.
    fn for_manifest(m: &manifest::Manifest) -> err::Result<Self>;

    /// Gets the atomic integer in slot i.
    /// Assumes that the implementation does range checking and returns a
    /// valid but undefined result if i is out of bounds.
    fn atomic_int(&self, i: usize) -> i32;

    /// Gets the integer in slot i.
    /// Assumes that the implementation does range checking and returns a
    /// valid but undefined result if i is out of bounds.
    fn int(&self, i: usize) -> i32;

    fn set_atomic_int(&mut self, i: usize, v: i32);

    fn set_int(&mut self, i: usize, v: i32);
}
