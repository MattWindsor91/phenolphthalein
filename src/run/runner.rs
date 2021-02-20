//! The high-level test runner.
//!
use super::{fsa, fsa::Fsa, halt, obs, shared, sync};
use crate::{err, model, testapi::abs};

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
    pub fn build(self) -> err::Result<Runner<'a, T>> {
        let manifest = self.entry.make_manifest()?;
        let shared = self.make_shared_state(manifest)?;
        let automata = fsa::Set::new(self.entry.clone(), self.sync, shared)?;

        let permuter = self.permuter();

        Ok(Runner {
            automata: Some(automata),
            permuter,
            report: None,
        })
    }

    fn permuter(&self) -> Box<dyn fsa::Permuter<'a, T> + 'a> {
        if self.permute_threads {
            Box::new(rand::thread_rng())
        } else {
            Box::new(fsa::NopPermuter {})
        }
    }

    fn make_shared_state(
        &self,
        manifest: model::manifest::Manifest,
    ) -> err::Result<shared::State<'a, T::Env>> {
        let mut env = obs::Manifested::for_manifest(manifest)?;
        env.reset();

        let observer = obs::Observer::new();
        Ok(shared::State {
            halt_rules: self.halt_rules.clone(),
            observer,
            checker: self.make_checker(),
            env,
        })
    }

    fn make_checker(&self) -> Box<dyn model::check::Checker<T::Env> + 'a> {
        if self.check {
            self.entry.checker()
        } else {
            Box::new(model::check::Outcome::Unknown)
        }
    }
}

pub struct Runner<'a, T: abs::Entry<'a>> {
    automata: Option<fsa::Set<'a, T>>,
    report: Option<model::obs::Report>,
    permuter: Box<dyn fsa::Permuter<'a, T> + 'a>,
}

impl<'a, 'scope> fsa::Threader<'a, 'scope> for &'scope crossbeam::thread::Scope<'a> {
    type Handle = crossbeam::thread::ScopedJoinHandle<'scope, fsa::Done>;

    fn spawn<T: abs::Entry<'a> + 'a>(
        &'scope self,
        automaton: fsa::Ready<'a, T>,
    ) -> err::Result<Self::Handle> {
        let builder = self.builder().name(format!("P{0}", automaton.tid()));
        Ok(builder.spawn(move |_| run_thread(automaton.start()))?)
    }

    fn join(&'scope self, handle: Self::Handle) -> err::Result<fsa::Done> {
        handle.join().map_err(|_| err::Error::ThreadPanic)
    }
}

impl<'a, T: abs::Entry<'a>> Runner<'a, T> {
    /// Runs the Runner's test until it exits.
    pub fn run(mut self) -> err::Result<model::obs::Report> {
        while let Some(am) = self.automata.take() {
            match self.run_rotation(am)? {
                fsa::Outcome::Rotate(am) => {
                    self.automata.replace(am);
                }
                fsa::Outcome::Exit(state) => self.make_report(state),
            }
        }
        // TODO(@MattWindsor91): for now
        self.report.ok_or(err::Error::LockReleaseFailed)
    }

    fn run_rotation(&mut self, automata: fsa::Set<'a, T>) -> err::Result<fsa::Outcome<'a, T>> {
        crossbeam::thread::scope(|s| automata.run(&s, &mut *self.permuter))
            .map_err(|_| err::Error::ThreadPanic)?
    }

    fn make_report(&mut self, state: shared::State<'a, T::Env>) {
        self.report.replace(state.observer.into_report());
    }
}

fn run_thread<'a, T: abs::Entry<'a>>(mut t: fsa::Runnable<'a, T>) -> fsa::Done {
    loop {
        match t.run() {
            fsa::RunOutcome::Done(d) => return d,
            fsa::RunOutcome::Wait(w) => t = w.wait(),
            fsa::RunOutcome::Observe(o) => t = observe(o),
        }
    }
}

fn observe<'a, T: abs::Entry<'a>>(mut o: fsa::Observable<'a, T>) -> fsa::Runnable<'a, T> {
    let shared = o.shared_state();
    if let Some(exit_type) = shared.observe() {
        o.kill(exit_type)
    } else {
        o.relinquish()
    }
}
