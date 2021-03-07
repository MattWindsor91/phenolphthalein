//! Test instances.
use super::{fsa, halt, permute::Permuter, shared, sync, thread::Threader};
use crate::{api::abs, err};

/// A single instance of a test, ready to be permuted and run.
///
/// An [Instance] manages multiple finite state automata (see
/// [super::fsa]), and allows controlled running of them over particular
/// [super::thread::Threader]s.
pub struct Instance<'a, T: abs::Entry<'a>> {
    /* TODO(@MattWindsor91):

    Ideally, we would store a vector of [fsa::Ready] here, and that would
    drop the weird dependency we have on [fsa::Inner].  This would need the
    ability to convert a vector of [fsa::Done] into a vector of [fsa::Ready],
    and every time I've tried to insert [fsa::Inner] into [fsa::Done] it's
    kicked up profound questions about how to send the result back through
    threads, how to make sure that we don't try resurrect parts of a test
    at different times, etc. */
    /// The underlying inner vector, used to create and resurrect automata.
    vec: Vec<fsa::Inner<'a, T>>,
}

impl<'a, T: abs::Entry<'a>> Instance<'a, T> {
    /// Spawns a series of threadlike objects, one for each test automaton;
    /// joins on each to retrieve evidence that the automaton is done; and
    /// returns the outcome of the run (possibly containing another instance).
    ///
    /// This method exists to allow situations where we want to re-run the FSAs
    /// of a test on multiple thread configurations, and attempts to prevent
    /// unsafe parallel usage of more FSAs at once than the test was built to
    /// handle.
    pub fn run<'scope, R: Threader<'a, 'scope>, P: Permuter<fsa::Ready<'a, T>> + ?Sized>(
        self,
        threader: &'scope R,
        permuter: &mut P,
    ) -> err::Result<Outcome<'a, T>> {
        // TODO(@MattWindsor91): the observations should only be visible from the environment once we've joined these threads
        // in general, all of the thread-unsafe stuff should be hidden inside the environment
        let handles = self.spawn_all(threader, permuter)?;
        self.into_outcome(threader.join_all(handles)?.halt_type)
    }

    /// Makes a ready state for every thread in this set, permutes them if
    /// necessary, and uses the threader to spawn a threadlike object.
    ///
    /// Ensures each thread will be spawned before returning.
    fn spawn_all<'scope, R: Threader<'a, 'scope>, P: Permuter<fsa::Ready<'a, T>> + ?Sized>(
        &self,
        threader: &'scope R,
        permuter: &mut P,
    ) -> err::Result<Vec<R::Handle>> {
        let mut ready: Vec<_> = self.vec.iter().map(|i| fsa::Ready(i.clone())).collect();
        permuter.permute(&mut ready);
        threader.spawn_all(ready)
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
    fn inner(&self) -> err::Result<&fsa::Inner<'a, T>> {
        self.vec.first().ok_or(err::Error::NotEnoughThreads)
    }

    /// Constructs an instance from a test entry point, synchronisation factory,
    /// and shared state.
    ///
    /// This function relies on the various inputs matching up; it
    /// presently relies on the rest of the runner infrastructure ensuring this.
    pub(super) fn new(
        entry: T,
        sync: sync::Factory,
        tester_state: shared::State<'a, T::Env>,
    ) -> err::Result<Self> {
        let nthreads = tester_state.env.manifest.n_threads;
        let sync = sync(nthreads)?;
        let last = fsa::Inner::new(nthreads.get() - 1, tester_state, entry, sync);
        Ok(Self {
            vec: last.replicate(),
        })
    }
}

/// Enumeration of outcomes that can occur when running a set.
pub enum Outcome<'a, T: abs::Entry<'a>> {
    /// The test should run again with a new rotation; the set is returned to
    /// facilitate this.
    Rotate(Instance<'a, T>),
    /// The test has exited, and the tester state passed outwards for
    /// inspection.
    Exit(shared::State<'a, T::Env>),
}
