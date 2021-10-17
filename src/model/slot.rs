//! Slots and slot reservations.
//!
//! Slots are indices into the environment's variable table.
//! Slots reference both whether the variable is atomic, and its index in the
//! particular type/atomicity list.

/// The location of a variable in its type-specific variable mapping.
#[derive(Copy, Clone)]
pub struct Slot {
    /// Whether this slot names an atomic variable or not.
    pub is_atomic: bool,

    /// The index of this slot.
    pub index: usize,
}

/// A pair of sizes determining the number of atomic and non-atomic slots to
/// reserve in the environment for the variables mentioned in a manifest.
///
/// The type parameter is phantom and used to distinguish reservations for
/// different types of integer.
#[derive(Default)]
pub struct Reservation<T> {
    // Phantom data to keep T in the type parameter.
    _marker: std::marker::PhantomData<T>,

    /// The number of atomic slots to reserve.
    pub atomic: usize,
    /// The number of non-atomic slots to reserve.
    pub non_atomic: usize,
}

impl<T> Reservation<T> {
    /// Extends the slot reservation to account for this variable's slot.
    #[must_use]
    pub fn add_slot(mut self, slot: Slot) -> Self {
        let count = if slot.is_atomic {
            &mut self.atomic
        } else {
            &mut self.non_atomic
        };

        *count = usize::max(*count, slot.index + 1);
        self
    }
}

impl<T: Default> Reservation<T> {
    /// Produces a reservation by folding over a slot iterator.
    pub fn of_slots(slots: impl Iterator<Item = Slot>) -> Self {
        slots.fold(Reservation::default(), Self::add_slot)
    }
}

/// A set of slot reservations.
pub struct ReservationSet {
    /// The reservations for 32-bit integers.
    pub i32s: Reservation<i32>,
}
