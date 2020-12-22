use crate::{c, env};
use std::sync::{Arc, Barrier};

/// Hidden implementation of all the various test structs.
#[derive(Clone)]
struct Test<'a> {
    tid: usize,
    e: c::Env,
    entry: c::CTestApi<'a>,
    b: Arc<Barrier>,
}

/// A test that is ready to send to its thread.
pub struct ReadyTest<'a>(Test<'a>);

/// We can 'safely' send ReadyTests across thread boundaries.
///
/// Of course, the entire point of concurrency testing is to find concurrency
/// bugs, and these can often manifest as a violation of the sorts of rules
/// that implementing Send is supposed to serve as a guarantee of.
///
/// The main rationale for this being 'mostly ok' to send across thread
/// boundaries is that the test wrappers constrain the operations we can perform
/// in respect to the thread barriers.
unsafe impl Send for ReadyTest<'_> {}

/// We can 'safely' send references to Envs across thread boundaries.
///
/// See the Sync implementation for the handwave.
unsafe impl Sync for ReadyTest<'_> {}

impl<'a> ReadyTest<'a> {
    pub fn start(self) -> RunnableTest<'a> {
        RunnableTest(self.0)
    }
}

/// A test that is in the runnable position.
pub struct RunnableTest<'a>(Test<'a>);

impl<'a> RunnableTest<'a> {
    pub fn run(mut self) -> RunOutcome<'a> {
        self.0.entry.run(self.0.tid, &mut self.0.e);
        let bwr = self.0.b.wait();
        if bwr.is_leader() {
            RunOutcome::Observe(ObservableTest(self.0))
        } else {
            RunOutcome::Wait(WaitingTest(self.0))
        }
    }
}

/// A test that is in the waiting position.
pub struct WaitingTest<'a>(Test<'a>);

impl<'a> WaitingTest<'a> {
    pub fn wait(self) -> RunnableTest<'a> {
        self.0.b.wait();
        RunnableTest(self.0)
    }
}

/// A test that is in the observable position.
pub struct ObservableTest<'a>(Test<'a>);

pub enum RunOutcome<'a> {
    /// This thread should wait until it can run again.
    Wait(WaitingTest<'a>),
    /// This thread should read the current state, then wait until it can run again.
    Observe(ObservableTest<'a>),
}

impl<'a> ObservableTest<'a> {
    pub fn env(&mut self) -> &mut dyn env::AnEnv {
        &mut self.0.e
    }

    pub fn relinquish(self) -> WaitingTest<'a> {
        WaitingTest(self.0)
    }
}

pub struct TestBuilder<'a> {
    num_threads: usize,
    num_atomic_ints: usize,
    num_ints: usize,
    entry: c::CTestApi<'a>
}

impl<'a> TestBuilder<'a> {
    pub fn new(entry: c::CTestApi<'a>, num_threads: usize, num_atomic_ints: usize, num_ints: usize) -> Self {
        TestBuilder {
            num_threads,
            num_atomic_ints,
            num_ints,
            entry
        }
    }

    pub fn build(self) -> c::Result<Vec<ReadyTest<'a>>> {
        if self.num_threads == 0 {
            return Err(c::Error::NotEnoughThreads);
        }

        let e = c::Env::new(self.num_atomic_ints, self.num_ints)?;
        let entry = self.entry;
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
