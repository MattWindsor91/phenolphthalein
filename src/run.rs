//! The high-level test runner.
//!
use crate::{env, err, fsa, manifest, obs, test};
use crossbeam::thread;
use fsa::Fsa;
use std::sync::{Arc, Mutex};

/// An exit condition for a test run.
#[derive(Copy, Clone)]
pub enum ExitCondition {
    /// The test should exit when the iteration count reaches this number.
    ExitOnNIterations(usize),
}

impl ExitCondition {
    /// Gets whether this exit condition implies the test should exit given the
    /// observation os.
    pub fn should_exit(&self, os: &obs::Summary) -> bool {
        match self {
            Self::ExitOnNIterations(n) => *n <= os.iterations,
        }
    }
}

/// A single thread controller for a test run.
pub struct Thread<C> {
    shared: Arc<Mutex<SharedState<C>>>,
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
                fsa::RunOutcome::Observe(o) => t = self.observe(o),
            }
        }
    }

    fn observe<T>(&self, mut o: fsa::Observable<T, C::Env>) -> fsa::Runnable<T, C::Env> {
        let should_exit = self.handle_env(o.env());
        let r = if should_exit {
            o.kill()
        } else {
            o.relinquish()
        };
        r.wait()
    }

    fn handle_env(&self, env: &mut C::Env) -> bool {
        // TODO(@MattWindsor91): handle poisoning here
        let mut s = self.shared.lock().unwrap();
        s.handle(env)
    }
}

/// The shared state available to runner threads whenever they get promoted to
/// observers.
struct SharedState<C> {
    /// The state checker for the test.
    checker: C,
    /// The exit condition for the test.
    conds: ExitCondition,
    /// The observer for the test.
    observer: obs::Observer,
    /// The manifest for the test.
    manifest: manifest::Manifest,
}

impl<C: obs::Checker> SharedState<C> {
    /// Handles the environment, including observing it and resetting it.
    fn handle(&mut self, env: &mut C::Env) -> bool {
        let mut m = env::Manifested {
            manifest: &self.manifest,
            env,
        };
        let summary = self.observer.observe(&mut m, &self.checker);
        m.reset();
        self.conds.should_exit(&summary)
    }
}

pub struct Runner {
    /// The exit conditions that should be applied to tests run by this runner.
    pub conds: ExitCondition,
}

impl Runner {
    pub fn run<T: test::Entry>(&self, entry: T) -> err::Result<obs::Observer> {
        let checker = entry.checker();

        let fsa::Bundle { automata, manifest } = fsa::Bundle::new(entry)?;
        let observer = obs::Observer::new();
        let shin = SharedState {
            conds: self.conds,
            observer,
            checker,
            manifest,
        };
        let shared = Arc::new(Mutex::new(shin));

        thread::scope(|s| {
            automata.run(
                |r: fsa::Ready<T, T::Env>| {
                    let builder = s.builder().name(format!("P{0}", r.tid()));
                    let thrd = Thread::<T::Checker> {
                        shared: shared.clone(),
                    };
                    builder.spawn(move |_| thrd.run(r.start())).unwrap()
                },
                |h| {
                    let x = h.join().unwrap();
                    Ok(x)
                },
            )
        })
        .unwrap()?;

        Arc::try_unwrap(shared)
            .map_err(|_| err::Error::LockReleaseFailed)
            .and_then(move |s| Ok(s.into_inner()?.observer))
    }
}
