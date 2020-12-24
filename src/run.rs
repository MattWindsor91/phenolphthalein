//! The high-level test runner.

use crate::{obs, fsa, test};
use std::sync::{Arc, Mutex};

/// An exit condition for a test run.
pub enum ExitCondition {
    /// The test should exit when the iteration count reaches this number.
    ExitOnNIterations(usize)
}

impl ExitCondition {
    /// Gets whether this exit condition implies the test should exit given the
    /// observation os.
    pub fn should_exit(&self, os: &obs::Summary) -> bool {
        match self {
            Self::ExitOnNIterations(n) => *n <= os.iterations
        }
    }
}

/// A single thread controller for a test run.
pub struct Thread<C> {
    pub shared: Arc<Mutex<SharedState<C>>>,
}

impl<C: obs::Checker> Thread<C> {
    pub fn run<T>(&self, t: fsa::Runnable<T, C::Env>) -> fsa::Done
    where
        T: test::Entry<Env = C::Env>,
    {
        let mut t = t;
        loop {
            match t.run() {
                fsa::RunOutcome::Done(d) => return d,
                fsa::RunOutcome::Wait(w) => t = w.wait(),
                fsa::RunOutcome::Observe(mut o) => {
                    let should_exit = self.handle_env(o.env());
                    let r = if should_exit {
                        o.kill()
                    } else {
                        o.relinquish()
                    };
                    t = r.wait()
                }
            }
        }
    }

    fn handle_env(&self, env: &mut C::Env) -> bool {
        // TODO(@MattWindsor91): handle poisoning here
        let mut s = self.shared.lock().unwrap();
        s.handle(env)
    }
}

/// The shared state available to runner threads whenever they get promoted to
/// observers.
pub struct SharedState<C> {
    /// The state checker for the test.
    pub checker: C,
    /// The exit condition for the test.
    pub conds: ExitCondition,
    /// The observer for the test.
    pub observer: obs::Observer,
}

impl<C: obs::Checker> SharedState<C> {
    fn handle(&mut self, env: &mut C::Env) -> bool {
        let summary = self.observer.observe_and_reset(env, &self.checker);
        self.conds.should_exit(&summary)
    }
}