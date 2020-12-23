use dlopen::symbor::{Library, Ref, SymBorApi, Symbol};

use crate::{env, err, manifest, obs, test};
use std::{collections::BTreeMap, ffi, ptr};

#[repr(C)]
struct UnsafeEnv {
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

/// The manifest structure that the test implements to communicate auxiliary
/// information to the test runner.
///
/// This must line up with 'struct manifest' in phenol.h.
#[repr(C)]
#[derive(Clone)]
struct CManifest {
    /// Number of threads in this test.
    n_threads: libc::size_t,
    /// Number of atomic_ints in this test.
    n_atomic_ints: libc::size_t,
    /// Number of ints in this test.
    n_ints: libc::size_t,
    /// Initial value for each atomic_int.
    atomic_int_initials: *const libc::c_int,
    /// Initial value for each int.
    int_initials: *const libc::c_int,
    /// Name of each atomic_int.
    atomic_int_names: *const *const libc::c_char,
    /// Name of each int.
    int_names: *const *const libc::c_char,
}

impl CManifest {
    fn atomic_int_name_vec(&self) -> Vec<String> {
        unsafe { names(self.atomic_int_names, self.n_atomic_ints) }
    }

    fn atomic_int_initial_vec(&self) -> Vec<i32> {
        unsafe { initials(self.atomic_int_initials, self.n_atomic_ints) }
    }

    fn atomic_int_map(&self) -> BTreeMap<String, manifest::VarRecord<i32>> {
        lift_to_var_map(self.atomic_int_name_vec(), self.atomic_int_initial_vec())
    }

    fn int_name_vec(&self) -> Vec<String> {
        unsafe { names(self.int_names, self.n_ints) }
    }

    fn int_initial_vec(&self) -> Vec<i32> {
        unsafe { initials(self.int_initials, self.n_ints) }
    }

    fn int_map(&self) -> BTreeMap<String, manifest::VarRecord<i32>> {
        lift_to_var_map(self.int_name_vec(), self.int_initial_vec())
    }

    fn to_manifest(&self) -> err::Result<manifest::Manifest> {
        if self.n_threads == 0 {
            Err(err::Error::NotEnoughThreads)
        } else {
            Ok(manifest::Manifest {
                n_threads: self.n_threads,
                atomic_ints: self.atomic_int_map(),
                ints: self.int_map(),
            })
        }
    }
}

/// Unsafe because in general we don't know how src and n relate.
unsafe fn names(src: *const *const libc::c_char, n: libc::size_t) -> Vec<String> {
    if n == 0 {
        vec![]
    } else {
        std::slice::from_raw_parts(src, n)
            .iter()
            .map(|ptr| ffi::CStr::from_ptr(*ptr).to_string_lossy().into_owned())
            .collect()
    }
}

unsafe fn initials(src: *const libc::c_int, n: libc::size_t) -> Vec<i32> {
    if n == 0 {
        vec![]
    } else {
        std::slice::from_raw_parts(src, n).to_vec()
    }
}

fn lift_to_var_map<T>(
    names: Vec<String>,
    inits: Vec<T>,
) -> BTreeMap<String, manifest::VarRecord<T>> {
    let records = inits.into_iter().map(|x| manifest::VarRecord {
        initial_value: Some(x),
    });
    names.into_iter().zip(records).collect()
}

#[derive(SymBorApi, Clone)]
pub struct CTestApi<'a> {
    manifest: Ref<'a, CManifest>,

    test: Symbol<'a, unsafe extern "C" fn(tid: libc::size_t, env: *mut UnsafeEnv)>,
    check: Symbol<'a, unsafe extern "C" fn(env: *const UnsafeEnv) -> bool>,
}

/// Thin layer over the C environment struct.
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

    fn for_manifest(m: &manifest::Manifest) -> err::Result<Self> {
        Self::new(m.atomic_ints.len(), m.ints.len())
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

pub struct CChecker<'a>(Symbol<'a, unsafe extern "C" fn(env: *const UnsafeEnv) -> bool>);

impl<'a> obs::Checker for CChecker<'a> {
    type Env = Env;

    fn check(&self, e: &self::Env) -> bool {
        unsafe { (self.0)(e.p) }
    }
}

impl<'a> test::Entry for CTestApi<'a> {
    type Env = Env;
    type Checker = CChecker<'a>;

    fn run(&self, tid: usize, e: &mut Env) {
        unsafe { (self.test)(tid, e.p) }
    }

    fn make_manifest(&self) -> err::Result<manifest::Manifest> {
        self.manifest.to_manifest()
    }

    /// Gets a checker for this test.
    fn checker(&self) -> Self::Checker {
        CChecker(self.check)
    }
}

pub fn load_test(lib: &Library) -> err::Result<CTestApi> {
    let c = unsafe { CTestApi::load(&lib) }?;
    Ok(c)
}
