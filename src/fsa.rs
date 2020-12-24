use super::{env::Env, err, manifest, test};
use std::sync::{Arc, Barrier};

/// Common functionality for states in the testing finite automaton.
pub trait Fsa {
    /// Gets the ID of the test thread to which this automaton state belongs.
    fn tid(&self) -> usize;
}

/// A test handle that is ready to send to its thread.
pub struct Ready<T, E>(Inner<T, E>);

impl<T, E> Ready<T, E> {
    /// Consumes this `Ready` and produces a `Runnable`.
    pub fn start(self) -> Runnable<T, E> {
        Runnable(self.0)
    }
}

/// We can 'safely' send ReadyTests across thread boundaries.
///
/// Of course, the entire point of concurrency testing is to find concurrency
/// bugs, and these can often manifest as a violation of the sorts of rules
/// that implementing Send is supposed to guarantee.
///
/// The main rationale for this being 'mostly ok' to send across thread
/// boundaries is that the test wrappers constrain the operations we can perform
/// in respect to the thread barriers.
unsafe impl<T, E> Send for Ready<T, E> {}

/// We can 'safely' send references to Envs across thread boundaries.
///
/// See the Sync implementation for the handwave.
unsafe impl<T, E> Sync for Ready<T, E> {}

impl<T, E> Fsa for Ready<T, E> {
    fn tid(&self) -> usize {
        self.0.tid
    }
}

/// A test handle that is in the runnable position.
pub struct Runnable<T, E>(Inner<T, E>);

impl<T, E> Fsa for Runnable<T, E> {
    fn tid(&self) -> usize {
        self.0.tid
    }
}

impl<T: test::Entry> Runnable<T, T::Env> {
    /// Runs another iteration of this FSA's thread body.
    pub fn run(mut self) -> RunOutcome<T, T::Env> {
        self.0.entry.run(self.0.tid, &mut self.0.env);
        let bwr = self.0.b.wait();
        if bwr.is_leader() {
            RunOutcome::Observe(Observable(self.0))
        } else {
            RunOutcome::Wait(Waiting(self.0))
        }
    }

    /// Signals that a test runner is finished running this FSA's thread.
    pub fn end(self) -> Done {
        /* TODO(@MattWindsor91): should go through the motions of waiting on
           the barriers until some 'all threads are done' signal.
        */
        Done { tid: self.0.tid }
    }
}

pub enum RunOutcome<T, E> {
    /// This thread should wait until it can run again.
    Wait(Waiting<T, E>),
    /// This thread should read the current state, then wait until it can run again.
    Observe(Observable<T, E>),
}

/// A test handle that is in the waiting position.
pub struct Waiting<T, E>(Inner<T, E>);

impl<T, E> Fsa for Waiting<T, E> {
    fn tid(&self) -> usize {
        self.0.tid
    }
}

impl<T, E> Waiting<T, E> {
    pub fn wait(self) -> Runnable<T, E> {
        self.0.b.wait();
        Runnable(self.0)
    }
}

/// A test handle that is in the observable position.
pub struct Observable<T, E>(Inner<T, E>);

impl<T, E> Fsa for Observable<T, E> {
    fn tid(&self) -> usize {
        self.0.tid
    }
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

/// A test state that represents the end of a test.
pub struct Done {
    tid: usize,
}

impl Fsa for Done {
    fn tid(&self) -> usize {
        self.tid
    }
}

/// Hidden implementation of all the various test handles.
#[derive(Clone)]
struct Inner<T, E> {
    tid: usize,
    env: E,
    entry: T,
    b: Arc<Barrier>,
}

/// A bundle of prepared test data, ready to be run.
pub struct Bundle<T, E> {
    /// The test manifest.
    pub manifest: manifest::Manifest,

    pub handles: ReadySet<T, E>,
}

/// A set of test FSAs, ready to be sent to threads and run.
///
/// We can always decompose a `ReadySet` into a single set of use-once FSAs,
/// but it is unsafe to clone the set whenever the existing set is being used,
/// and so we only provide specific support for reconstituting `ReadySet`s at
/// the end of particular patterns of use.
pub struct ReadySet<T, E> {
    vec: Vec<Inner<T, E>>,
}

impl<T: Clone, E: Clone> ReadySet<T, E> {
    /// Spawns a series of threadlike objects using the FSAs in this set,
    /// joins on each to retrieve evidence that the FSA is done, and returns
    /// a copy of this `ReadySet`.
    ///
    /// This method exists to allow situations where we want to re-run the FSAs
    /// of a test on multiple thread configurations, and attempts to prevent
    /// unsafe parallel usage of more FSAs at once than the test was built to
    /// handle.
    pub fn run<H>(
        self,
        spawn: impl Fn(Ready<T, E>) -> H,
        join: fn(H) -> err::Result<Done>,
    ) -> err::Result<Self> {
        let vec = self.vec.clone();

        // Collecting to force all handles to be produced before we join any
        let handles = self.into_iter().map(spawn).collect::<Vec<H>>();

        // TODO(@MattWindsor91): the observations should only be visible from the environment once we've joined these threads
        // in general, all of the thread-unsafe stuff should be hidden inside the environment
        for h in handles {
            join(h)?;
        }
        Ok(ReadySet { vec })
    }
}

/// We can consume a ReadySet into an iterator over Ready FSA handles.
impl<T, E> IntoIterator for ReadySet<T, E> {
    type Item = Ready<T, E>;

    type IntoIter = SetIter<T, E>;

    fn into_iter(self) -> Self::IntoIter {
        SetIter(self.vec.into_iter().map(Ready))
    }
}

/// Type alias of taking `Ready` as a function.
type Readier<T, E> = fn(Inner<T, E>) -> Ready<T, E>;

/// Iterator produced by iterating on `ReadySet`s.
pub struct SetIter<T, E>(
    // This mainly just exists so that we don't leak `Inner`, which we would
    // do if we set this to a type alias.
    std::iter::Map<std::vec::IntoIter<Inner<T, E>>, Readier<T, E>>,
);

impl<T, E> Iterator for SetIter<T, E> {
    type Item = Ready<T, E>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

pub fn build<T: test::Entry>(entry: T) -> err::Result<Bundle<T, T::Env>> {
    let manifest = entry.make_manifest()?;
    let env = T::Env::for_manifest(&manifest)?;
    let b = Arc::new(Barrier::new(manifest.n_threads));
    let inner = Inner {
        tid: manifest.n_threads - 1,
        env,
        b,
        entry,
    };

    let mut handles = ReadySet {
        vec: Vec::with_capacity(manifest.n_threads),
    };
    for tid in 0..manifest.n_threads - 1 {
        let mut tc = inner.clone();
        tc.tid = tid;
        handles.vec.push(tc);
    }
    handles.vec.push(inner);
    Ok(Bundle { manifest, handles })
}
