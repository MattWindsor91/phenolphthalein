use dlopen::symbor::{Library, SymBorApi, Symbol};

use crate::env;
use std::ptr;

#[repr(C)]
pub struct UnsafeEnv {
    _private: [u8; 0],
}

extern "C" {
    pub fn alloc_env(atomic_ints: libc::size_t, ints: libc::size_t) -> *mut UnsafeEnv;
    pub fn copy_env(e: *mut UnsafeEnv) -> *mut UnsafeEnv;
    pub fn free_env(e: *mut UnsafeEnv);
    pub fn get_atomic_int(e: *const UnsafeEnv, index: libc::size_t) -> libc::c_int;
    pub fn get_int(e: *const UnsafeEnv, index: libc::size_t) -> libc::c_int;
    pub fn set_atomic_int(e: *mut UnsafeEnv, index: libc::size_t, value: libc::c_int);
    pub fn set_int(e: *mut UnsafeEnv, index: libc::size_t, value: libc::c_int);
}

#[derive(SymBorApi, Clone)]
pub struct CTestApi<'a> {
    test: Symbol<'a, unsafe extern "C" fn(tid: libc::size_t, env: *mut UnsafeEnv)>
}

// Enumeration of errors that can happen with test creation.
#[derive(Debug)]
pub enum Error {
    EnvAllocFailed,
    NotEnoughThreads,
    DlopenFailed(dlopen::Error)
}
pub type Result<T> = std::result::Result<T, Error>;

impl From<dlopen::Error> for Error {
    fn from(e : dlopen::Error) -> Self {
        Self::DlopenFailed(e)
    }
}


/// Thin layer over the C environment struct, also wrapping in the test stub.
pub struct Env {
    /// The C thread environment.
    p: *mut UnsafeEnv,
}

impl env::AnEnv for Env {
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
}

/// Envs can be dropped.
///
/// We rely on the UnsafeEnv having a reference counter or similar scheme.
impl Drop for Env {
    fn drop(&mut self) {
        unsafe {
            free_env(self.p);
            self.p = ptr::null_mut();
        }
    }
}

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
    pub fn new(num_atomic_ints: usize, num_ints: usize) -> Result<Self> {
        let mut e = Env { p: ptr::null_mut() };
        unsafe {
            e.p = alloc_env(num_atomic_ints, num_ints);
        }
        if e.p.is_null() {
            Err(Error::EnvAllocFailed)
        } else {
            Ok(e)
        }
    }
}

impl CTestApi<'_> {
    pub fn run(&self, tid: usize, e: &mut Env) {
        unsafe {
            (self.test)(tid, e.p)
        }
    }
}

pub fn load_test<'a>(lib: &'a Library) -> Result<CTestApi<'a>> {
    let c = unsafe { CTestApi::load(&lib) }?;
    Ok(c)
}
