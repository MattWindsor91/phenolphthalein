use crate::{model, testapi::abs};
use std::collections::{BTreeMap, HashMap};

/* TODO(@MattWindsor91): morally, a State should only borrow the variable names,
   as they are held by the parent Observer's Manifest for the entire scope that
   States are available; trying to get this to work with borrowck has proven a
   little difficult.
*/

/// An observation after a particular test iteration.
type Obs = BTreeMap<String, i32>;

/// An observer for the outcomes of a test.
#[derive(Default)]
pub struct Observer {
    /// The observations that this observer has made so far.
    pub obs: HashMap<Obs, Info>,

    /// The number of iterations this observer has seen so far.
    iterations: usize,
}

/// Information about an observation.
#[derive(Copy, Clone)]
pub struct Info {
    /// The number of times this observation has occurred.
    pub occurs: usize,
    /// The result of asking the test to check this observation.
    pub check_result: model::check::Outcome,
}

impl Info {
    /// Computes the Info resulting from increasing this Info's
    /// occurs count by 1.
    pub fn inc(&self) -> Info {
        Info {
            occurs: self.occurs + 1,
            check_result: self.check_result,
        }
    }
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
        self.iterations += 1;
        Summary {
            iterations: self.iterations,
            info,
        }
    }

    fn observe_state<T: abs::Env, C: abs::Checker<Env = T>>(
        &mut self,
        env: &mut Manifested<T>,
        checker: &C,
    ) -> Info {
        let state = current_state(env);
        let info = self.obs.get(&state).map_or_else(
            || {
                let check_result = checker.check(env.env);
                Info {
                    occurs: 1,
                    check_result,
                }
            },
            Info::inc,
        );
        self.obs.insert(state, info);
        info
    }
}

/// Gets the current state of the environment.
/// Note that this is not thread-safe until all test threads are synchronised.
fn current_state<T: abs::Env>(env: &Manifested<T>) -> Obs {
    let mut s = Obs::new();
    // TODO(@MattWindsor91): have one great big iterator for values and collect it.
    s.extend(env.atomic_int_values());
    s.extend(env.int_values());
    s
}

/// A summary of the observer's current state, useful for calculating test
/// exit conditions.
pub struct Summary {
    /// The number of iterations the observer has seen so far, including
    /// this one.
    pub iterations: usize,

    /// The information from the current observation.
    pub info: Info,
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
            self.env.set_atomic_int(i, r.initial_value.unwrap_or(0))
        }
        for (i, (_, r)) in self.manifest.ints.iter().enumerate() {
            self.env.set_int(i, r.initial_value.unwrap_or(0))
        }
    }

    // Iterates over all of the atomic integer variables in the environment.
    pub fn atomic_int_values(&self) -> impl Iterator<Item = (String, i32)> + '_ {
        self.manifest
            .atomic_int_names()
            .enumerate()
            .map(move |(i, n)| (n.to_string(), self.env.atomic_int(i)))
    }

    // Iterates over all of the integer variables in the environment.
    pub fn int_values(&self) -> impl Iterator<Item = (String, i32)> + '_ {
        self.manifest
            .int_names()
            .enumerate()
            .map(move |(i, n)| (n.to_string(), self.env.int(i)))
    }
}
