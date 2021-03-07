//! The high-level test runner.
//!
use super::{
    fsa, halt, instance, obs,
    permute::{self, Permuter},
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
    permuter: permute::Factory<fsa::Ready<'a, T>>,
}

impl<'a, T: abs::Entry<'a>> Builder<'a, T> {
    /// Constructs a new builder with minimalistic defaults.
    pub fn new(entry: T) -> Self {
        Self {
            entry,
            halt_rules: vec![],
            sync: sync::make_spinner,
            checker: abs::check::make_unknown,
            permuter: permute::make_nop,
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
    pub fn with_permuter(mut self, permuter: permute::Factory<fsa::Ready<'a, T>>) -> Self {
        self.permuter = permuter;
        self
    }

    /// Builds a test runner with the stored configuration.
    ///
    /// Building doesn't take ownership of the builder, so it can be used to
    /// run multiple (isolated) instances of the same test.
    pub fn build(&self) -> err::Result<Runner<'a, T>> {
        let manifest = self.entry.make_manifest()?;
        let shared = self.make_shared_state(manifest)?;

        Ok(Runner {
            instance: Some(instance::Instance::new(
                self.entry.clone(),
                self.sync,
                shared,
            )?),
            permuter: (self.permuter)(),
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

/// A top-level runner for a particular
pub struct Runner<'a, T: abs::Entry<'a>> {
    instance: Option<instance::Instance<'a, T>>,
    report: Option<model::report::Report>,
    permuter: Box<dyn Permuter<fsa::Ready<'a, T>> + 'a>,
}

impl<'a, T: abs::Entry<'a>> Runner<'a, T> {
    /// Runs the Runner's test until it exits.
    pub fn run(mut self) -> err::Result<model::report::Report> {
        while let Some(am) = self.instance.take() {
            match self.run_rotation(am)? {
                instance::Outcome::Rotate(am) => {
                    self.instance.replace(am);
                }
                instance::Outcome::Exit(state) => self.make_report(state),
            }
        }
        // TODO(@MattWindsor91): for now
        self.report.ok_or(err::Error::LockReleaseFailed)
    }

    fn run_rotation(
        &mut self,
        automata: instance::Instance<'a, T>,
    ) -> err::Result<instance::Outcome<'a, T>> {
        crossbeam::thread::scope(|s| automata.run(&s, &mut *self.permuter))
            .map_err(|_| err::Error::ThreadPanic)?
    }

    fn make_report(&mut self, state: shared::State<'a, T::Env>) {
        self.report.replace(state.observer.into_report());
    }
}
