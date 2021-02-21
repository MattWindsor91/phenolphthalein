//! The main testing finite state automaton, and helper functions for it.

use super::{halt, shared, sync};
use crate::{api::Entry, err};
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
pub struct Ready<'a, T: Entry<'a>>(Inner<'a, T>);

impl<'a, T: Entry<'a>> Ready<'a, T> {
    /// Consumes this `Ready` and produces a `Runnable`.
    pub fn start(self) -> Runnable<'a, T> {
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
unsafe impl<'a, T: Entry<'a>> Send for Ready<'a, T> {}

/// We can 'safely' send references to Envs across thread boundaries.
///
/// See the Sync implementation for the handwave.
unsafe impl<'a, T: Entry<'a>> Sync for Ready<'a, T> {}

impl<'a, T: Entry<'a>> Fsa for Ready<'a, T> {
    fn tid(&self) -> usize {
        self.0.tid
    }
}

/// A test handle that is in the runnable position.
pub struct Runnable<'a, T: Entry<'a>>(Inner<'a, T>);

impl<'a, T: Entry<'a>> Fsa for Runnable<'a, T> {
    fn tid(&self) -> usize {
        self.0.tid
    }
}

impl<'a, T: Entry<'a>> Runnable<'a, T> {
    /// Runs another iteration of this FSA's thread body.
    pub fn run(self) -> RunOutcome<'a, T> {
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
pub enum RunOutcome<'a, T: Entry<'a>> {
    /// The test has finished.
    Done(Done),
    /// This thread should wait until it can run again.
    Wait(Waiting<'a, T>),
    /// This thread should read the current state, then wait until it can run again.
    Observe(Observable<'a, T>),
}

/// A test handle that is in the waiting position.
pub struct Waiting<'a, T: Entry<'a>>(Inner<'a, T>);

impl<'a, T: Entry<'a>> Fsa for Waiting<'a, T> {
    fn tid(&self) -> usize {
        self.0.tid
    }
}

impl<'a, T: Entry<'a>> Waiting<'a, T> {
    pub fn wait(self) -> Runnable<'a, T> {
        self.0.sync.wait();
        Runnable(self.0)
    }
}

/// A test handle that is in the observable position.
pub struct Observable<'a, T: Entry<'a>>(Inner<'a, T>);

impl<'a, T: Entry<'a>> Fsa for Observable<'a, T> {
    fn tid(&self) -> usize {
        self.0.tid
    }
}

impl<'a, T: Entry<'a>> Observable<'a, T> {
    /// Borrows access to the shared state exposed by this `Observable`.
    pub fn shared_state(&mut self) -> &mut shared::State<'a, T::Env> {
        /* This is safe provided that the FSA's synchroniser correctly
        guarantees only one automaton can be in the Observable state
        at any given time, and remains in it for the duration of this
        mutable borrow (note that relinquishing Observable requires
        taking ownership of it). */

        unsafe { &mut *self.0.tester_state.get() }
    }

    /// Relinquishes the ability to observe the environment, and returns to a
    /// running state.
    pub fn relinquish(self) -> Runnable<'a, T> {
        self.0.sync.obs();
        Runnable(self.0)
    }

    /// Relinquishes the ability to observe the environment, marks the test as
    /// dead, and returns to a waiting state.
    pub fn kill(self, state: halt::Type) -> Runnable<'a, T> {
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
struct Inner<'a, T: Entry<'a>> {
    /// The thread ID of this automaton.
    tid: usize,

    /// Wraps shared tester state in such a way that it can become mutable when
    /// we are in the `Observing` state.
    tester_state: Arc<UnsafeCell<shared::State<'a, T::Env>>>,

    /// The test entry point, used when running the test body.
    entry: T,

    /// Points to the synchroniser used to keep automata in valid states.
    sync: Arc<dyn sync::Synchroniser>,

    /// Set to rotate when an observer thread has decided the test should
    /// rotate its threads, and exit when it decides the test should
    /// be stopped; once set to either, all threads will stop the test the next
    /// time they try to run the test.
    halt_state: Arc<AtomicU8>,
}

impl<'a, T: Entry<'a>> Inner<'a, T> {
    fn new(
        tid: usize,
        tester_state: shared::State<'a, T::Env>,
        entry: T,
        sync: Arc<dyn sync::Synchroniser>,
    ) -> Self {
        Inner {
            tid,
            sync,
            halt_state: Arc::new(AtomicU8::new(0)),
            tester_state: Arc::new(UnsafeCell::new(tester_state)),
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
    fn get_state(self) -> err::Result<shared::State<'a, T::Env>> {
        let cell = Arc::try_unwrap(self.tester_state).map_err(|_| err::Error::LockReleaseFailed)?;
        Ok(cell.into_inner())
    }

    /// Clones an inner handle, but with the new thread ID `new_tid`.
    fn clone_with_tid(&self, new_tid: usize) -> Self {
        Inner {
            tid: new_tid,
            sync: self.sync.clone(),
            halt_state: self.halt_state.clone(),
            tester_state: self.tester_state.clone(),
            entry: self.entry.clone(),
        }
    }

    /// Produces a vector of inner handles with thread IDs from 0 up to this
    /// handle's thread ID.
    fn replicate(self) -> Vec<Self> {
        let mut vec = Vec::with_capacity(self.tid + 1);
        for tid in 0..self.tid {
            vec.push(self.clone_with_tid(tid));
        }
        vec.push(self);
        vec
    }

    /// Runs the inner handle's entry with the current environment.
    ///
    /// Unsafe because there may be mutable references to the environment held
    /// by safe code (in `Observable`s), and we rely on the `Inner`'s owning
    /// state structs (eg `Runnable`) to implement the right form of
    /// synchronisation.
    unsafe fn run(&self) {
        let env = &(*self.tester_state.get()).env.env;
        self.entry.run(self.tid, env);
    }
}

/// We can't derive Clone, because it infers the wrong bound on `E`.
impl<'a, T: Entry<'a>> Clone for Inner<'a, T> {
    fn clone(&self) -> Self {
        self.clone_with_tid(self.tid)
    }
}

/// A set of test FSAs, ready to be sent to threads and run.
///
/// We can always decompose a `Set` into a single set of use-once FSAs,
/// but it is unsafe to clone the set whenever the existing set is being used,
/// and so we only provide specific support for reconstituting `Set`s at
/// the end of particular patterns of use.
pub struct Set<'a, T: Entry<'a>> {
    vec: Vec<Inner<'a, T>>,
}

impl<'a, T: Entry<'a>> Set<'a, T> {
    /// Spawns a series of threadlike objects using the FSAs in this set,
    /// joins on each to retrieve evidence that the FSA is done, and returns
    /// the outcome of the run (possibly containing another set).
    ///
    /// This method exists to allow situations where we want to re-run the FSAs
    /// of a test on multiple thread configurations, and attempts to prevent
    /// unsafe parallel usage of more FSAs at once than the test was built to
    /// handle.
    pub fn run<'scope, R: Threader<'a, 'scope>, P: Permuter<'a, T> + ?Sized>(
        self,
        threader: &'scope R,
        permuter: &mut P,
    ) -> err::Result<Outcome<'a, T>> {
        // TODO(@MattWindsor91): the observations should only be visible from the environment once we've joined these threads
        // in general, all of the thread-unsafe stuff should be hidden inside the environment
        let handles = self.spawn_all(threader, permuter);
        self.into_outcome(join_all(handles, threader)?)
    }

    /// Makes a ready state for every thread in this set, permutes them if
    /// necessary, and uses the threader to spawn a threadlike object.
    ///
    /// Ensures each thread will be spawned before returning.
    fn spawn_all<'scope, R: Threader<'a, 'scope>, P: Permuter<'a, T> + ?Sized>(
        &self,
        threader: &'scope R,
        permuter: &mut P,
    ) -> Vec<err::Result<R::Handle>> {
        let mut ready: Vec<Ready<'a, T>> = self.vec.iter().map(|i| Ready(i.clone())).collect();
        permuter.permute(&mut ready);
        ready.into_iter().map(|x| threader.spawn(x)).collect()
    }

    fn into_outcome(self, halt_type: halt::Type) -> err::Result<Outcome<'a, T>> {
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

    /// Borrows the inner state of one of the threads in this set.
    ///
    /// It is undefined as to which thread will be picked on for this borrowing,
    /// but most of the inner state is shared through `Arc`s and so this detail
    /// usually doesn't matter.
    fn inner(&self) -> err::Result<&Inner<'a, T>> {
        self.vec.first().ok_or(err::Error::NotEnoughThreads)
    }

    /// Constructs a `Set` from a test entry point and its associated manifest.
    ///
    /// This function relies on the manifest and entry point matching up; it
    /// presently relies on the rest of the runner infrastructure ensuring
    /// this.
    pub(super) fn new(
        entry: T,
        sync: sync::Factory,
        tester_state: shared::State<'a, T::Env>,
    ) -> err::Result<Self> {
        let nthreads = tester_state.env.manifest.n_threads;
        let sync = sync(nthreads)?;
        let last = Inner::new(nthreads - 1, tester_state, entry, sync);
        Ok(Set {
            vec: last.replicate(),
        })
    }
}

/// Enumeration of outcomes that can occur when running a set.
pub enum Outcome<'a, T: Entry<'a>> {
    /// The test should run again with a new rotation; the set is returned to
    /// facilitate this.
    Rotate(Set<'a, T>),
    /// The test has exited, and the tester state passed outwards for
    /// inspection.
    Exit(shared::State<'a, T::Env>),
}

/// Trait for things that can permute threads.
pub trait Permuter<'a, E: Entry<'a>> {
    /// Permutes a set of ready automata.
    ///
    /// Given that the FSA set presents each automaton to the thread runner
    /// in order, this can be used to change thread ordering or affinity.
    fn permute(&mut self, threads: &mut [Ready<'a, E>]);
}

impl<'a, T: rand::Rng + ?Sized, E: Entry<'a>> Permuter<'a, E> for T {
    fn permute(&mut self, threads: &mut [Ready<'a, E>]) {
        threads.shuffle(self)
    }
}

// A permuter that doesn't actually permute.
pub struct NopPermuter {}

impl<'a, E: Entry<'a>> Permuter<'a, E> for NopPermuter {
    fn permute(&mut self, _: &mut [Ready<'a, E>]) {}
}

/// Trait for things that can 'run' a test automaton as a thread.
pub trait Threader<'a, 'scope> {
    /// The type of thread handles.
    type Handle;

    /// Spawns a runner for a ready state, returning a handle.
    fn spawn<E: Entry<'a>>(&'scope self, state: Ready<'a, E>) -> err::Result<Self::Handle>;

    /// Joins a handle, returning the done state of the FSA.
    fn join(&'scope self, handle: Self::Handle) -> err::Result<Done>;
}

fn join_all<'a, 'scope, R: Threader<'a, 'scope>>(
    handles: Vec<err::Result<R::Handle>>,
    runner: &'scope R,
) -> err::Result<halt::Type> {
    let mut halt_type = halt::Type::Exit;
    for h in handles {
        let done = runner.join(h?)?;
        // These'll be the same, so it doesn't matter which we grab.
        halt_type = done.halt_type;
    }
    Ok(halt_type)
}
