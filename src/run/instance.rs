//! Test instances.
use super::{fsa, halt, permute::Permuter, shared, sync, thread::Threader};
use crate::{api::abs, err};

/// A single instance of a test, ready to be permuted and run.
///
/// An [Instance] manages multiple finite state automata (see
/// `super::fsa`), and allows controlled running of them over particular
/// `super::thread::Threader`s.
pub struct Instance<'entry, E: abs::Entry<'entry>> {
    /// A persistent copy of the ready automaton with the highest thread ID.
    ///
    /// This automaton holds references to the shared state, halt signal,
    /// and other useful things; it is also used to clone out the full set of
    /// automata when the instance is spawned.
    top: fsa::ReadyAutomaton<'entry, E>,
}

impl<'entry, E: abs::Entry<'entry>> Instance<'entry, E> {
    /// Spawns a series of threadlike objects, one for each test automaton;
    /// joins on each to retrieve evidence that the automaton is done; and
    /// returns the outcome of the run (possibly containing another instance).
    ///
    /// This method exists to allow situations where we want to re-run the FSAs
    /// of a test on multiple thread configurations, and attempts to prevent
    /// unsafe parallel usage of more FSAs at once than the test was built to
    /// handle.
    pub fn run<
        'scope,
        R: Threader<'entry, 'scope>,
        P: Permuter<fsa::ReadyAutomaton<'entry, E>> + ?Sized,
    >(
        self,
        threader: &'scope R,
        permuter: &mut P,
    ) -> err::Result<Outcome<'entry, E>> {
        let vec = self.make_vec(permuter);
        let handles = threader.spawn_all(vec)?;
        self.into_outcome(threader.join_all(handles)?.halt_type)
    }

    /// Makes a permuted vector of ready automata.
    fn make_vec<P: Permuter<fsa::ReadyAutomaton<'entry, E>> + ?Sized>(
        &self,
        permuter: &mut P,
    ) -> Vec<fsa::ReadyAutomaton<'entry, E>> {
        let mut v = unsafe { self.top.clone().replicate() };
        permuter.permute(&mut v);
        v
    }

    fn into_outcome(self, halt_type: halt::Type) -> err::Result<Outcome<'entry, E>> {
        Ok(match halt_type {
            halt::Type::Rotate => {
                // If we don't do this, then threads will spawn, immediately
                // think they need to rotate again, and fail to advance.
                self.top.halt_signal().clear();
                Outcome::Rotate(self)
            }
            halt::Type::Exit => {
                // The reference count for the tester state should be 1, as top
                // should be the only automaton left on this state.
                Outcome::Exit(self.top.into_shared_state()?)
            }
        })
    }

    /// Constructs an instance from a test entry point, synchronisation factory,
    /// and shared state.
    ///
    /// This function relies on the various inputs matching up; it
    /// presently relies on the rest of the runner infrastructure ensuring this.
    pub(super) fn new(
        entry: E,
        sync: sync::Factory,
        tester_state: shared::State<'entry, E::Env>,
    ) -> err::Result<Self> {
        let nthreads = tester_state.env.manifest.n_threads;
        let sync = sync(nthreads)?;
        Ok(Self {
            top: fsa::Automaton::new(nthreads.get() - 1, tester_state, entry, sync),
        })
    }
}

/// Enumeration of outcomes that can occur when running a set.
pub enum Outcome<'entry, E: abs::Entry<'entry>> {
    /// The test should run again with a new rotation; the set is returned to
    /// facilitate this.
    Rotate(Instance<'entry, E>),
    /// The test has exited, and the tester state passed outwards for
    /// inspection.
    Exit(shared::State<'entry, E::Env>),
}
