//! Helpers for implementing tests over the abstract test API.
use crate::{
    err,
    model::slot::{Reservation, ReservationSet, Slot},
};
use std::iter::once;

/// Generic testing function for environments' i32 getter/setters pairs.
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
