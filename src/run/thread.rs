//! Implementation of a single thread in a test run.

use super::{fsa, halt, shared};
use crate::testapi::abs;
use std::sync::{Arc, Mutex};

/// A single thread controller for a test run.
///
/// Perhaps strangely, this is parametrised over the checker type of the test
/// API (it needs access only to the checker and its underlying environment
/// type).  This may change in future.
pub struct Thread<'a, E> {
    pub shared: Arc<Mutex<shared::State<'a, E>>>,
}

impl<'a, E: abs::Env> Thread<'a, E> {
    pub fn run<T>(&self, t: fsa::Runnable<T, E>) -> fsa::Done
    where
        T: abs::Entry<'a, Env = E>,
    {
        let mut t = t;
        loop {
            match t.run() {
                fsa::RunOutcome::Done(d) => return d,
                fsa::RunOutcome::Wait(w) => t = w.wait(),
                fsa::RunOutcome::Observe(o) => t = self.observe(o),
            }
        }
    }

    fn observe<T>(&self, mut o: fsa::Observable<T, E>) -> fsa::Runnable<T, E> {
        if let Some(exit_type) = self.handle_env(o.env()) {
            o.kill(exit_type)
        } else {
            o.relinquish()
        }
    }

    fn handle_env(&self, env: &mut E) -> Option<halt::Type> {
        // TODO(@MattWindsor91): handle poisoning here
        let mut s = self.shared.lock().unwrap();
        s.handle(env)
    }
}
