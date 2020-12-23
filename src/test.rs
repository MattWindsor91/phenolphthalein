use crate::{env::Env, err, manifest, obs};
use std::sync::{Arc, Barrier};

/// Trait of cloneable entry points into tests.
pub trait Entry: Clone {
    /* NOTE(@MattWindsor91): this will likely need a lifetime adding to it
    eventually; I think its lack of one thus far is a quirk of how
    dlopen Containers manage lifetimes. */

    /// Every test entry has an associated environment type, which implements
    /// a fairly basic API for inspection and resetting.
    type Env: Env;

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

/// Trait of top-level tests.
///
/// Each test can spawn multiple entry points into itself.
pub trait Test<'a> {
    /// The type of entry point into the test.
    type Entry: Entry;

    /// Spawns a new entry point into the test.
    fn spawn(&self) -> Self::Entry;
}

/// Hidden implementation of all the various test handles.
#[derive(Clone)]
struct Inner<T, E> {
    tid: usize,
    env: E,
    entry: T,
    b: Arc<Barrier>,
}

/// A test handle that is ready to send to its thread.
pub struct Ready<T, E>(Inner<T, E>);

/// We can 'safely' send ReadyTests across thread boundaries.
///
/// Of course, the entire point of concurrency testing is to find concurrency
/// bugs, and these can often manifest as a violation of the sorts of rules
/// that implementing Send is supposed to serve as a guarantee of.
///
/// The main rationale for this being 'mostly ok' to send across thread
/// boundaries is that the test wrappers constrain the operations we can perform
/// in respect to the thread barriers.
unsafe impl<T, E> Send for Ready<T, E> {}

/// We can 'safely' send references to Envs across thread boundaries.
///
/// See the Sync implementation for the handwave.
unsafe impl<T, E> Sync for Ready<T, E> {}

impl<T, E> Ready<T, E> {
    pub fn start(self) -> Runnable<T, E> {
        Runnable(self.0)
    }
}

/// A test handle that is in the runnable position.
pub struct Runnable<T, E>(Inner<T, E>);

impl<T: Entry> Runnable<T, T::Env> {
    pub fn run(mut self) -> RunOutcome<T, T::Env> {
        self.0.entry.run(self.0.tid, &mut self.0.env);
        let bwr = self.0.b.wait();
        if bwr.is_leader() {
            RunOutcome::Observe(Observable(self.0))
        } else {
            RunOutcome::Wait(Waiting(self.0))
        }
    }
}

/// A test handle that is in the waiting position.
pub struct Waiting<T, E>(Inner<T, E>);

impl<T, E> Waiting<T, E> {
    pub fn wait(self) -> Runnable<T, E> {
        self.0.b.wait();
        Runnable(self.0)
    }
}

/// A test handle that is in the observable position.
pub struct Observable<T, E>(Inner<T, E>);

pub enum RunOutcome<T, E> {
    /// This thread should wait until it can run again.
    Wait(Waiting<T, E>),
    /// This thread should read the current state, then wait until it can run again.
    Observe(Observable<T, E>),
}

impl<T, E> Observable<T, E> {
    /// Borrows access to the test's shared environment.
    pub fn env(&mut self) -> &mut E {
        &mut self.0.env
    }

    /// Relinquishes the ability to observe the environment, and returns to a
    /// waiting state.
    pub fn relinquish(self) -> Waiting<T, E> {
        Waiting(self.0)
    }
}

/// A bundle of test artefacts, ready to be run.
pub struct Bundle<T, E> {
    /// The test manifest.
    pub manifest: manifest::Manifest,

    pub handles: Vec<Ready<T, E>>,
}

pub fn build<T: Entry>(entry: T) -> err::Result<Bundle<T, T::Env>> {
    let manifest = entry.make_manifest()?;
    let env = T::Env::for_manifest(&manifest)?;
    let b = Arc::new(Barrier::new(manifest.n_threads));
    let inner = Inner {
        tid: manifest.n_threads - 1,
        env,
        b,
        entry,
    };

    let mut handles = Vec::with_capacity(manifest.n_threads);
    for tid in 0..manifest.n_threads - 1 {
        let mut tc = inner.clone();
        tc.tid = tid;
        handles.push(Ready(tc));
    }
    handles.push(Ready(inner));
    Ok(Bundle { manifest, handles })
}
