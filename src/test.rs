use crate::{env, env::AnEnv, err, manifest, obs};
use std::sync::{Arc, Barrier};

/// Trait of entry points into tests.
pub trait Entry {
    /// Every test entry has an associated environment type, which implements
    /// a fairly basic API for inspection and resetting.
    type Env: env::AnEnv;

    /// Test entries must also have an associated checker type, for checking
    /// environments uphold test conditions.
    type Checker: obs::Checker<Env = Self::Env>;

    /// Makes a manifest using information taken from the test entry point.
    fn make_manifest(&self) -> err::Result<manifest::Manifest>;

    /// Runs the entry point given a thread ID and handle to the environment.
    fn run(&self, tid: usize, e: &mut Self::Env);

    /// Gets a checker for this entry point's environments.
    fn checker(&self) -> Self::Checker;
}

/// Hidden implementation of all the various test structs.
#[derive(Clone)]
struct Test<T, E> {
    tid: usize,
    env: E,
    entry: T,
    b: Arc<Barrier>,
}

/// A test that is ready to send to its thread.
pub struct ReadyTest<T, E>(Test<T, E>);

/// We can 'safely' send ReadyTests across thread boundaries.
///
/// Of course, the entire point of concurrency testing is to find concurrency
/// bugs, and these can often manifest as a violation of the sorts of rules
/// that implementing Send is supposed to serve as a guarantee of.
///
/// The main rationale for this being 'mostly ok' to send across thread
/// boundaries is that the test wrappers constrain the operations we can perform
/// in respect to the thread barriers.
unsafe impl<T, E> Send for ReadyTest<T, E> {}

/// We can 'safely' send references to Envs across thread boundaries.
///
/// See the Sync implementation for the handwave.
unsafe impl<T, E> Sync for ReadyTest<T, E> {}

impl<T, E> ReadyTest<T, E> {
    pub fn start(self) -> RunnableTest<T, E> {
        RunnableTest(self.0)
    }
}

/// A test that is in the runnable position.
pub struct RunnableTest<T, E>(Test<T, E>);

impl<T> RunnableTest<T, T::Env>
where
    T: Entry,
{
    pub fn run(mut self) -> RunOutcome<T, T::Env> {
        self.0.entry.run(self.0.tid, &mut self.0.env);
        let bwr = self.0.b.wait();
        if bwr.is_leader() {
            RunOutcome::Observe(ObservableTest(self.0))
        } else {
            RunOutcome::Wait(WaitingTest(self.0))
        }
    }
}

/// A test that is in the waiting position.
pub struct WaitingTest<T, E>(Test<T, E>);

impl<T, E> WaitingTest<T, E> {
    pub fn wait(self) -> RunnableTest<T, E> {
        self.0.b.wait();
        RunnableTest(self.0)
    }
}

/// A test that is in the observable position.
pub struct ObservableTest<T, E>(Test<T, E>);

pub enum RunOutcome<T, E> {
    /// This thread should wait until it can run again.
    Wait(WaitingTest<T, E>),
    /// This thread should read the current state, then wait until it can run again.
    Observe(ObservableTest<T, E>),
}

impl<T, E> ObservableTest<T, E> {
    pub fn env(&mut self) -> &mut E {
        &mut self.0.env
    }

    pub fn relinquish(self) -> WaitingTest<T, E> {
        WaitingTest(self.0)
    }
}

/// A bundle of test artefacts, ready to be run.
pub struct Bundle<T, E> {
    /// The test manifest.
    pub manifest: manifest::Manifest,

    pub handles: Vec<ReadyTest<T, E>>,
}

pub fn build<T>(entry: T) -> err::Result<Bundle<T, T::Env>>
where
    T: Entry,
    T: Clone,
    T::Env: Clone,
{
    let manifest = entry.make_manifest()?;
    let env = T::Env::for_manifest(&manifest)?;
    let b = Arc::new(Barrier::new(manifest.n_threads));
    let test = Test {
        tid: manifest.n_threads - 1,
        env,
        b,
        entry,
    };

    let mut handles = Vec::with_capacity(manifest.n_threads);
    for tid in 0..manifest.n_threads - 1 {
        let mut tc = test.clone();
        tc.tid = tid;
        handles.push(ReadyTest(tc));
    }
    handles.push(ReadyTest(test));
    Ok(Bundle { manifest, handles })
}
