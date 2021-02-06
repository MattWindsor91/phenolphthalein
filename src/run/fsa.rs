//! The main testing finite state automaton, and helper functions for it.

use super::{halt, obs, sync};
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

impl<'a, T: abs::Entry<'a>> Runnable<T, T::Env> {
    /// Runs another iteration of this FSA's thread body.
    pub fn run(mut self) -> RunOutcome<T, T::Env> {
        if let Some(halt_type) = self.halt_type() {
            return RunOutcome::Done(Done {
                tid: self.0.tid,
                halt_type,
            });
        }

        self.0.entry.run(self.0.tid, &mut self.0.env);
        if self.0.sync.run() {
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
        self.0.sync.wait();
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
        self.0.sync.obs();
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
    sync: Arc<dyn sync::Synchroniser>,

    /// Set to rotate when an observer thread has decided the test should
    /// rotate its threads, and exit when it decides the test should
    /// be stopped; once set to either, all threads will stop the test the next
    /// time they try to run the test.
    state: Arc<AtomicU8>,
}

impl<T, E> Inner<T, E> {
    fn new(tid: usize, entry: T, env: E, sync: Arc<dyn sync::Synchroniser>) -> Self {
        Inner {
            tid,
            env,
            sync,
            entry,
            state: Arc::new(AtomicU8::new(0)),
        }
    }

    fn set_state(&self, state: Option<halt::Type>) {
        self.state.store(state.map(halt::Type::to_u8).unwrap_or(0), Ordering::Release);
    }
}

impl<T: Clone, E: Clone> Inner<T, E> {
    // These aren't public because Inner isn't public.

    /// Clones an inner handle, but with the new thread ID `new_tid`.
    fn clone_with_tid(&self, new_tid: usize) -> Self {
        let mut new = self.clone();
        new.tid = new_tid;
        new
    }
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
        spawn: impl Fn(Ready<T, E>) -> err::Result<H>,
        join: fn(H) -> err::Result<Done>,
    ) -> err::Result<(halt::Type, Self)> {
        // TODO(@MattWindsor91): the observations should only be visible from the environment once we've joined these threads
        // in general, all of the thread-unsafe stuff should be hidden inside the environment
        Ok((join_all(self.spawn_all(spawn), join)?, self.reset()))
    }

    /// Permutes the thread automata inside this set.
    pub fn permute<R: rand::Rng>(&mut self, rng: &mut R) {
        let v = &mut self.vec[..];
        v.shuffle(rng);
    }

    /// Makes a ready state for every thread in this set, and uses `spawn` to
    /// spawn a threadlike object with handle type `H`.
    /// 
    /// Ensures each thread will be spawned before returning.
    fn spawn_all<H>(&self, spawn: impl Fn(Ready<T, E>) -> err::Result<H>) -> Vec<err::Result<H>> {
        self.vec.clone().into_iter().map(|i| spawn(Ready(i))).collect()
    }

    /// Prepares this set for potentially being re-run.
    fn reset(self) -> Self {
        if let Some(inner) = self.vec.first() {
            inner.set_state(None);
        }
        self
    }
}

fn join_all<H>(handles: Vec<err::Result<H>>, join: fn(H) -> err::Result<Done>) -> err::Result<halt::Type> {
    let mut halt_type = halt::Type::Exit;
    for h in handles {
        let done = join(h?)?;
        // These'll be the same, so it doesn't matter which we grab.
        halt_type = done.halt_type;
    }
    Ok(halt_type)
}



impl<'a, T: abs::Entry<'a>> Set<T, T::Env> {
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
        let mut env = T::Env::for_manifest(&manifest)?;
        Self::init_state(&mut env, &manifest);

        let nth = manifest.n_threads;
        Ok(Self::new_with_env_and_sync(nth, entry, env, sync(nth)?))
    }

    fn init_state(env: &mut T::Env, manifest: &manifest::Manifest) {
        /* There is no obligation that the above environment has the correct
        initial values. */
        obs::Manifested { env, manifest }.reset();
    }

    fn new_with_env_and_sync(
        nthreads: usize,
        entry: T,
        env: T::Env,
        sync: Arc<dyn sync::Synchroniser>,
    ) -> Self {
        let last_tid = nthreads - 1;
        let inner = Inner::new(last_tid, entry, env, sync);
        let mut automata = Set {
            vec: Vec::with_capacity(nthreads),
        };
        for tid in 0..last_tid {
            automata.vec.push(inner.clone_with_tid(tid));
        }
        automata.vec.push(inner);
        automata
    }
}
