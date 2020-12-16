extern crate libc;
use std::collections::BTreeMap;
use std::ptr;
use std::thread;
use std::sync::{Arc, Barrier};

#[repr(C)]
pub struct UnsafeEnv {
    _private: [u8; 0],
}

extern "C" {
    pub fn alloc_env(atomic_ints: libc::size_t, ints: libc::size_t) -> *mut UnsafeEnv;
    pub fn free_env(e: *mut UnsafeEnv);
    pub fn get_atomic_int(e: *const UnsafeEnv, index: libc::size_t) -> libc::c_int;
    pub fn get_int(e: *const UnsafeEnv, index: libc::size_t) -> libc::c_int;

    pub fn test(tid: libc::size_t, e: *mut UnsafeEnv);
}

/// A thin wrapper over the C thread environment type.
pub struct Env {
    /// Tracks whether this Env is a copy of the one inside the original harness.
    copy: bool,
    p: *mut UnsafeEnv,
}

impl Env {
    pub fn new(num_atomic_ints: usize, num_ints: usize) -> Option<Self> {
        let mut e = Env {
            copy: false,
            p: ptr::null_mut(),
        };
        unsafe {
            e.p = alloc_env(num_atomic_ints, num_ints);
        }
        if e.p.is_null() {
            None
        } else {
            Some(e)
        }
    }

    /// Gets the atomic integer in slot i.
    /// Assumes that the C implementation does range checking and returns a
    /// valid but undefined result if i is out of bounds.
    pub fn atomic_int(&self, i: usize) -> i32 {
        unsafe { get_atomic_int(self.p, i) }
    }

    /// Gets the integer in slot i.
    /// Assumes that the C implementation does range checking and returns a
    /// valid but undefined result if i is out of bounds.
    pub fn int(&self, i: usize) -> i32 {
        unsafe { get_int(self.p, i) }
    }
}

/// We can 'safely' send Envs across thread boundaries.
///
/// Of course, the entire point of concurrency testing is to find concurrency
/// bugs, and these can often manifest as a violation of the sorts of rules
/// that implementing Send is supposed to serve as a guarantee of.
unsafe impl Send for Env {}

/// We can 'safely' send references to Envs across thread boundaries.
unsafe impl Sync for Env {}

impl Clone for Env {
    fn clone(&self) -> Self {
        Env {
            copy: true,
            p: self.p,
        }
    }
}

/// Envs can be dropped;
/// when the original is dropped, it releases the inner C structure.
impl Drop for Env {
    fn drop(&mut self) {
        if self.copy {
            return;
        }
        unsafe {
            free_env(self.p);
            self.p = ptr::null_mut();
        }
    }
}

struct Environment<'a> {
    atomic_ints: &'a [&'a str],
    ints: &'a [&'a str],
    env: Env,
}

impl<'a> Environment<'a> {
    pub fn new(atomic_ints: &'a [&'a str], ints: &'a [&'a str]) -> Option<Self> {
        Env::new(atomic_ints.len(), ints.len()).map(|env| Environment {
            atomic_ints,
            ints,
            env,
        })
    }

    pub fn atomic_int_values(&self) -> BTreeMap<&'a str, i32> {
        self.atomic_ints
            .iter()
            .enumerate()
            .map(|(i, x)| (*x, self.env.atomic_int(i)))
            .collect()
    }

    pub fn int_values(&self) -> BTreeMap<&'a str, i32> {
        self.ints
            .iter()
            .enumerate()
            .map(|(i, x)| (*x, self.env.int(i)))
            .collect()
    }
}

fn run_thread(tid: usize, e: &mut Env) {
    unsafe {
        test(tid, e.p);
    }
}

fn thread_body(tid: usize, mut e: Env, b: Arc<Barrier>) {
    for _i in 0..=100 {
        run_thread(tid, &mut e);
        b.wait();
    }
}

fn main() {
    let atomic_ints = vec!["x", "y"];
    let ints = vec!["0:r0", "1:r0"];
    let e = Environment::new(&atomic_ints, &ints).unwrap();

    let nthreads = 2;
    let mut handles: Vec<thread::JoinHandle<()>> = Vec::with_capacity(nthreads);
    let barrier = Arc::new(Barrier::new(nthreads + 1));

    for i in 0..nthreads {
        let builder = thread::Builder::new().name(format!("P{0}", i));
        let env = e.env.clone();
        let bar = barrier.clone();
        let t = builder.spawn(move || thread_body(i, env, bar)).unwrap();
        handles.push(t)
    }

    for _i in 0..=100 {
        barrier.wait();
        for (k, v) in e.atomic_int_values().iter() {
            println!("{0}={1}", k, v)
        }
        for (k, v) in e.int_values().iter() {
            println!("{0}={1}", k, v)
        }
    }

    for h in handles.into_iter() {
        h.join();
    }

}
