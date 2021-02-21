//! Test manifests.
use super::slot::{Reservation, ReservationSet, Slot};
use std::collections::BTreeMap;

/// A test manifest, describing properties of a test.
#[derive(Clone)]
pub struct Manifest {
    /// The number of threads available in the test.
    pub n_threads: usize,
    /// Ordered map of int variables declared in the test.
    pub i32s: VarMap<i32>,
}

impl Manifest {
    /// Constructs a slot reservation wide enough for the variables in this
    /// manifest.
    pub fn reserve(&self) -> ReservationSet {
        ReservationSet {
            i32s: reserve_var_map(&self.i32s),
        }
    }
}

/// Type alias for ordered variable maps.
pub type VarMap<T> = BTreeMap<String, VarRecord<T>>;

fn reserve_var_map<T: Default>(map: &VarMap<T>) -> Reservation<T> {
    Reservation::of_slots(map.values().map(|x| x.slot))
}

/// A variable record in a test manifest.
#[derive(Clone)]
pub struct VarRecord<T> {
    /// The initial value of the variable, if one exists.
    pub initial_value: Option<T>,

    /// The slot of the variable.
    pub slot: Slot,
}
