use slot::ReservationSet;

use crate::{api::abs, err, model::slot};
use std::ptr;

/// Dummy object used to represent pointers to C environments.
///
/// Some parts of the C ABI need to work with this directly, so it is exposed
/// throughout the `c` module.
#[repr(C)]
pub(super) struct UnsafeEnv {
    _private: [u8; 0],
}

extern "C" {
    fn alloc_env(atomic_ints: libc::size_t, ints: libc::size_t) -> *mut UnsafeEnv;
    fn free_env(e: *mut UnsafeEnv);
    fn get_atomic_int32(e: *const UnsafeEnv, index: libc::size_t) -> i32;
    fn get_int32(e: *const UnsafeEnv, index: libc::size_t) -> i32;
    fn set_atomic_int32(e: *mut UnsafeEnv, index: libc::size_t, value: i32);
    fn set_int32(e: *mut UnsafeEnv, index: libc::size_t, value: i32);
}

/// Thin layer over the C environment struct.
pub struct Env {
    /// The C thread environment.
    ///
    /// Some parts of the C ABI need to work with this directly, so it is exposed
    /// throughout the `c` module.
    pub(super) p: *mut UnsafeEnv,
}

impl abs::Env for Env {
    /// Gets the 32-bit integer in slot slot.
    /// Assumes that the C implementation does range checking and returns a
    /// valid but undefined result if i is out of bounds.
    fn get_i32(&self, slot: slot::Slot) -> i32 {
        if slot.is_atomic {
            unsafe { get_atomic_int32(self.p, slot.index) }
        } else {
            unsafe { get_int32(self.p, slot.index) }
        }
    }

    fn set_i32(&mut self, slot: slot::Slot, v: i32) {
        if slot.is_atomic {
            unsafe { set_atomic_int32(self.p, slot.index, v) }
        } else {
            unsafe { set_int32(self.p, slot.index, v) }
        }
    }

    fn of_reservations(reservations: slot::ReservationSet) -> err::Result<Self> {
        let ReservationSet { i32s } = reservations;

        let mut e = Env { p: ptr::null_mut() };
        unsafe {
            e.p = alloc_env(i32s.atomic, i32s.non_atomic);
        }
        if e.p.is_null() {
            Err(err::Error::EnvAllocFailed)
        } else {
            Ok(e)
        }
    }
}

/// Envs can be dropped.
impl Drop for Env {
    fn drop(&mut self) {
        unsafe {
            free_env(self.p);
            self.p = ptr::null_mut();
        }
    }
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
