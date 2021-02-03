use crate::{model, testapi::abs};

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
    pub fn observe<T: abs::Env, C: abs::Checker<Env = T>>(
        &mut self,
        env: &mut Manifested<T>,
        checker: &C,
    ) -> Summary {
        let info = self.observe_state(env, checker);
        self.iterations = self.iterations.saturating_add(1);
        Summary {
            iterations: self.iterations,
            info,
        }
    }

    fn observe_state<T: abs::Env, C: abs::Checker<Env = T>>(
        &mut self,
        env: &mut Manifested<T>,
        checker: &C,
    ) -> model::obs::Obs {
        let state = current_state(env);
        let info = self.obs.get(&state).map_or_else(
            || {
                let check_result = checker.check(env.env);
                model::obs::Obs {
                    occurs: 1,
                    check_result,
                    iteration: self.iterations,
                }
            },
            model::obs::Obs::inc,
        );
        self.obs.insert(state, info);
        info
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
    s.extend(env.atomic_i32_values());
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

/// A borrowed environment combined with a borrowed manifest.
///
/// Bundling these two together lets us interpret the environment using the
/// manifest.
///
/// For this to be safe, we assume that the environment gracefully handles any
/// mismatches between itself and the manifest.
pub struct Manifested<'a, T> {
    pub manifest: &'a model::manifest::Manifest,
    pub env: &'a mut T,
}

impl<'a, T: abs::Env> Manifested<'a, T> {
    /// Resets the environment to the initial values in the manifest.
    pub fn reset(&mut self) {
        for (i, (_, r)) in self.manifest.atomic_ints.iter().enumerate() {
            self.env.set_atomic_i32(i, r.initial_value.unwrap_or(0))
        }
        for (i, (_, r)) in self.manifest.ints.iter().enumerate() {
            self.env.set_i32(i, r.initial_value.unwrap_or(0))
        }
    }

    // Iterates over all of the atomic integer variables in the environment.
    pub fn atomic_i32_values(&self) -> impl Iterator<Item = (String, i32)> + '_ {
        self.manifest
            .atomic_int_names()
            .enumerate()
            .map(move |(i, n)| (n.to_string(), self.env.get_atomic_i32(i)))
    }

    // Iterates over all of the 32-bit integer variables in the environment.
    pub fn i32_values(&self) -> impl Iterator<Item = (String, i32)> + '_ {
        self.manifest
            .int_names()
            .enumerate()
            .map(move |(i, n)| (n.to_string(), self.env.get_i32(i)))
    }
}
