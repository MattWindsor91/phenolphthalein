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

/// A borrowed environment combined with a borrowed manifest.
///
/// Bundling these two together lets us interpret the environment using the
/// manifest.
///
/// For this to be safe, we assume that the environment gracefully handles any
/// mismatches between itself and the manifest.
pub struct Manifested<'a, T> {
    pub manifest: &'a manifest::Manifest,
    pub env: &'a mut T,
}

impl<'a, T: Env> Manifested<'a, T> {
    /// Resets the environment to the initial values in the manifest.
    pub fn reset(&mut self) {
        for (i, (_, r)) in self.manifest.atomic_ints.iter().enumerate() {
            self.env.set_atomic_int(i, r.initial_value.unwrap_or(0))
        }
        for (i, (_, r)) in self.manifest.ints.iter().enumerate() {
            self.env.set_int(i, r.initial_value.unwrap_or(0))
        }
    }

    // Iterates over all of the atomic integer variables in the environment.
    pub fn atomic_int_values(&self) -> impl Iterator<Item = (String, i32)> + '_ {
        self.manifest
            .atomic_int_names()
            .enumerate()
            .map(move |(i, n)| (n.to_string(), self.env.atomic_int(i)))
    }

    // Iterates over all of the integer variables in the environment.
    pub fn int_values(&self) -> impl Iterator<Item = (String, i32)> + '_ {
        self.manifest
            .int_names()
            .enumerate()
            .map(move |(i, n)| (n.to_string(), self.env.int(i)))
    }
}
