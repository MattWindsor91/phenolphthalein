//! The main testing finite state automaton, and helper functions for it.

use super::{halt, sync};
use crate::{
    err,
    model::manifest,
    testapi::{abs, abs::Env},
};
use rand::seq::SliceRandom;
use std::sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
};

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

impl<T: abs::Entry> Runnable<T, T::Env> {
    /// Runs another iteration of this FSA's thread body.
    pub fn run(mut self) -> RunOutcome<T, T::Env> {
        if let Some(halt_type) = self.halt_type() {
            return RunOutcome::Done(Done {
                tid: self.0.tid,
                halt_type,
            });
        }

        self.0.entry.run(self.0.tid, &mut self.0.env);
        if self.0.b.run() {
            RunOutcome::Observe(Observable(self.0))
        } else {
            RunOutcome::Wait(Waiting(self.0))
        }
    }

    fn halt_type(&self) -> Option<halt::Type> {
        halt::Type::from_u8(self.0.state.load(Ordering::Acquire))
    }
}

/// Enumeration of outcomes from running a `Runnable`.
pub enum RunOutcome<T, E> {
    /// The test has finished.
    Done(Done),
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
    /// running state.
    pub fn relinquish(self) -> Runnable<T, E> {
        self.0.b.obs();
        Runnable(self.0)
    }

    /// Relinquishes the ability to observe the environment, marks the test as
    /// dead, and returns to a waiting state.
    pub fn kill(self, state: halt::Type) -> Runnable<T, E> {
        /* TODO(@MattWindsor91): maybe return Done here, and mock up waiting
        on the final barrier, or return Waiting<Done> somehow. */
        self.0.state.store(state.to_u8(), Ordering::Release);
        self.relinquish()
    }
}

/// A test state that represents the end of a test.
pub struct Done {
    tid: usize,

    /// The status at the end of the test.
    pub halt_type: halt::Type,
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
    b: Arc<dyn sync::Synchroniser>,

    /// Set to rotate when an observer thread has decided the test should
    /// rotate its threads, and exit when it decides the test should
    /// be stopped; once set to either, all threads will stop the test the next
    /// time they try to run the test.
    state: Arc<AtomicU8>,
}

/// A set of test FSAs, ready to be sent to threads and run.
///
/// We can always decompose a `Set` into a single set of use-once FSAs,
/// but it is unsafe to clone the set whenever the existing set is being used,
/// and so we only provide specific support for reconstituting `Set`s at
/// the end of particular patterns of use.
pub struct Set<T, E> {
    vec: Vec<Inner<T, E>>,
}

impl<T: Clone, E: Clone> Set<T, E> {
    /// Spawns a series of threadlike objects using the FSAs in this set,
    /// joins on each to retrieve evidence that the FSA is done, and returns
    /// a copy of this `Set`.
    ///
    /// This method exists to allow situations where we want to re-run the FSAs
    /// of a test on multiple thread configurations, and attempts to prevent
    /// unsafe parallel usage of more FSAs at once than the test was built to
    /// handle.
    pub fn run<H>(
        self,
        spawn: impl Fn(Ready<T, E>) -> H,
        join: fn(H) -> err::Result<Done>,
    ) -> err::Result<(halt::Type, Self)> {
        let vec = self.vec.clone();

        // Collecting to force all handles to be produced before we join any
        let handles = self.into_iter().map(spawn).collect::<Vec<H>>();

        // TODO(@MattWindsor91): the observations should only be visible from the environment once we've joined these threads
        // in general, all of the thread-unsafe stuff should be hidden inside the environment
        let mut et = halt::Type::Exit;
        for h in handles {
            let done = join(h)?;
            // These'll be the same, so it doesn't matter which we grab.
            et = done.halt_type;
        }
        Ok((et, Set { vec }))
    }

    /// Permutes the thread automata inside this set.
    pub fn permute<R: rand::Rng>(&mut self, rng: &mut R) {
        let v = &mut self.vec[..];
        v.shuffle(rng);
    }
}

impl<T: abs::Entry> Set<T, T::Env> {
    /// Constructs a `Set` from a test entry point and its associated manifest.
    ///
    /// This function relies on the manifest and entry point matching up; it
    /// presently relies on the rest of the runner infrastructure ensuring
    /// this.
    pub(super) fn new(
        entry: T,
        manifest: manifest::Manifest,
        sync: sync::Factory,
    ) -> err::Result<Self> {
        let env = T::Env::for_manifest(&manifest)?;
        let b = sync(manifest.n_threads)?;
        let inner = Inner {
            tid: manifest.n_threads - 1,
            env,
            b,
            entry,
            state: Arc::new(AtomicU8::new(0)),
        };
        let mut automata = Set {
            vec: Vec::with_capacity(manifest.n_threads),
        };
        for tid in 0..manifest.n_threads - 1 {
            let mut tc = inner.clone();
            tc.tid = tid;
            automata.vec.push(tc);
        }
        automata.vec.push(inner);
        Ok(automata)
    }
}

/// We can consume a Set into an iterator over Ready FSA handles.
impl<T, E> IntoIterator for Set<T, E> {
    type Item = Ready<T, E>;

    type IntoIter = SetIter<T, E>;

    fn into_iter(self) -> Self::IntoIter {
        SetIter(self.vec.into_iter().map(Ready))
    }
}

/// Type alias of taking `Ready` as a function.
type Readier<T, E> = fn(Inner<T, E>) -> Ready<T, E>;

/// Iterator produced by iterating on `Set`s.
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
