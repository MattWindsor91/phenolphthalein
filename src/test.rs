use crate::{c, env, manifest};
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

pub fn build<'a>(entry: c::CTestApi<'a>, mf: manifest::Manifest) -> c::Result<Vec<ReadyTest<'a>>> {
    // TODO(@MattWindsor91): make it so that we wrap the manifest up with the
    // test and can't separate the two.

    let e = c::Env::new(mf.atomic_ints.len(), mf.ints.len())?;
    let b = Arc::new(Barrier::new(mf.n_threads));
    let test = Test {
        tid: mf.n_threads - 1,
        e,
        b,
        entry,
    };

    let mut v = Vec::with_capacity(mf.n_threads);
    for tid in 0..mf.n_threads - 1 {
        let mut tc = test.clone();
        tc.tid = tid;
        v.push(ReadyTest(tc));
    }
    v.push(ReadyTest(test));
    Ok(v)
}
