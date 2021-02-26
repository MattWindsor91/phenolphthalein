//! The high-level test runner.
//!
use super::{
    fsa, halt, obs,
    permute::{HasTid, Permuter},
    shared, sync,
};
use crate::{api::abs, err, model};

/// A builder for tests.
pub struct Builder<'a, T: abs::Entry<'a>> {
    // TODO(@MattWindsor91): use the actual builder pattern here.
    /// The halting rules that should be applied to tests run by this runner.
    halt_rules: Vec<halt::Rule>,

    /// The factory function to use to construct synchronisation.
    sync: sync::Factory,

    /// A cloneable entry into the test.
    entry: T,

    /// The factory function to use to construct a checker.
    checker: abs::check::Factory<'a, T, T::Env>,

    /// The permuter to use for permuting threads.
    permuter: Box<dyn Permuter<fsa::Ready<'a, T>>>,
}

impl<'a, T: abs::Entry<'a>> Builder<'a, T> {
    /// Constructs a new builder with minimalistic defaults.
    pub fn new(entry: T) -> Self {
        Self {
            entry,
            halt_rules: vec![],
            sync: sync::make_spinner,
            checker: abs::check::unknown_factory,
            permuter: Box::new(super::permute::Nop),
        }
    }

    /// Adds the given halt rules to this builder.
    pub fn add_halt_rules(mut self, rules: impl IntoIterator<Item = halt::Rule>) -> Self {
        self.halt_rules.extend(rules);
        self
    }

    /// Overrides this builder's checker factory.
    pub fn with_checker(mut self, checker: abs::check::Factory<'a, T, T::Env>) -> Self {
        self.checker = checker;
        self
    }

    /// Overrides this builder's synchroniser factory.
    pub fn with_sync(mut self, sync: sync::Factory) -> Self {
        self.sync = sync;
        self
    }

    /// Overrides this builder's permuter factory.
    pub fn with_permuter(mut self, permuter: Box<dyn Permuter<fsa::Ready<'a, T>>>) -> Self {
        self.permuter = permuter;
        self
    }

    pub fn build(self) -> err::Result<Runner<'a, T>> {
        let manifest = self.entry.make_manifest()?;
        let shared = self.make_shared_state(manifest)?;
        let automata = fsa::Set::new(self.entry.clone(), self.sync, shared)?;

        Ok(Runner {
            automata: Some(automata),
            permuter: self.permuter,
            report: None,
        })
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
            checker: (self.checker)(&self.entry),
            env,
        })
    }
}

pub struct Runner<'a, T: abs::Entry<'a>> {
    automata: Option<fsa::Set<'a, T>>,
    report: Option<model::obs::Report>,
    permuter: Box<dyn Permuter<fsa::Ready<'a, T>> + 'a>,
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
