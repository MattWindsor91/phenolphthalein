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

/// The initial state of a thread's finite state automaton.
///
/// This is separate from [Running] as it unambiguously identifies a thread that
/// has not yet started.  As such, it can be sent across threads, whereas a
/// [Running] thread cannot.
pub struct Ready<'a, T: Entry<'a>>(pub(super) Inner<'a, T>);

impl<'a, T: Entry<'a>> Ready<'a, T> {
    /// Consumes this [Ready] and produces a [Running].
    pub fn start(self) -> Running<'a, T> {
        Running(self.0)
    }
}

/// We can 'safely' send Ready states across thread boundaries.
///
/// Of course, the entire point of concurrency testing is to find concurrency
/// bugs, and these can often manifest as a violation of the sorts of rules
/// that implementing Send is supposed to guarantee.
///
/// The main rationale for this being 'mostly ok' to send across thread
/// boundaries is that the test wrappers constrain the operations we can perform
/// in respect to the thread barriers.
unsafe impl<'a, T: Entry<'a>> Send for Ready<'a, T> {}

/// We can 'safely' send Ready states across thread boundaries.
///
/// See the Sync implementation for the handwave.
unsafe impl<'a, T: Entry<'a>> Sync for Ready<'a, T> {}

impl<'a, T: Entry<'a>> HasTid for Ready<'a, T> {
    fn tid(&self) -> usize {
        self.0.tid
    }
}

/// A test handle that is in the Running position.
pub struct Running<'a, T: Entry<'a>>(Inner<'a, T>);

impl<'a, T: Entry<'a>> HasTid for Running<'a, T> {
    fn tid(&self) -> usize {
        self.0.tid
    }
}

impl<'a, T: Entry<'a>> Running<'a, T> {
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
    pub fn step(self) -> RunOutcome<'a, T> {
        if let Some(halt_type) = self.halt_type() {
            return RunOutcome::Done(Done {
                tid: self.0.tid,
                halt_type,
            });
        }

        unsafe { self.0.run() };
        if self.0.sync.run() {
            RunOutcome::Observe(Observing(self.0))
        } else {
            RunOutcome::Wait(Waiting(self.0))
        }
    }

    fn halt_type(&self) -> Option<halt::Type> {
        halt::Type::from_u8(self.0.halt_state.load(Ordering::Acquire))
    }
}

/// Enumeration of outcomes from running a `Running`.
pub enum RunOutcome<'a, T: Entry<'a>> {
    /// The test has finished.
    Done(Done),
    /// This thread should wait until it can run again.
    Wait(Waiting<'a, T>),
    /// This thread should read the current state, then wait until it can run again.
    Observe(Observing<'a, T>),
}

/// A test handle that is in the waiting position.
pub struct Waiting<'a, T: Entry<'a>>(Inner<'a, T>);

impl<'a, T: Entry<'a>> HasTid for Waiting<'a, T> {
    fn tid(&self) -> usize {
        self.0.tid
    }
}

impl<'a, T: Entry<'a>> Waiting<'a, T> {
    pub fn wait(self) -> Running<'a, T> {
        self.0.sync.wait();
        Running(self.0)
    }
}

/// A state that can observe the current tester shared state.
pub struct Observing<'a, T: Entry<'a>>(Inner<'a, T>);

impl<'a, T: Entry<'a>> HasTid for Observing<'a, T> {
    fn tid(&self) -> usize {
        self.0.tid
    }
}

impl<'a, T: Entry<'a>> Observing<'a, T> {
    /// Observes the shared state, returning back to a Running state.
    pub fn observe(mut self) -> Running<'a, T> {
        // We can't map_or_else here, because both legs move self.
        if let Some(kill_type) = self.shared_state().observe() {
            self.kill(kill_type)
        } else {
            self.relinquish()
        }
    }

    /// Borrows access to the shared state exposed by this `Observing`.
    pub fn shared_state(&mut self) -> &mut shared::State<'a, T::Env> {
        /* This is safe provided that the FSA's synchroniser correctly
        guarantees only one automaton can be in the Observing state
        at any given time, and remains in it for the duration of this
        mutable borrow (note that relinquishing Observing requires
        taking ownership of it). */

        unsafe { &mut *self.0.tester_state.get() }
    }

    /// Relinquishes the ability to observe the environment, and returns to a
    /// running state.
    pub fn relinquish(self) -> Running<'a, T> {
        self.0.sync.obs();
        Running(self.0)
    }

    /// Relinquishes the ability to observe the environment, marks the test as
    /// dead, and returns to a waiting state.
    pub fn kill(self, state: halt::Type) -> Running<'a, T> {
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

impl HasTid for Done {
    fn tid(&self) -> usize {
        self.tid
    }
}

/// Hidden implementation of all the various automaton states.
///
/// The implementation of [super::instance] depends on this implementation at the
/// moment, but this may change.
pub(super) struct Inner<'a, T: Entry<'a>> {
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
    /// Constructs the inner state for an automaton.
    pub(super) fn new(
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
    pub fn set_halt_state(&self, state: Option<halt::Type>) {
        self.halt_state
            .store(state.map(halt::Type::to_u8).unwrap_or(0), Ordering::Release);
    }

    /// Pulls the tester state out of an inner handle.
    ///
    /// This is safe, but can fail if more than one `Inner` exists at this
    /// stage.
    pub fn get_state(self) -> err::Result<shared::State<'a, T::Env>> {
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
    pub(super) fn replicate(self) -> Vec<Self> {
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
    /// by safe code (in [Observing]s), and we rely on the [Inner]'s owning
    /// state structs (eg [Running]) to implement the right form of
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
