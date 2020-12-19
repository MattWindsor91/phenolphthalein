use std::ptr;
use std::sync::{Arc, Barrier};

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

    // for now
    pub fn test(tid: libc::size_t, e: *mut UnsafeEnv);
}

// Enumeration of errors that can happen with test creation.
#[derive(Debug)]
pub enum Error {
    EnvAllocFailed,
    NotEnoughThreads,
}
type Result<T> = std::result::Result<T, Error>;

/// Thin layer over the C environment struct, also wrapping in the test stub.
pub struct Env {
    /// The entry point into the C test.
    entry: unsafe extern "C" fn(libc::size_t, *mut UnsafeEnv),
    /// The C thread environment.
    p: *mut UnsafeEnv,
}

impl Env {
    /// Runs the entry point.
    ///
    /// This isn't exposed publicly, because the only thing that should be
    /// calling it is a RunnableTest.
    fn run(&mut self, tid: usize) {
        unsafe { (self.entry)(tid, self.p) }
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
        Env {
            entry: self.entry,
            p,
        }
    }
}

impl Env {
    pub fn new(num_atomic_ints: usize, num_ints: usize) -> Result<Self> {
        let mut e = Env {
            p: ptr::null_mut(),
            entry: test,
        };
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

pub struct TestBuilder {
    num_threads: usize,
    num_atomic_ints: usize,
    num_ints: usize,
}

impl TestBuilder {
    pub fn new(num_threads: usize, num_atomic_ints: usize, num_ints: usize) -> Self {
        TestBuilder {
            num_threads,
            num_atomic_ints,
            num_ints,
        }
    }

    pub fn build(&self) -> Result<Vec<ReadyTest>> {
        if self.num_threads == 0 {
            return Err(Error::NotEnoughThreads);
        }

        let e = Env::new(self.num_atomic_ints, self.num_ints)?;
        let b = Arc::new(Barrier::new(self.num_threads));

        let mut v = Vec::with_capacity(self.num_threads);
        for tid in 0..self.num_threads - 1 {
            v.push(ReadyTest(Test {
                tid,
                e: e.clone(),
                b: b.clone(),
            }));
        }
        v.push(ReadyTest(Test {
            tid: self.num_threads - 1,
            e,
            b,
        }));
        Ok(v)
    }
}

/// Hidden implementation of all the various test structs.
struct Test {
    tid: usize,
    e: Env,
    b: Arc<Barrier>,
}

// A test that is ready to send to its thread.
pub struct ReadyTest(Test);

/// We can 'safely' send ReadyTests across thread boundaries.
///
/// Of course, the entire point of concurrency testing is to find concurrency
/// bugs, and these can often manifest as a violation of the sorts of rules
/// that implementing Send is supposed to serve as a guarantee of.
unsafe impl Send for ReadyTest {}

/// We can 'safely' send references to Envs across thread boundaries.
unsafe impl Sync for ReadyTest {}

impl ReadyTest {
    pub fn start(self) -> RunnableTest {
        RunnableTest(self.0)
    }
}

/// A test that is in the runnable position.
///
/// Runnable tests are copiable between threads.
pub struct RunnableTest(Test);

impl RunnableTest {
    pub fn run(mut self) -> RunOutcome {
        self.0.e.run(self.0.tid);
        let bwr = self.0.b.wait();
        if bwr.is_leader() {
            RunOutcome::Observe(ObservableTest(self.0))
        } else {
            RunOutcome::Wait(WaitingTest(self.0))
        }
    }
}

pub struct WaitingTest(Test);

impl WaitingTest {
    pub fn wait(self) -> RunnableTest {
        self.0.b.wait();
        RunnableTest(self.0)
    }
}

pub struct ObservableTest(Test);

pub enum RunOutcome {
    /// This thread should wait until it can run again.
    Wait(WaitingTest),
    /// This thread should read the current state, then wait until it can run again.
    Observe(ObservableTest),
}

impl ObservableTest {
    pub fn env(&mut self) -> &mut Env {
        &mut self.0.e
    }

    pub fn relinquish(self) -> WaitingTest {
        WaitingTest(self.0)
    }
}
