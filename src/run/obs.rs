use crate::{
    api::abs,
    err,
    model::{self, state},
};

/* TODO(@MattWindsor91): morally, a State should only borrow the variable names,
   as they are held by the parent Observer's Manifest for the entire scope that
   States are available; trying to get this to work with borrowck has proven a
   little difficult.
*/

/// An observer for the outcomes of a test.
#[derive(Default)]
pub struct Observer {
    /// The observations that this observer has made so far.
    pub obs: std::collections::HashMap<state::State, state::Info>,

    /// The number of iterations this observer has seen so far.
    iterations: usize,
}

impl Observer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Observes a test environment into this runner's observations.
    pub fn observe<'a, E: abs::Env>(
        &mut self,
        env: &mut Manifested<E>,
        checker: &'a dyn abs::Checker<E>,
    ) -> Summary {
        let info = self.observe_state(env, checker);
        self.iterations = self.iterations.saturating_add(1);
        Summary {
            iterations: self.iterations,
            info,
        }
    }

    fn observe_state<'a, E: abs::Env>(
        &mut self,
        env: &mut Manifested<E>,
        checker: &'a dyn abs::Checker<E>,
    ) -> model::state::Info {
        let state = current_state(env);
        let info = self.obs.get(&state).map_or_else(
            || self.observe_state_for_first_time(&env.env, checker),
            model::state::Info::inc,
        );
        self.obs.insert(state, info);
        info
    }

    fn observe_state_for_first_time<'a, E: abs::Env>(
        &self,
        env: &E,
        checker: &'a dyn abs::Checker<E>,
    ) -> model::state::Info {
        let outcome = checker.check(env);
        model::state::Info::new(outcome, self.iterations)
    }

    /// Consumes this Observer and returns a summary of its state.
    pub fn into_report(self) -> model::report::Report {
        let mut report = model::report::Report {
            outcome: None,
            states: Vec::with_capacity(self.obs.len()),
        };

        for (state, info) in self.obs {
            report.insert(model::report::State { state, info });
        }

        report
    }
}

/// Gets the current state of the environment.
/// Note that this is not thread-safe until all test threads are synchronised.
fn current_state<T: abs::Env>(env: &Manifested<T>) -> model::state::State {
    let mut s = model::state::State::new();
    s.extend(env.values());
    s
}

/// A summary of the observer's current state, useful for calculating test
/// exit conditions.
#[derive(Clone, Copy)]
pub struct Summary {
    /// The number of iterations the observer has seen so far, including
    /// this one.  This number will saturate at usize.MAX.
    pub iterations: usize,

    /// The information from the current observation.
    pub info: model::state::Info,
}

/// An environment combined with a manifest.
///
/// Bundling these two together lets us interpret the environment using the
/// manifest.
///
/// For this to be safe, we assume that the environment gracefully handles any
/// mismatches between itself and the manifest.
pub struct Manifested<E> {
    /// The manifest.
    pub manifest: model::manifest::Manifest,

    /// The environment being interpreted by the manifest.
    pub env: E,
}

impl<E: abs::Env> Manifested<E> {
    /// Resets the environment to the initial values in the manifest.
    pub fn reset(&mut self) {
        for r in self.manifest.i32s.values() {
            self.env.set_i32(r.slot, r.initial_value.unwrap_or(0));
        }
    }

    // Iterates over all of the variables in the environment.
    pub fn values(&self) -> impl Iterator<Item = (String, model::state::Value)> + '_ {
        // Space for rent.
        self.i32_values()
    }

    // Iterates over all of the 32-bit integer variables in the environment.
    fn i32_values(&self) -> impl Iterator<Item = (String, model::state::Value)> + '_ {
        self.manifest.i32s.iter().map(move |(n, r)| {
            (
                n.to_string(),
                model::state::Value::I32(self.env.get_i32(r.slot)),
            )
        })
    }

    /// Constructs a manifested environment for a given manifest.
    pub fn for_manifest(manifest: model::manifest::Manifest) -> err::Result<Manifested<E>> {
        let env = E::of_reservations(manifest.reserve())?;
        Ok(Self { manifest, env })
    }
}
