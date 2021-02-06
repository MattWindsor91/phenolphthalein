use crate::{err, model::manifest, testapi::abs};
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
    /// Gets the atomic integer in slot i.
    /// Assumes that the C implementation does range checking and returns a
    /// valid but undefined result if i is out of bounds.
    fn get_atomic_i32(&self, i: usize) -> i32 {
        unsafe { get_atomic_int32(self.p, i) }
    }

    /// Gets the integer in slot i.
    /// Assumes that the C implementation does range checking and returns a
    /// valid but undefined result if i is out of bounds.
    fn get_i32(&self, i: usize) -> i32 {
        unsafe { get_int32(self.p, i) }
    }

    fn set_atomic_i32(&mut self, i: usize, v: i32) {
        unsafe { set_atomic_int32(self.p, i, v) }
    }

    fn set_i32(&mut self, i: usize, v: i32) {
        unsafe { set_int32(self.p, i, v) }
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

#[cfg(test)]
mod tests {
    use crate::testapi::abs::Env;

    #[test]
    /// Tests storing and loading a 32-bit atomic integer.
    fn test_store_load_atomic_i32() {
        let mut env = super::Env::new(1, 0).unwrap();
        let env2 = env.clone();
        assert_eq!(0, env.get_atomic_i32(0));
        env.set_atomic_i32(0, 42);
        assert_eq!(42, env.get_atomic_i32(0));
        assert_eq!(42, env2.get_atomic_i32(0))
    }

    #[test]
    /// Tests storing and loading a 32-bit integer.
    fn test_store_load_i32() {
        let mut env = super::Env::new(0, 1).unwrap();
        let env2 = env.clone();
        assert_eq!(0, env.get_i32(0));
        env.set_i32(0, 42);
        assert_eq!(42, env.get_i32(0));
        assert_eq!(42, env2.get_i32(0))
    }
}
