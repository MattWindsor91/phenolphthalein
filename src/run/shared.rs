//! Shared state in the runner.
//!
//! Presently we implement this using a mutex, but future work might let the
//! synchronisers in `sync` also synchronise access to this.

use super::{halt, obs};
use crate::{api::abs, model};

/// The shared state available to runner threads whenever they get promoted to
/// observers.
pub struct State<'a, E> {
    /// The state checker for the test.
    pub checker: Box<dyn model::check::Checker<E> + 'a>,
    /// The manifested environment.
    pub env: obs::Manifested<E>,
    /// The halt rules for the test.
    pub halt_rules: Vec<halt::Rule>,
    /// The observer for the test.
    pub observer: obs::Observer,
}

impl<'a, E: abs::Env> State<'a, E> {
    /// Handles the environment, including observing it and resetting it.
    pub fn observe(&mut self) -> Option<halt::Type> {
        let summary = self.observer.observe(&mut self.env, &*self.checker);
        self.env.reset();
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
