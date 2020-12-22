use crate::env;
use std::sync::{Arc, Barrier};

/// Hidden implementation of all the various test structs.
#[derive(Clone)]
struct Test {
    tid: usize,
    e: env::Env,
    entry: env::TestEntry<env::Env>,
    b: Arc<Barrier>,
}

/// A test that is ready to send to its thread.
pub struct ReadyTest(Test);

/// We can 'safely' send ReadyTests across thread boundaries.
///
/// Of course, the entire point of concurrency testing is to find concurrency
/// bugs, and these can often manifest as a violation of the sorts of rules
/// that implementing Send is supposed to serve as a guarantee of.
///
/// The main rationale for this being 'mostly ok' to send across thread
/// boundaries is that the test wrappers constrain the operations we can perform
/// in respect to the thread barriers.
unsafe impl Send for ReadyTest {}

/// We can 'safely' send references to Envs across thread boundaries.
///
/// See the Sync implementation for the handwave.
unsafe impl Sync for ReadyTest {}

impl ReadyTest {
    pub fn start(self) -> RunnableTest {
        RunnableTest(self.0)
    }
}

/// A test that is in the runnable position.
pub struct RunnableTest(Test);

impl RunnableTest {
    pub fn run(mut self) -> RunOutcome {
        (self.0.entry)(self.0.tid, &mut self.0.e);
        let bwr = self.0.b.wait();
        if bwr.is_leader() {
            RunOutcome::Observe(ObservableTest(self.0))
        } else {
            RunOutcome::Wait(WaitingTest(self.0))
        }
    }
}

/// A test that is in the waiting position.
pub struct WaitingTest(Test);

impl WaitingTest {
    pub fn wait(self) -> RunnableTest {
        self.0.b.wait();
        RunnableTest(self.0)
    }
}

/// A test that is in the observable position.
pub struct ObservableTest(Test);

pub enum RunOutcome {
    /// This thread should wait until it can run again.
    Wait(WaitingTest),
    /// This thread should read the current state, then wait until it can run again.
    Observe(ObservableTest),
}

impl ObservableTest {
    pub fn env(&mut self) -> &mut dyn env::AnEnv {
        &mut self.0.e
    }

    pub fn relinquish(self) -> WaitingTest {
        WaitingTest(self.0)
    }
}

pub struct TestBuilder {
    num_threads: usize,
    num_atomic_ints: usize,
    num_ints: usize,
    entry: Box<dyn Fn() -> env::Result<env::TestEntry<env::Env>>>,
}

impl TestBuilder {
    pub fn new(num_threads: usize, num_atomic_ints: usize, num_ints: usize) -> Self {
        TestBuilder {
            num_threads,
            num_atomic_ints,
            num_ints,
            entry: Box::new(env::load_test),
        }
    }

    pub fn build(&self) -> env::Result<Vec<ReadyTest>> {
        if self.num_threads == 0 {
            return Err(env::Error::NotEnoughThreads);
        }

        let e = env::Env::new(self.num_atomic_ints, self.num_ints)?;
        let entry = (self.entry)()?;
        let b = Arc::new(Barrier::new(self.num_threads));
        let test = Test {
            tid: self.num_threads - 1,
            e,
            b,
            entry,
        };

        let mut v = Vec::with_capacity(self.num_threads);
        for tid in 0..self.num_threads - 1 {
            let mut tc = test.clone();
            tc.tid = tid;
            v.push(ReadyTest(tc));
        }
        v.push(ReadyTest(test));
        Ok(v)
    }
}
