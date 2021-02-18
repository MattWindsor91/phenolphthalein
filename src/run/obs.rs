use crate::{err, model, testapi::abs};

/* TODO(@MattWindsor91): morally, a State should only borrow the variable names,
   as they are held by the parent Observer's Manifest for the entire scope that
   States are available; trying to get this to work with borrowck has proven a
   little difficult.
*/

/// An observer for the outcomes of a test.
#[derive(Default)]
pub struct Observer {
    /// The observations that this observer has made so far.
    pub obs: model::obs::Set,

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
        checker: &'a dyn model::check::Checker<E>,
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
        checker: &'a dyn model::check::Checker<E>,
    ) -> model::obs::Obs {
        let state = current_state(env);
        let info = self.obs.get(&state).map_or_else(
            || self.observe_state_for_first_time(&env.env, checker),
            model::obs::Obs::inc,
        );
        self.obs.insert(state, info);
        info
    }

    fn observe_state_for_first_time<'a, E: abs::Env>(
        &self,
        env: &E,
        checker: &'a dyn model::check::Checker<E>,
    ) -> model::obs::Obs {
        let check_result = checker.check(env);
        model::obs::Obs {
            occurs: 1,
            check_result,
            iteration: self.iterations,
        }
    }

    /// Consumes this Observer and returns a summary of its state.
    pub fn into_report(self) -> model::obs::Report {
        let outcome = self.obs.iter().map(|(_, v)| v.check_result).max();
        model::obs::Report {
            outcome,
            obs: self.obs,
        }
    }
}

/// Gets the current state of the environment.
/// Note that this is not thread-safe until all test threads are synchronised.
fn current_state<T: abs::Env>(env: &Manifested<T>) -> model::obs::State {
    let mut s = model::obs::State::new();
    // TODO(@MattWindsor91): have one great big iterator for values and collect it.
    s.extend(env.i32_values());
    s
}

/// A summary of the observer's current state, useful for calculating test
/// exit conditions.
pub struct Summary {
    /// The number of iterations the observer has seen so far, including
    /// this one.  This number will saturate at usize.MAX.
    pub iterations: usize,

    /// The information from the current observation.
    pub info: model::obs::Obs,
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
            self.env.set_i32(r.slot, r.initial_value.unwrap_or(0))
        }
    }

    // Iterates over all of the 32-bit integer variables in the environment.
    pub fn i32_values(&self) -> impl Iterator<Item = (String, i32)> + '_ {
        self.manifest
            .i32s
            .iter()
            .map(move |(n, r)| (n.to_string(), self.env.get_i32(r.slot)))
    }

    /// Constructs a manifested environment for a given manifest.
    pub fn for_manifest(manifest: model::manifest::Manifest) -> err::Result<Manifested<E>> {
        let env = E::for_manifest(&manifest)?;
        Ok(Self { manifest, env })
    }
}
