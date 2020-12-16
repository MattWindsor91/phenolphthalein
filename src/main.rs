extern crate libc;
use std::collections::BTreeMap;
use std::ptr;

#[repr(C)]
pub struct Env {
    _private: [u8; 0],
}
extern "C" {
    pub fn alloc_env(atomic_ints: libc::size_t, ints: libc::size_t) -> *mut Env;
    pub fn free_env(e: *mut Env);
    pub fn get_atomic_int(e: *const Env, index: libc::size_t) -> libc::c_int;
    pub fn get_int(e: *const Env, index: libc::size_t) -> libc::c_int;

    pub fn test(tid: libc::c_int, e: *mut Env);
}

struct Environment<'a> {
    atomic_ints: &'a [&'a str],
    ints: &'a [&'a str],
    env: *mut Env,
}

impl<'a> Environment<'a> {
    pub fn new(atomic_ints: &'a [&'a str], ints: &'a [&'a str]) -> Option<Self> {
        let mut e = Environment {
            atomic_ints,
            ints,
            env: ptr::null_mut(),
        };
        unsafe {
            e.env = alloc_env(e.atomic_ints.len(), e.ints.len());
        }
        if e.env.is_null() {
            None
        } else {
            Some(e)
        }
    }

    pub fn atomic_int_values(&self) -> BTreeMap<&'a str, i32> {
        self.atomic_ints
            .iter()
            .enumerate()
            .map(|(i, x)| unsafe { (*x, get_atomic_int(self.env, i)) })
            .collect()
    }

    pub fn int_values(&self) -> BTreeMap<&'a str, i32> {
        self.ints
            .iter()
            .enumerate()
            .map(|(i, x)| unsafe { (*x, get_atomic_int(self.env, i)) })
            .collect()
    }
}

impl Drop for Environment<'_> {
    fn drop(&mut self) {
        unsafe {
            free_env(self.env);
            self.env = ptr::null_mut();
        }
    }
}

fn run_thread(tid: i32, e: &mut Environment) {
    unsafe {
        test(tid, e.env);
    }
}

fn main() {
    let atomic_ints = vec!["x", "y"];
    let ints = vec!["0:r0", "1:r0"];

    let mut e = Environment::new(&atomic_ints, &ints).unwrap();

    run_thread(0, &mut e);
    for (k, v) in e.atomic_int_values().iter() {
        println!("{0}={1}", k, v)
    }
    for (k, v) in e.int_values().iter() {
        println!("{0}={1}", k, v)
    }

    run_thread(1, &mut e);
    for (k, v) in e.atomic_int_values().iter() {
        println!("{0}={1}", k, v)
    }
    for (k, v) in e.int_values().iter() {
        println!("{0}={1}", k, v)
    }
}
