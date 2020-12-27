//! The high-level test runner.
//!
use crate::{err, model, testapi::abs};
use std::sync::{Arc, Mutex};

mod fsa;
pub mod halt;
pub mod obs;
mod shared;
pub mod sync;
mod thread;

use fsa::Fsa;

pub struct Runner<T> {
    /// The exit conditions that should be applied to tests run by this runner.
    pub conds: Vec<halt::Condition>,

    /// The factory function to use to construct synchronisation.
    pub sync: sync::Factory,

    /// A cloneable entry into the test.
    pub entry: T,

    /// Whether we should permute threads at each thread rotation.
    pub permute_threads: bool,
}

impl<'a, T: abs::Entry> Runner<T> {
    pub fn run(&self) -> err::Result<obs::Observer> {
        let manifest = self.entry.make_manifest()?;
        let shared = self.make_shared_state(manifest.clone())?;
        let mut rng = rand::thread_rng();

        let mut automata = fsa::Set::new(self.entry.clone(), manifest, self.sync)?;
        loop {
            if self.permute_threads {
                automata.permute(&mut rng);
            }
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

    fn make_shared_state(
        &self,
        manifest: model::manifest::Manifest,
    ) -> err::Result<Arc<Mutex<shared::State<T::Checker>>>> {
        let observer = obs::Observer::new();
        let shin = shared::State {
            conds: self.conds.clone(),
            observer,
            checker: self.entry.checker(),
            manifest,
        };
        Ok(Arc::new(Mutex::new(shin)))
    }

    fn run_rotation(
        &self,
        shared: Arc<Mutex<shared::State<T::Checker>>>,
        automata: fsa::Set<T, T::Env>,
    ) -> err::Result<(halt::Type, fsa::Set<T, T::Env>)> {
        crossbeam::thread::scope(|s| {
            automata.run(
                |r: fsa::Ready<T, T::Env>| {
                    let builder = s.builder().name(format!("P{0}", r.tid()));
                    let thrd = thread::Thread::<T::Checker> {
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
