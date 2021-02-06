//! Shared state in the runner.
//!
//! Presently we implement this using a mutex, but future work might let the
//! synchronisers in `sync` also synchronise access to this.

use super::{halt, obs};
use crate::{model, testapi::abs};

/// The shared state available to runner threads whenever they get promoted to
/// observers.
pub struct State<'a, E> {
    /// The state checker for the test.
    pub checker: Box<dyn abs::Checker<E> + 'a>,
    /// The halt rules for the test.
    pub halt_rules: Vec<halt::Rule>,
    /// The observer for the test.
    pub observer: obs::Observer,
    /// The manifest for the test.
    pub manifest: model::manifest::Manifest,
}

impl<'a, E: abs::Env> State<'a, E> {
    /// Handles the environment, including observing it and resetting it.
    pub fn handle(&mut self, env: &mut E) -> Option<halt::Type> {
        let mut m = obs::Manifested {
            manifest: &self.manifest,
            env,
        };
        let summary = self.observer.observe(&mut m, &*self.checker);
        m.reset();
        self.exit_type(summary)
    }

    /// Checks whether the test should exit now.
    pub fn exit_type(&self, summary: obs::Summary) -> Option<halt::Type> {
        self.halt_rules
            .iter()
            .filter_map(|c| c.exit_type(&summary))
            .max()
    }
}
