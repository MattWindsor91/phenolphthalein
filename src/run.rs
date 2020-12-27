//! The high-level test runner.
//!
use crate::{err, model, testapi::abs};
use crossbeam::thread;
use std::sync::{Arc, Mutex};

mod fsa;
pub mod halt;
pub mod obs;
pub mod sync;

use fsa::Fsa;

/// A single thread controller for a test run.
pub struct Thread<C> {
    shared: Arc<Mutex<SharedState<C>>>,
}

impl<C: abs::Checker> Thread<C> {
    pub fn run<T>(&self, t: fsa::Runnable<T, C::Env>) -> fsa::Done
    where
        T: abs::Entry<Env = C::Env>,
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
        if let Some(exit_type) = self.handle_env(o.env()) {
            o.kill(exit_type)
        } else {
            o.relinquish()
        }
    }

    fn handle_env(&self, env: &mut C::Env) -> Option<halt::Type> {
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
    /// The halt conditions for the test.
    conds: Vec<halt::Condition>,
    /// The observer for the test.
    observer: obs::Observer,
    /// The manifest for the test.
    manifest: model::manifest::Manifest,
}

impl<C: abs::Checker> SharedState<C> {
    /// Handles the environment, including observing it and resetting it.
    fn handle(&mut self, env: &mut C::Env) -> Option<halt::Type> {
        let mut m = obs::Manifested {
            manifest: &self.manifest,
            env,
        };
        let summary = self.observer.observe(&mut m, &self.checker);
        m.reset();
        self.exit_type(summary)
    }

    /// Checks whether the test should exit now.
    fn exit_type(&self, summary: obs::Summary) -> Option<halt::Type> {
        self.conds
            .iter()
            .filter_map(|c| c.exit_type(&summary))
            .max()
    }
}

pub struct Runner<T> {
    /// The exit conditions that should be applied to tests run by this runner.
    pub conds: Vec<halt::Condition>,

    /// The factory function to use to construct synchronisation.
    pub sync: sync::Factory,

    /// A cloneable entry into the test.
    pub entry: T,
}

impl<'a, T: abs::Entry> Runner<T> {
    pub fn run(&self) -> err::Result<obs::Observer> {
        let checker = self.entry.checker();

        let fsa::Bundle {
            mut automata,
            manifest,
        } = fsa::Bundle::new(self.entry.clone(), self.sync)?;
        let observer = obs::Observer::new();
        let shin = SharedState {
            conds: self.conds.clone(),
            observer,
            checker,
            manifest,
        };
        let shared = Arc::new(Mutex::new(shin));

        loop {
            let (etype, am) = self.run_rotation(shared.clone(), automata)?;
            if etype == halt::Type::Exit {
                break;
            }
            automata = am;
        }

        Arc::try_unwrap(shared)
            .map_err(|_| err::Error::LockReleaseFailed)
            .and_then(move |s| Ok(s.into_inner()?.observer))
    }

    fn run_rotation(
        &self,
        shared: Arc<Mutex<SharedState<T::Checker>>>,
        automata: fsa::Set<T, T::Env>,
    ) -> err::Result<(halt::Type, fsa::Set<T, T::Env>)> {
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
        .unwrap()
    }
}
