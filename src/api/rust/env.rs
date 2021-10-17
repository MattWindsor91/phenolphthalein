//! Native-Rust shared environment and related types.

use crate::{api::abs, err, model::slot};
use std::{
    cell::UnsafeCell,
    sync::{self, atomic::AtomicI32},
};

/// A native-Rust implementation of the environment.
pub struct Env {
    /// The 32-bit slots.
    pub i32s: Slotset<AtomicI32, i32>,
}

impl abs::Env for Env {
    fn of_reservations(reservations: slot::ReservationSet) -> err::Result<Self> {
        let slot::ReservationSet { i32s } = reservations;
        Ok(Env {
            i32s: Slotset::new(&i32s),
        })
    }

    fn get_i32(&self, slot: slot::Slot) -> i32 {
        self.i32s.get(slot)
    }

    fn set_i32(&mut self, slot: slot::Slot, v: i32) {
        self.i32s.set(slot, v);
    }
}

/// A set of atomic and non-atomic slots for a particular type.
///
/// `A` should be the atomic equivalent of `T`.
pub struct Slotset<A, T> {
    /// The atomic component of this slot-set.
    ///
    /// Atomics can always be read from and written to on an aliased borrow,
    /// so these are just `A`.
    pub atomic: Vec<A>,

    /// The non-atomic component of this slot-set.
    ///
    /// Non-atomics can only be read from and written to based on Rust's usual
    /// ownership rules.  We don't yet enforce these rules ourselves (partly
    /// because thread-local environments aren't implemented and partly
    /// because we sometimes need global non-atomic variables in tests), and
    /// consequently the test writer must assert the safety of each use
    /// themselves by unwrapping an [UnsafeCell].
    pub non_atomic: Vec<UnsafeCell<T>>,
}

impl<A: Default, T: Default> Slotset<A, T> {
    /// Constructs a new slotset from a slot reservation.
    pub fn new(res: &slot::Reservation<T>) -> Self {
        Self {
            atomic: default_vec(res.atomic),
            non_atomic: default_vec(res.non_atomic),
        }
    }
}

/// Trait for things that can be loaded and stored, atomically, to a slot.
///
/// This is mostly just a thin layer over the atomic load/stores in each atomic
/// type, but forces relaxed semantics.
pub trait SlotAtomic<T> {
    /// Atomically loads a `T` from this variable with relaxed ordering.
    fn slot_load(&self) -> T;
    /// Atomically stores a `T` to this variable with relaxed ordering.
    fn slot_store(&self, val: T);
}

impl SlotAtomic<i32> for AtomicI32 {
    fn slot_load(&self) -> i32 {
        self.load(sync::atomic::Ordering::Relaxed)
    }

    fn slot_store(&self, val: i32) {
        self.store(val, sync::atomic::Ordering::Relaxed);
    }
}

impl<A: SlotAtomic<T>, T: Copy + Default> Slotset<A, T> {
    pub fn get(&self, slot: slot::Slot) -> T {
        if slot.is_atomic {
            self.get_atomic(slot.index)
        } else {
            self.get_non_atomic(slot.index)
        }
    }

    fn get_atomic(&self, index: usize) -> T {
        self.atomic.get(index).map(A::slot_load).unwrap_or_default()
    }

    fn get_non_atomic(&self, index: usize) -> T {
        /* TODO(@MattWindsor91): is this sound?  The rationale would be
        that any mutable borrows of this by phph itself will be behind
        a mutable reference to the Slotset, and any mutable borrows by
        the test are for the test writer to assert safety over. */
        self.non_atomic
            .get(index)
            .map(|s| unsafe { *s.get() })
            .unwrap_or_default()
    }

    pub fn set(&mut self, slot: slot::Slot, v: T) {
        if slot.is_atomic {
            if let Some(s) = self.atomic.get(slot.index) {
                s.slot_store(v);
            }
        } else if let Some(s) = self.non_atomic.get_mut(slot.index) {
            *s.get_mut() = v;
        }
    }
}

/// Constructs a vector of type `T`, length `len`, and contents `default`.
fn default_vec<T: Default>(len: usize) -> Vec<T> {
    let mut v = Vec::with_capacity(len);
    v.resize_with(len, Default::default);
    v
}

#[cfg(test)]
mod tests {
    use crate::{api::abs::test_helpers, err};

    #[test]
    /// Tests getting and setting a 32-bit atomic integer.
    fn test_get_set_atomic_i32() -> err::Result<()> {
        test_helpers::test_i32_get_set::<super::Env>(true)
    }

    #[test]
    /// Tests getting and setting a 32-bit integer.
    fn test_get_set_i32() -> err::Result<()> {
        test_helpers::test_i32_get_set::<super::Env>(false)
    }
}
