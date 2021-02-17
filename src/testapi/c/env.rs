use crate::{
    err,
    model::{manifest, slot},
    testapi::abs,
};
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

    fn for_manifest(m: &manifest::Manifest) -> err::Result<Self> {
        Self::new(m.reserve_i32s())
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

impl Env {
    /// Creates a new environment with the given dimensions.
    pub fn new(i32s: slot::Reservation<i32>) -> err::Result<Self> {
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

#[cfg(test)]
mod tests {
    use crate::{
        model::slot::{Reservation, Slot},
        testapi::abs::Env,
    };

    #[test]
    /// Tests storing and loading a 32-bit atomic integer.
    fn test_store_load_atomic_i32() {
        let slot = Slot {
            index: 0,
            is_atomic: true,
        };
        let mut env = super::Env::new(Reservation::of_slots(vec![slot].into_iter())).unwrap();

        assert_eq!(0, env.get_i32(slot));
        env.set_i32(slot, 42);
        assert_eq!(42, env.get_i32(slot));
    }

    #[test]
    /// Tests storing and loading a 32-bit integer.
    fn test_store_load_i32() {
        let slot = Slot {
            index: 0,
            is_atomic: false,
        };
        let mut env = super::Env::new(Reservation::of_slots(vec![slot].into_iter())).unwrap();

        assert_eq!(0, env.get_i32(slot));
        env.set_i32(slot, 42);
        assert_eq!(42, env.get_i32(slot));
    }
}
