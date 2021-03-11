//! The Threader trait, crossbeam implementation, and support code.
use super::{fsa, permute::HasTid};
use crate::{api::abs, err};

/// Trait for things that can 'run' a test automaton as a thread.
///
/// This trait combines the notion of spawning a [super::fsa::ReadyAutomaton] into a
/// thread (producing a handle), and then joining it, eventually yielding a
/// a [super::fsa::Done].
pub trait Threader<'entry, 'scope> {
    /// The type of thread handles.
    type Handle;

    /// Spawns a runner for an automaton, returning a handle.
    fn spawn<E: abs::Entry<'entry>>(
        &'scope self,
        state: fsa::ReadyAutomaton<'entry, E>,
    ) -> err::Result<Self::Handle>;

    /// Spawns a runner for each ReadyAutomaton state in an iterable, returning a
    /// vector of handles.
    fn spawn_all<E: abs::Entry<'entry>>(
        &'scope self,
        automata: impl IntoIterator<Item = fsa::ReadyAutomaton<'entry, E>>,
    ) -> err::Result<Vec<Self::Handle>> {
        automata.into_iter().map(|x| self.spawn(x)).collect()
    }

    /// Joins a handle, returning the done state of the FSA.
    fn join(&'scope self, handle: Self::Handle) -> err::Result<fsa::Done>;

    /// Joins each handle in an iterable, returning an arbitrary done state.
    fn join_all(
        &'scope self,
        handles: impl IntoIterator<Item = Self::Handle>,
    ) -> err::Result<fsa::Done> {
        let mut done = None;
        for h in handles {
            // These'll be the same, so it doesn't matter which we grab.
            let _ = done.get_or_insert(self.join(h)?);
        }
        done.ok_or(err::Error::NotEnoughThreads)
    }
}

/// Implementation of thread spawning and joining for crossbeam threads.
impl<'a, 'scope> Threader<'a, 'scope> for &'scope crossbeam::thread::Scope<'a> {
    type Handle = crossbeam::thread::ScopedJoinHandle<'scope, fsa::Done>;

    fn spawn<T: abs::Entry<'a> + 'a>(
        &'scope self,
        automaton: fsa::ReadyAutomaton<'a, T>,
    ) -> err::Result<Self::Handle> {
        let builder = self.builder().name(format!("P{0}", automaton.tid()));
        Ok(builder.spawn(move |_| automaton.start().run())?)
    }

    fn join(&'scope self, handle: Self::Handle) -> err::Result<fsa::Done> {
        handle.join().map_err(|_| err::Error::ThreadPanic)
    }
}
