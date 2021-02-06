//! The high-level test runner.
//!
use super::{fsa, fsa::Fsa, halt, obs, shared, sync, thread};
use crate::{err, model, testapi::abs};
use std::sync::{Arc, Mutex};

/// A builder for tests.
pub struct Builder<T> {
    // TODO(@MattWindsor91): use the actual builder pattern here.
    /// The halting rules that should be applied to tests run by this runner.
    pub halt_rules: Vec<halt::Rule>,

    /// The factory function to use to construct synchronisation.
    pub sync: sync::Factory,

    /// A cloneable entry into the test.
    pub entry: T,

    /// Whether we should enable state checking.
    pub check: bool,

    /// Whether we should permute threads at each thread rotation.
    pub permute_threads: bool,
}

impl<'a, T: abs::Entry<'a>> Builder<T> {
    pub fn build(self) -> err::Result<Runner<'a, T, T::Env>> {
        let manifest = self.entry.make_manifest()?;
        let shared = self.make_shared_state(manifest.clone())?;
        let rng = rand::thread_rng();
        let automata = fsa::Set::new(self.entry.clone(), manifest, self.sync)?;

        Ok(Runner {
            automata: Some(automata),
            shared,
            permute_threads: self.permute_threads,
            rng,
        })
    }

    fn make_shared_state(
        &self,
        manifest: model::manifest::Manifest,
    ) -> err::Result<Arc<Mutex<shared::State<'a, T::Env>>>> {
        let observer = obs::Observer::new();
        let shin = shared::State {
            halt_rules: self.halt_rules.clone(),
            observer,
            checker: self.make_checker(),
            manifest,
        };
        Ok(Arc::new(Mutex::new(shin)))
    }

    fn make_checker(&self) -> Box<dyn model::check::Checker<T::Env> + 'a> {
        if self.check {
            self.entry.checker()
        } else {
            Box::new(model::check::Outcome::Unknown)
        }
    }
}

pub struct Runner<'a, T, E> {
    automata: Option<fsa::Set<T, E>>,
    shared: Arc<Mutex<shared::State<'a, E>>>,
    permute_threads: bool,
    rng: rand::prelude::ThreadRng,
}

impl<'a, T: abs::Entry<'a>> Runner<'a, T, T::Env> {
    /// Runs the Runner's test until it exits.
    pub fn run(&mut self) -> err::Result<()> {
        while let Some(mut am) = self.automata.take() {
            if self.permute_threads {
                am.permute(&mut self.rng);
            }
            let (etype, am) = self.run_rotation(self.shared.clone(), am)?;
            if etype != halt::Type::Exit {
                self.automata.replace(am);
            }
        }
        Ok(())
    }

    /// Consumes this Runner, producing a report over its runs.
    pub fn into_report(self) -> err::Result<model::obs::Report> {
        Arc::try_unwrap(self.shared)
            .map_err(|_| err::Error::LockReleaseFailed)
            .and_then(move |s| Ok(s.into_inner()?.observer.into_report()))
    }

    fn run_rotation(
        &self,
        shared: Arc<Mutex<shared::State<'a, T::Env>>>,
        automata: fsa::Set<T, T::Env>,
    ) -> err::Result<(halt::Type, fsa::Set<T, T::Env>)> {
        crossbeam::thread::scope(|s| {
            automata.run(
                |r: fsa::Ready<T, T::Env>| {
                    let builder = s.builder().name(format!("P{0}", r.tid()));
                    let thrd = thread::Thread::<'a, T::Env> {
                        shared: shared.clone(),
                    };
                    let handle = builder.spawn(move |_| thrd.run(r.start()))?;
                    Ok(handle)
                },
                |h| h.join().map_err(|_| err::Error::ThreadPanic),
            )
        })
        .map_err(|_| err::Error::ThreadPanic)?
    }
}
