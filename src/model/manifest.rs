//! Test manifests.
use std::collections::BTreeMap;

/// A test manifest, describing properties of a test.
#[derive(Clone)]
pub struct Manifest {
    /// The number of threads available in the test.
    pub n_threads: usize,
    /// Ordered map of atomic int variables declared in the test.
    pub atomic_ints: VarMap<i32>,
    /// Ordered map of int variables declared in the test.
    pub ints: VarMap<i32>,
}

impl<'a> Manifest {
    /// Iterates over the names of each atomic int variable, in order.
    pub fn atomic_int_names(&'a self) -> impl Iterator<Item = &'a str> + '_ {
        self.atomic_ints.keys().map(String::as_str)
    }

    /// Iterates over the names of each int variable, in order.
    pub fn int_names(&'a self) -> impl Iterator<Item = &'a str> + '_ {
        self.ints.keys().map(String::as_str)
    }
}

/// Type alias for ordered variable maps.
type VarMap<T> = BTreeMap<String, VarRecord<T>>;

/// A variable record in a test manifest.
#[derive(Clone)]
pub struct VarRecord<T> {
    pub initial_value: Option<T>, // Space for rent
}
