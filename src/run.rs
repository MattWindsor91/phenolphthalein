//! The high-level test runner.
//!
use crate::{env, err, fsa, manifest, obs, test};
use crossbeam::thread;
use fsa::Fsa;
use std::sync::{Arc, Mutex};

/// An control flow condition for a test run.
#[derive(Copy, Clone)]
pub enum Condition {
    /// The test should rotate or exit when the iteration count reaches this
    /// a multiple of this number.
    EveryNIterations(usize, fsa::ExitType),
}

fn exit_if(p: bool, ty: fsa::ExitType) -> Option<fsa::ExitType> {
    if p {
        Some(ty)
    } else {
        None
    }
}

impl Condition {
    /// Gets the sort of exit, if any, that should occur given this condition
    /// and the most recent observation os.
    pub fn exit_type(&self, os: &obs::Summary) -> Option<fsa::ExitType> {
        match self {
            Self::EveryNIterations(n, et) => exit_if(os.iterations % *n == 0, *et),
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
        if let Some(exit_type) = self.handle_env(o.env()) {
            o.kill(exit_type)
        } else {
            o.relinquish()
        }
    }

    fn handle_env(&self, env: &mut C::Env) -> Option<fsa::ExitType> {
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
    conds: Vec<Condition>,
    /// The observer for the test.
    observer: obs::Observer,
    /// The manifest for the test.
    manifest: manifest::Manifest,
}

impl<C: obs::Checker> SharedState<C> {
    /// Handles the environment, including observing it and resetting it.
    fn handle(&mut self, env: &mut C::Env) -> Option<fsa::ExitType> {
        let mut m = env::Manifested {
            manifest: &self.manifest,
            env,
        };
        let summary = self.observer.observe(&mut m, &self.checker);
        m.reset();
        self.exit_type(summary)
    }

    /// Checks whether the test should exit now.
    fn exit_type(&self, summary: obs::Summary) -> Option<fsa::ExitType> {
        self.conds
            .iter()
            .filter_map(|c| c.exit_type(&summary))
            .max()
    }
}

pub struct Runner {
    /// The exit conditions that should be applied to tests run by this runner.
    pub conds: Vec<Condition>,

    /// The factory function to use to construct synchronisation.
    pub sync: fsa::sync::Factory,
}

impl Runner {
    pub fn run<T: test::Entry>(&self, entry: T) -> err::Result<obs::Observer> {
        let checker = entry.checker();

        let fsa::Bundle {
            mut automata,
            manifest,
        } = fsa::Bundle::new(entry, self.sync)?;
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
            if etype == fsa::ExitType::Exit {
                break;
            }
            automata = am;
        }

        Arc::try_unwrap(shared)
            .map_err(|_| err::Error::LockReleaseFailed)
            .and_then(move |s| Ok(s.into_inner()?.observer))
    }

    fn run_rotation<T: test::Entry>(
        &self,
        shared: Arc<Mutex<SharedState<T::Checker>>>,
        automata: fsa::Set<T, T::Env>,
    ) -> err::Result<(fsa::ExitType, fsa::Set<T, T::Env>)> {
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
