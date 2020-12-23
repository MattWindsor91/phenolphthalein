use crate::{env, manifest};
use std::collections::{BTreeMap, HashMap};

/* TODO(@MattWindsor91): morally, a State should only borrow the variable names,
   as they are held by the parent Observer's Manifest for the entire scope that
   States are available; trying to get this to work with borrowck has proven a
   little difficult.
*/

/// An observation after a particular test iteration.
type Obs = BTreeMap<String, i32>;

/// Type of functions that can check an environment.
pub trait Checker: Sync + Send + Clone {
    // The type of the environment this checker checks.
    type Env: env::Env;

    fn check(&self, env: &Self::Env) -> bool;
}

pub struct Observer {
    manifest: manifest::Manifest,

    /// The observations that this observer has made so far.
    pub obs: HashMap<Obs, Info>,
}

/// Information about an observation.
pub struct Info {
    /// The number of times this observation has occurred.
    pub occurs: usize,
    /// The result of asking the test to check this observation.
    pub check_result: bool,
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
    pub fn new(manifest: manifest::Manifest) -> Self {
        Observer {
            manifest,
            obs: HashMap::new(),
        }
    }

    /// Observes a test environment into this runner's observations.
    pub fn observe_and_reset<T: env::Env, C: Checker<Env = T>>(
        &mut self,
        env: &mut T,
        checker: &C,
    ) {
        self.observe(env, checker);
        self.reset(env)
    }

    fn observe<T, C>(&mut self, env: &mut T, checker: &C)
    where
        T: env::Env,
        C: Checker<Env = T>,
    {
        let state = self.current_state(env);
        let inc = self.obs.get(&state).map_or_else(
            || {
                let check_result = checker.check(env);
                Info {
                    occurs: 1,
                    check_result,
                }
            },
            Info::inc,
        );
        self.obs.insert(state, inc);
    }

    /// Gets the current state of the environment.
    /// Note that this is not thread-safe until all test threads are synchronised.
    fn current_state<T: env::Env>(&self, env: &T) -> Obs {
        let mut s = Obs::new();
        // TODO(@MattWindsor91): have one great big iterator for values and collect it.
        s.extend(self.atomic_int_values(env));
        s.extend(self.int_values(env));
        s
    }

    fn atomic_int_values<T: env::Env>(&self, env: &T) -> Obs {
        self.manifest
            .atomic_int_names()
            .enumerate()
            .map(|(i, n)| (n.to_string(), env.atomic_int(i)))
            .collect()
    }

    fn int_values<T: env::Env>(&self, env: &T) -> Obs {
        self.manifest
            .int_names()
            .enumerate()
            .map(|(i, n)| (n.to_string(), env.int(i)))
            .collect()
    }

    /// Resets every variable in the environment to its initial value.
    fn reset<T: env::Env>(&mut self, env: &mut T) {
        // TODO(@MattWindsor91): this seems an odd inversion of control?

        for (i, (_, r)) in self.manifest.atomic_ints.iter().enumerate() {
            env.set_atomic_int(i, r.initial_value.unwrap_or(0))
        }
        for (i, (_, r)) in self.manifest.ints.iter().enumerate() {
            env.set_int(i, r.initial_value.unwrap_or(0))
        }
    }
}
