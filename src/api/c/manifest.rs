use crate::{
    err,
    model::{manifest, slot},
};
use std::{convert::TryFrom, ffi, num::NonZeroUsize};

/// The raw manifest structure that the test implements to communicate auxiliary
/// information to the test runner.
///
/// This must line up with 'struct manifest' in phenol.h.
#[repr(C)]
#[derive(Clone)]
pub(super) struct Manifest {
    /// Number of threads in this test.
    n_threads: libc::size_t,
    /// Number of atomic 32-bit integers in this test.
    n_atomic_i32: libc::size_t,
    /// Initial value for each atomic_int.
    atomic_i32_initials: *const i32,
    /// Name of each atomic_int.
    atomic_i32_names: *const *const libc::c_char,
    /// Number of 32-bit integers in this test.
    n_i32: libc::size_t,
    /// Initial value for each int.
    i32_initials: *const i32,
    /// Name of each int.
    i32_names: *const *const libc::c_char,
}

impl Manifest {
    fn atomic_i32_name_vec(&self) -> Vec<String> {
        unsafe { names(self.atomic_i32_names, self.n_atomic_i32) }
    }

    fn atomic_i32_initial_vec(&self) -> Vec<i32> {
        unsafe { initials(self.atomic_i32_initials, self.n_atomic_i32) }
    }

    fn i32_name_vec(&self) -> Vec<String> {
        unsafe { names(self.i32_names, self.n_i32) }
    }

    fn i32_initial_vec(&self) -> Vec<i32> {
        unsafe { initials(self.i32_initials, self.n_i32) }
    }

    fn i32_map(&self) -> manifest::VarMap<i32> {
        let mut map = lift_to_var_map(self.i32_name_vec(), self.i32_initial_vec(), false);
        map.extend(lift_to_var_map(
            self.atomic_i32_name_vec(),
            self.atomic_i32_initial_vec(),
            true,
        ));
        map
    }

    /// Tries to convert this C manifest to the standard structure.
    pub(super) fn to_manifest(&self) -> err::Result<manifest::Manifest> {
        let n_threads =
            NonZeroUsize::try_from(self.n_threads).map_err(|_| err::Error::NotEnoughThreads)?;
        Ok(manifest::Manifest {
            n_threads,
            i32s: self.i32_map(),
        })
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

fn lift_to_var_map<T>(names: Vec<String>, inits: Vec<T>, is_atomic: bool) -> manifest::VarMap<T> {
    let records = inits
        .into_iter()
        .enumerate()
        .map(|(index, x)| manifest::VarRecord {
            initial_value: Some(x),
            slot: slot::Slot { is_atomic, index },
        });
    names.into_iter().zip(records).collect()
}
