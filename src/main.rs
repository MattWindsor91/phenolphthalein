extern crate libc;
use std::collections::{BTreeMap, HashMap};
use std::ptr;
use std::sync::{Arc, Barrier};
use std::thread;

#[repr(C)]
pub struct UnsafeEnv {
    _private: [u8; 0],
}

extern "C" {
    pub fn alloc_env(atomic_ints: libc::size_t, ints: libc::size_t) -> *mut UnsafeEnv;
    pub fn free_env(e: *mut UnsafeEnv);
    pub fn get_atomic_int(e: *const UnsafeEnv, index: libc::size_t) -> libc::c_int;
    pub fn get_int(e: *const UnsafeEnv, index: libc::size_t) -> libc::c_int;
    pub fn set_atomic_int(e: *mut UnsafeEnv, index: libc::size_t, value: libc::c_int);
    pub fn set_int(e: *mut UnsafeEnv, index: libc::size_t, value: libc::c_int);

    pub fn test(tid: libc::size_t, e: *mut UnsafeEnv);
}

/// A thin wrapper over the C thread environment type.
pub struct Env {
    p: *mut UnsafeEnv,
}

impl Env {
    pub fn new(num_atomic_ints: usize, num_ints: usize) -> Option<Self> {
        let mut e = Env { p: ptr::null_mut() };
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

    pub fn set_atomic_int(&mut self, i: usize, v: i32) {
        unsafe { set_atomic_int(self.p, i, v) }
    }

    pub fn set_int(&mut self, i: usize, v: i32) {
        unsafe { set_int(self.p, i, v) }
    }

    /// Clones out a weak reference to the environment for use in a thread.
    pub fn clone(&self) -> ThreadEnv {
        ThreadEnv { p: self.p }
    }
}

/// Envs can be dropped;
/// when the original is dropped, it releases the inner C structure.
impl Drop for Env {
    fn drop(&mut self) {
        // TODO(@MattWindsor91): this isn't safe in general: the ThreadEnvs could outlast the Env.
        unsafe {
            free_env(self.p);
            self.p = ptr::null_mut();
        }
    }
}

/// A copy of the environment structure that can only be used to run threads.
pub struct ThreadEnv {
    p: *mut UnsafeEnv,
}

/// We can 'safely' send Envs across thread boundaries.
///
/// Of course, the entire point of concurrency testing is to find concurrency
/// bugs, and these can often manifest as a violation of the sorts of rules
/// that implementing Send is supposed to serve as a guarantee of.
unsafe impl Send for ThreadEnv {}

/// We can 'safely' send references to Envs across thread boundaries.
unsafe impl Sync for ThreadEnv {}

pub enum TestOutcome {
    Next(ThreadEnv),
}

pub struct Test {
    tid: usize,
    runner: Box<dyn Fn(usize, *mut UnsafeEnv) -> ()>,
    barrier: Arc<Barrier>,
}

impl Test {
    pub fn run(&self, t: ThreadEnv) -> TestOutcome {
        (self.runner)(self.tid, t.p);
        self.barrier.wait();
        // TODO(@MattWindsor91): cancellation
        // TODO(@MattWindsor91): really the leader should be writing back?
        self.barrier.wait();
        TestOutcome::Next(t)
    }
}

type State<'a> = BTreeMap<&'a str, i32>;

struct Environment<'a> {
    atomic_ints: BTreeMap<&'a str, VarRecord<i32>>,
    ints: BTreeMap<&'a str, VarRecord<i32>>,

    pub obs: HashMap<State<'a>, usize>,

    /// The main handle to the shared-memory environment that this test is presenting to threads.
    env: Env,
}

impl<'a> Environment<'a> {
    pub fn new(
        atomic_ints: BTreeMap<&'a str, VarRecord<i32>>,
        ints: BTreeMap<&'a str, VarRecord<i32>>,
    ) -> Option<Self> {
        Env::new(atomic_ints.len(), ints.len()).map(|env| Environment {
            atomic_ints,
            ints,
            env,
            obs: HashMap::new(),
        })
    }

    fn main_loop(&mut self, barrier: Arc<Barrier>) {
        // TODO(@MattWindsor91): move the barrier into Environment
        for _i in 0..=100 {
            // TODO(@MattWindsor91): handle partial computation
            barrier.wait();
            self.log_obs();
            self.reset();
            barrier.wait();
        }
    }

    fn log_obs(&mut self) {
        let state = self.current_state();
        let inc = self.obs.get(&state).map_or(0, |k| k + 1);
        self.obs.insert(state, inc);
    }

    /// Gets the current state of the environment.
    /// Note that this is not thread-safe until all test threads are synchronised.
    fn current_state(&self) -> State<'a> {
        // TODO(@MattWindsor91): work out a good state-machine-ish approach for
        // ensuring this can only be called when threads are quiescent.
        let mut s = State::new();
        // TODO(@MattWindsor91): have one great big iterator for values and collect it.
        s.extend(self.atomic_int_values().iter());
        s.extend(self.int_values().iter());
        s
    }

    fn reset(&mut self) {
        for (i, (_, r)) in self.atomic_ints.iter().enumerate() {
            self.env.set_atomic_int(i, r.initial_value.unwrap_or(0))
        }
        for (i, (_, r)) in self.ints.iter().enumerate() {
            self.env.set_int(i, r.initial_value.unwrap_or(0))
        }
    }

    fn atomic_int_values(&self) -> BTreeMap<&'a str, i32> {
        self.atomic_ints
            .iter()
            .enumerate()
            .map(|(i, (n, _))| (*n, self.env.atomic_int(i)))
            .collect()
    }

    fn int_values(&self) -> BTreeMap<&'a str, i32> {
        self.ints
            .iter()
            .enumerate()
            .map(|(i, (n, _))| (*n, self.env.int(i)))
            .collect()
    }
}

fn thread_body(tid: usize, mut e: ThreadEnv, barrier: Arc<Barrier>) {
    let ts = Test {
        tid,
        runner: Box::new(|i, x| unsafe { test(i, x) }),
        barrier,
    };
    for _i in 0..=100 {
        match ts.run(e) {
            TestOutcome::Next(e2) => e = e2,
        }
    }
}

struct VarRecord<T> {
    initial_value: Option<T>, // Space for rent
}

fn main() {
    let mut atomic_ints = BTreeMap::new();
    atomic_ints.insert(
        "x",
        VarRecord {
            initial_value: Some(0),
        },
    );
    atomic_ints.insert(
        "y",
        VarRecord {
            initial_value: Some(0),
        },
    );

    let mut ints = BTreeMap::new();
    ints.insert(
        "0:r0",
        VarRecord {
            initial_value: Some(0),
        },
    );
    ints.insert(
        "1:r0",
        VarRecord {
            initial_value: Some(0),
        },
    );

    let mut e = Environment::new(atomic_ints, ints).unwrap();

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

    e.main_loop(barrier);

    // TODO(@MattWindsor91): the observations should only be visible from the environment once we've joined these threads
    // in general, all of the thread-unsafe stuff should be hidden inside the environment
    for h in handles.into_iter() {
        h.join().unwrap();
    }

    for (k, v) in e.obs {
        println!("{0:?}: {1}", k, v);
    }
}
