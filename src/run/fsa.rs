//! The main testing finite state automaton, and helper functions for it.

use super::{halt, obs, sync};
use crate::{
    err,
    model::manifest,
    testapi::{abs, abs::Env},
};
use rand::seq::SliceRandom;
use std::cell::UnsafeCell;
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
pub struct Ready<S, T, E>(Inner<S, T, E>);

impl<S, T, E> Ready<S, T, E> {
    /// Consumes this `Ready` and produces a `Runnable`.
    pub fn start(self) -> Runnable<S, T, E> {
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
unsafe impl<S, T, E> Send for Ready<S, T, E> {}

/// We can 'safely' send references to Envs across thread boundaries.
///
/// See the Sync implementation for the handwave.
unsafe impl<S, T, E> Sync for Ready<S, T, E> {}

impl<S, T, E> Fsa for Ready<S, T, E> {
    fn tid(&self) -> usize {
        self.0.tid
    }
}

/// A test handle that is in the runnable position.
pub struct Runnable<S, T, E>(Inner<S, T, E>);

impl<S, T, E> Fsa for Runnable<S, T, E> {
    fn tid(&self) -> usize {
        self.0.tid
    }
}

impl<'a, S, T: abs::Entry<'a>> Runnable<S, T, T::Env> {
    /// Runs another iteration of this FSA's thread body.
    pub fn run(self) -> RunOutcome<S, T, T::Env> {
        if let Some(halt_type) = self.halt_type() {
            return RunOutcome::Done(Done {
                tid: self.0.tid,
                halt_type,
            });
        }

        unsafe { self.0.run() };
        if self.0.sync.run() {
            RunOutcome::Observe(Observable(self.0))
        } else {
            RunOutcome::Wait(Waiting(self.0))
        }
    }

    fn halt_type(&self) -> Option<halt::Type> {
        halt::Type::from_u8(self.0.halt_state.load(Ordering::Acquire))
    }
}

/// Enumeration of outcomes from running a `Runnable`.
pub enum RunOutcome<S, T, E> {
    /// The test has finished.
    Done(Done),
    /// This thread should wait until it can run again.
    Wait(Waiting<S, T, E>),
    /// This thread should read the current state, then wait until it can run again.
    Observe(Observable<S, T, E>),
}

/// A test handle that is in the waiting position.
pub struct Waiting<S, T, E>(Inner<S, T, E>);

impl<S, T, E> Fsa for Waiting<S, T, E> {
    fn tid(&self) -> usize {
        self.0.tid
    }
}

impl<S, T, E> Waiting<S, T, E> {
    pub fn wait(self) -> Runnable<S, T, E> {
        self.0.sync.wait();
        Runnable(self.0)
    }
}

/// A test handle that is in the observable position.
pub struct Observable<S, T, E>(Inner<S, T, E>);

impl<S, T, E> Fsa for Observable<S, T, E> {
    fn tid(&self) -> usize {
        self.0.tid
    }
}

impl<S, T, E> Observable<S, T, E> {
    /// Borrows access to the shared state exposed by this `Observable`.
    pub fn shared_state(&mut self) -> (&mut E, &mut S) {
        /* This is safe provided that the FSA's synchroniser correctly
        guarantees only one automaton can be in the Observable state
        at any given time, and remains in it for the duration of this
        mutable borrow (note that relinquishing Observable requires
        taking ownership of it). */

        // TODO(@MattWindsor91): E and S should be the same thing.
        (unsafe { &mut *self.0.env.get() }, unsafe {
            &mut *self.0.tester_state.get()
        })
    }

    /// Relinquishes the ability to observe the environment, and returns to a
    /// running state.
    pub fn relinquish(self) -> Runnable<S, T, E> {
        self.0.sync.obs();
        Runnable(self.0)
    }

    /// Relinquishes the ability to observe the environment, marks the test as
    /// dead, and returns to a waiting state.
    pub fn kill(self, state: halt::Type) -> Runnable<S, T, E> {
        /* TODO(@MattWindsor91): maybe return Done here, and mock up waiting
        on the final barrier, or return Waiting<Done> somehow. */
        self.0.set_halt_state(Some(state));
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

/// Hidden implementation of all the various automaton states.
struct Inner<S, T, E> {
    tid: usize,

    /// Wraps shared tester state in such a way that it can become mutable when
    /// we are in the `Observing` state.
    tester_state: Arc<UnsafeCell<S>>,

    entry: T,

    /// Handle to the state of the concurrency test itself.
    env: Arc<UnsafeCell<E>>,

    sync: Arc<dyn sync::Synchroniser>,

    /// Set to rotate when an observer thread has decided the test should
    /// rotate its threads, and exit when it decides the test should
    /// be stopped; once set to either, all threads will stop the test the next
    /// time they try to run the test.
    halt_state: Arc<AtomicU8>,
}

impl<S, T, E> Inner<S, T, E> {
    fn new(
        tid: usize,
        tester_state: S,
        entry: T,
        env: E,
        sync: Arc<dyn sync::Synchroniser>,
    ) -> Self {
        Inner {
            tid,
            sync,
            halt_state: Arc::new(AtomicU8::new(0)),
            tester_state: Arc::new(UnsafeCell::new(tester_state)),
            env: Arc::new(UnsafeCell::new(env)),
            entry,
        }
    }

    /// Atomically sets (or erases) the halt state flag.
    fn set_halt_state(&self, state: Option<halt::Type>) {
        self.halt_state
            .store(state.map(halt::Type::to_u8).unwrap_or(0), Ordering::Release);
    }

    /// Pulls the tester state out of an inner handle.
    ///
    /// This is safe, but can fail if more than one `Inner` exists at this
    /// stage.
    fn get_state(self) -> err::Result<S> {
        let cell = Arc::try_unwrap(self.tester_state).map_err(|_| err::Error::LockReleaseFailed)?;
        Ok(cell.into_inner())
    }
}

impl<S, T: Clone, E: Clone> Inner<S, T, E> {
    // These aren't public because Inner isn't public.

    /// Clones an inner handle, but with the new thread ID `new_tid`.
    fn clone_with_tid(&self, new_tid: usize) -> Self {
        Inner {
            tid: new_tid,
            sync: self.sync.clone(),
            halt_state: self.halt_state.clone(),
            tester_state: self.tester_state.clone(),
            env: self.env.clone(),
            entry: self.entry.clone(),
        }
    }
}

/// We can't derive Clone, because it infers the wrong bound on `S`.
impl<S, T: Clone, E: Clone> Clone for Inner<S, T, E> {
    fn clone(&self) -> Self {
        self.clone_with_tid(self.tid)
    }
}

impl<'a, S, T: abs::Entry<'a>> Inner<S, T, T::Env> {
    /// Runs the test's entry with the current environment.
    ///
    /// Unsafe because there may be mutable references to the environment held
    /// by safe code (in `Observable`s), and we rely on the `Inner`'s owning
    /// state structs to implement the right form of synchronisation.
    unsafe fn run(&self) {
        self.entry.run(self.tid, &*self.env.get());
    }
}

/// A set of test FSAs, ready to be sent to threads and run.
///
/// We can always decompose a `Set` into a single set of use-once FSAs,
/// but it is unsafe to clone the set whenever the existing set is being used,
/// and so we only provide specific support for reconstituting `Set`s at
/// the end of particular patterns of use.
pub struct Set<S, T, E> {
    vec: Vec<Inner<S, T, E>>,
}

impl<S, T, E> Set<S, T, E> {
    /// Borrows the inner state of one of the threads in this set.
    ///
    /// It is undefined as to which thread will be picked on for this borrowing,
    /// but most of the inner state is shared through `Arc`s and so this detail
    /// usually doesn't matter.
    fn inner(&self) -> err::Result<&Inner<S, T, E>> {
        self.vec.first().ok_or(err::Error::NotEnoughThreads)
    }
}

impl<S, T: Clone, E: Clone> Set<S, T, E> {
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
        spawn: impl Fn(Ready<S, T, E>) -> err::Result<H>,
        join: fn(H) -> err::Result<Done>,
    ) -> err::Result<Outcome<S, T, E>> {
        // TODO(@MattWindsor91): the observations should only be visible from the environment once we've joined these threads
        // in general, all of the thread-unsafe stuff should be hidden inside the environment
        let handles = self.spawn_all(spawn);
        self.into_outcome(join_all(handles, join)?)
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
    fn spawn_all<H>(
        &self,
        spawn: impl Fn(Ready<S, T, E>) -> err::Result<H>,
    ) -> Vec<err::Result<H>> {
        self.vec
            .clone()
            .into_iter()
            .map(|i| spawn(Ready(i)))
            .collect()
    }

    fn into_outcome(self, halt_type: halt::Type) -> err::Result<Outcome<S, T, E>> {
        let inner = self.inner()?;
        Ok(match halt_type {
            halt::Type::Rotate => {
                // If we don't do this, then threads will spawn, immediately
                // think they need to rotate again, and fail to advance.
                inner.set_halt_state(None);
                Outcome::Rotate(self)
            }
            halt::Type::Exit => {
                // Making sure the reference count for the tester state is 1.
                let inc = inner.clone();
                drop(self);
                Outcome::Exit(inc.get_state()?)
            }
        })
    }
}

fn join_all<H>(
    handles: Vec<err::Result<H>>,
    join: fn(H) -> err::Result<Done>,
) -> err::Result<halt::Type> {
    let mut halt_type = halt::Type::Exit;
    for h in handles {
        let done = join(h?)?;
        // These'll be the same, so it doesn't matter which we grab.
        halt_type = done.halt_type;
    }
    Ok(halt_type)
}

impl<'a, S, T: abs::Entry<'a>> Set<S, T, T::Env> {
    /// Constructs a `Set` from a test entry point and its associated manifest.
    ///
    /// This function relies on the manifest and entry point matching up; it
    /// presently relies on the rest of the runner infrastructure ensuring
    /// this.
    pub(super) fn new(
        entry: T,
        manifest: manifest::Manifest,
        sync: sync::Factory,
        tester_state: S,
    ) -> err::Result<Self> {
        let mut env = T::Env::for_manifest(&manifest)?;
        Self::init_state(&mut env, &manifest);

        let nth = manifest.n_threads;
        Ok(Self::new_with_env_and_sync(
            nth,
            entry,
            env,
            sync(nth)?,
            tester_state,
        ))
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
        tester_state: S,
    ) -> Self {
        let last_tid = nthreads - 1;
        let inner = Inner::new(last_tid, tester_state, entry, env, sync);
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

/// Enumeration of outcomes that can occur when running a set.
pub enum Outcome<S, T, E> {
    /// The test should run again with a new rotation; the set is returned to
    /// facilitate this.
    Rotate(Set<S, T, E>),
    /// The test has exited, and the tester state passed outwards for
    /// inspection.
    Exit(S),
}
