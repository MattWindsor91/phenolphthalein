use crate::{err, manifest};
use std::{collections::BTreeMap, ffi};

/// The raw manifest structure that the test implements to communicate auxiliary
/// information to the test runner.
///
/// This must line up with 'struct manifest' in phenol.h.
#[repr(C)]
#[derive(Clone)]
pub(super) struct Manifest {
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

impl Manifest {
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

    /// Tries to convert this C manifest to the standard structure.
    pub(super) fn to_manifest(&self) -> err::Result<manifest::Manifest> {
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
