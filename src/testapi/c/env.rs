use crate::{env, err, manifest};
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
    fn copy_env(e: *mut UnsafeEnv) -> *mut UnsafeEnv;
    fn free_env(e: *mut UnsafeEnv);
    fn get_atomic_int(e: *const UnsafeEnv, index: libc::size_t) -> libc::c_int;
    fn get_int(e: *const UnsafeEnv, index: libc::size_t) -> libc::c_int;
    fn set_atomic_int(e: *mut UnsafeEnv, index: libc::size_t, value: libc::c_int);
    fn set_int(e: *mut UnsafeEnv, index: libc::size_t, value: libc::c_int);
}

/// Thin layer over the C environment struct.
pub struct Env {
    /// The C thread environment.
    ///
    /// Some parts of the C ABI need to work with this directly, so it is exposed
    /// throughout the `c` module.
    pub(super) p: *mut UnsafeEnv,
}

impl env::Env for Env {
    /// Gets the atomic integer in slot i.
    /// Assumes that the C implementation does range checking and returns a
    /// valid but undefined result if i is out of bounds.
    fn atomic_int(&self, i: usize) -> i32 {
        unsafe { get_atomic_int(self.p, i) }
    }

    /// Gets the integer in slot i.
    /// Assumes that the C implementation does range checking and returns a
    /// valid but undefined result if i is out of bounds.
    fn int(&self, i: usize) -> i32 {
        unsafe { get_int(self.p, i) }
    }

    fn set_atomic_int(&mut self, i: usize, v: i32) {
        unsafe { set_atomic_int(self.p, i, v) }
    }

    fn set_int(&mut self, i: usize, v: i32) {
        unsafe { set_int(self.p, i, v) }
    }

    fn for_manifest(m: &manifest::Manifest) -> err::Result<Self> {
        Self::new(m.atomic_ints.len(), m.ints.len())
    }
}

/// Envs can be dropped.
///
/// We rely on the `UnsafeEnv` having a reference counter or similar scheme.
impl Drop for Env {
    fn drop(&mut self) {
        unsafe {
            free_env(self.p);
            self.p = ptr::null_mut();
        }
    }
}

/// Envs can be cloned.
///
/// We again rely on the `UnsafeEnv` to implement the right semantics.
impl Clone for Env {
    fn clone(&self) -> Self {
        let p;
        // TODO(@MattWindsor91): what if this returns null?
        unsafe {
            p = copy_env(self.p);
        }
        Env { p }
    }
}

impl Env {
    /// Creates a new environment with the given dimensions.
    pub fn new(num_atomic_ints: usize, num_ints: usize) -> err::Result<Self> {
        let mut e = Env { p: ptr::null_mut() };
        unsafe {
            e.p = alloc_env(num_atomic_ints, num_ints);
        }
        if e.p.is_null() {
            Err(err::Error::EnvAllocFailed)
        } else {
            Ok(e)
        }
    }
}
