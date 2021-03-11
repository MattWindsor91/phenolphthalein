//! The main testing finite state automaton, and helper functions for it.

use super::{halt, permute::HasTid, shared, sync};
use crate::{api::abs::Entry, err};
use std::{
    cell::UnsafeCell,
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    },
};

/// An automaton, parametrised over its current state's phantom type.
pub struct Automaton<'a, S: State, T: Entry<'a>> {
    /// The specific state type.
    ///
    /// This cannot safely be changed in general, for reasons that should be
    /// obvious.
    state: std::marker::PhantomData<S>,

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

/// Automata always have a thread ID associated.
impl<'a, S: State, E: Entry<'a>> HasTid for Automaton<'a, S, E> {
    fn tid(&self) -> usize {
        self.tid
    }
}

impl<'a, S: State, E: Entry<'a>> Automaton<'a, S, E> {
    /// Atomically sets (or erases) the halt state flag.
    pub fn set_halt_state(&self, state: Option<halt::Type>) {
        self.halt_state
            .store(state.map(halt::Type::to_u8).unwrap_or(0), Ordering::Release);
    }

    /// Pulls the tester state out of an inner handle.
    ///
    /// This is safe, but can fail if more than one `Inner` exists at this
    /// stage.
    pub fn into_shared_state(self) -> err::Result<shared::State<'a, E::Env>> {
        let cell = Arc::try_unwrap(self.tester_state).map_err(|_| err::Error::LockReleaseFailed)?;
        Ok(cell.into_inner())
    }

    unsafe fn change_state<T: State>(self) -> Automaton<'a, T, E> {
        Automaton {
            state: std::marker::PhantomData::default(),
            tid: self.tid,
            tester_state: self.tester_state,
            entry: self.entry,
            sync: self.sync,
            halt_state: self.halt_state,
        }
    }
}

/// Marker trait for automaton states.
pub trait State {}

/// The initial state of a thread's finite state automaton.
///
/// This is separate from [Running] as it unambiguously identifies a thread that
/// has not yet started.  As such, it can be sent across threads, whereas a
/// [Running] thread cannot.
pub struct Ready;

/// [Ready] is a valid state.
impl State for Ready {}

/// Type alias for ready automata, which are common elsewhere.
pub type ReadyAutomaton<'a, E> = Automaton<'a, Ready, E>;

/// We can 'safely' send Ready states across thread boundaries.
///
/// Of course, the entire point of concurrency testing is to find concurrency
/// bugs, and these can often manifest as a violation of the sorts of rules
/// that implementing Send is supposed to guarantee.
///
/// The main rationale for this being 'mostly ok' to send across thread
/// boundaries is that the test wrappers constrain the operations we can perform
/// in respect to the thread barriers.
unsafe impl<'a, E: Entry<'a>> Send for Automaton<'a, Ready, E> {}

/// We can 'safely' send Ready states across thread boundaries.
///
/// See the Sync implementation for the handwave.
unsafe impl<'a, E: Entry<'a>> Sync for Automaton<'a, Ready, E> {}

impl<'a, E: Entry<'a>> Automaton<'a, Ready, E> {
    /// Constructs an automaton.
    ///
    /// This takes full ownership of the shared tester state, but only
    /// constructs one automaton.  It is not safe in general to have multiple
    /// automata over the same shared state, as they may accidentally alias the
    /// same thread ID.
    ///
    /// To get multiple thread automata (which is what you'll want in most
    /// cases that aren't unit tests), use the unsafe [clone], [clone_with_tid],
    /// and [replicate] functions, or the safe [super::instance::Instance]
    /// wrapper.
    pub fn new(
        tid: usize,
        tester_state: shared::State<'a, E::Env>,
        entry: E,
        sync: Arc<dyn sync::Synchroniser>,
    ) -> Self {
        Automaton {
            state: std::marker::PhantomData::default(),
            tid,
            sync,
            halt_state: Arc::new(AtomicU8::new(0)),
            tester_state: Arc::new(UnsafeCell::new(tester_state)),
            entry,
        }
    }

    /// Produces a vector of automata with thread IDs from 0 up to this
    /// automaton's thread ID.
    ///
    /// This is unsafe because it is not sound to hold multiple
    /// automata with the same thread ID, and the automata don't check to
    /// enforce this.
    pub unsafe fn replicate(self) -> Vec<Self> {
        let mut vec = Vec::with_capacity(self.tid + 1);
        for tid in 0..self.tid {
            vec.push(self.clone_with_tid(tid));
        }
        vec.push(self);
        vec
    }

    /// Clones an automaton, but with the new thread ID `new_tid`.
    ///
    /// This is unsafe because it does not check that the new thread ID is
    /// already in use.
    pub unsafe fn clone_with_tid(&self, new_tid: usize) -> Self {
        Self {
            state: self.state,
            tid: new_tid,
            sync: self.sync.clone(),
            halt_state: self.halt_state.clone(),
            tester_state: self.tester_state.clone(),
            entry: self.entry.clone(),
        }
    }

    /// Clones an automaton.
    ///
    /// This is unsafe because, in general, having two automata with the same
    /// thread ID is unsafe.
    pub unsafe fn clone(&self) -> Self {
        self.clone_with_tid(self.tid)
    }

    /// Consumes this [Ready] state and produces a [Running] state.
    pub fn start(self) -> Automaton<'a, Running, E> {
        unsafe { self.change_state() }
    }
}
/// The running state of the automaton.
pub struct Running;

/// [Running] is a valid state.
impl State for Running {}

impl<'a, E: Entry<'a>> Automaton<'a, Running, E> {
    /// Runs this automaton to completion.
    pub fn run(mut self) -> Done {
        loop {
            match self.step() {
                RunOutcome::Done(d) => break d,
                RunOutcome::Wait(w) => self = w.wait(),
                RunOutcome::Observe(o) => self = o.observe(),
            }
        }
    }

    /// Runs a single iteration of this automaton.
    pub fn step(self) -> RunOutcome<'a, E> {
        if let Some(halt_type) = self.halt_type() {
            return RunOutcome::Done(Done {
                tid: self.tid,
                halt_type,
            });
        }

        unsafe { self.run_entry() };
        match self.sync.run() {
            sync::Role::Observer => RunOutcome::Observe(unsafe { self.change_state() }),
            sync::Role::Waiter => RunOutcome::Wait(unsafe { self.change_state() }),
        }
    }

    fn halt_type(&self) -> Option<halt::Type> {
        halt::Type::from_u8(self.halt_state.load(Ordering::Acquire))
    }

    /// Runs the entry with the current environment.
    ///
    /// Unsafe because there may be mutable references to the environment held
    /// by safe code (in [Observing]s), and we rely on the [Inner]'s owning
    /// state structs (eg [Running]) to implement the right form of
    /// synchronisation.
    unsafe fn run_entry(&self) {
        let env = &(*self.tester_state.get()).env.env;
        self.entry.run(self.tid, env);
    }
}

/// Enumeration of outcomes from running a `Running`.
pub enum RunOutcome<'a, T: Entry<'a>> {
    /// The test has finished.
    Done(Done),
    /// This thread should wait until it can run again.
    Wait(Automaton<'a, Waiting, T>),
    /// This thread should read the current state, then wait until it can run again.
    Observe(Automaton<'a, Observing, T>),
}

/// The state where the automaton is waiting for an [Observing] automaton to
/// finish.
pub struct Waiting;

/// [Waiting] is a valid state.
impl State for Waiting {}

impl<'a, E: Entry<'a>> Automaton<'a, Waiting, E> {
    /// Waits for the observing thread's automaton to move to the [Running]
    /// state, then also moves to the [Running] state.
    pub fn wait(self) -> Automaton<'a, Running, E> {
        self.sync.wait();
        unsafe { self.change_state() }
    }
}

/// The state that can observe the current tester shared state.
pub struct Observing;

/// [Observing] is a valid state.
impl State for Observing {}

impl<'a, E: Entry<'a>> Automaton<'a, Observing, E> {
    /// Observes the shared state, returning back to a Running state.
    pub fn observe(mut self) -> Automaton<'a, Running, E> {
        // We can't map_or_else here, because both legs move self.
        if let Some(kill_type) = self.shared_state().observe() {
            self.kill(kill_type)
        } else {
            self.relinquish()
        }
    }

    /// Borrows access to the shared state exposed by this `Observing`.
    pub fn shared_state(&mut self) -> &mut shared::State<'a, E::Env> {
        /* This is safe provided that the FSA's synchroniser correctly
        guarantees only one automaton can be in the Observing state
        at any given time, and remains in it for the duration of this
        mutable borrow (note that relinquishing Observing requires
        taking ownership of it). */

        unsafe { &mut *self.tester_state.get() }
    }

    /// Relinquishes the ability to observe the environment, and returns to a
    /// running state.
    pub fn relinquish(self) -> Automaton<'a, Running, E> {
        self.sync.obs();
        unsafe { self.change_state() }
    }

    /// Relinquishes the ability to observe the environment, marks the test as
    /// dead, and returns to a waiting state.
    pub fn kill(self, state: halt::Type) -> Automaton<'a, Running, E> {
        /* TODO(@MattWindsor91): maybe return Done here, and mock up waiting
        on the final barrier, or return Waiting<Done> somehow. */
        self.set_halt_state(Some(state));
        self.relinquish()
    }
}

/// A test state that represents the end of a test.
pub struct Done {
    tid: usize,

    /// The status at the end of the test.
    pub halt_type: halt::Type,
}

impl HasTid for Done {
    fn tid(&self) -> usize {
        self.tid
    }
}
