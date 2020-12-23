use crate::{env, manifest};
use std::collections::{BTreeMap, HashMap};

/* TODO(@MattWindsor91): morally, a State should only borrow the variable names,
   as they are held by the parent Observer's Manifest for the entire scope that
   States are available; trying to get this to work with borrowck has proven a
   little difficult.
*/

type State = BTreeMap<String, i32>;

/// Type of functions that can check an environment.
pub trait Checker: Sync + Send {
    // The type of the environment this checker checks.
    type Env;

    fn check(&self, env: &Self::Env) -> bool;
}

pub struct Observer {
    manifest: manifest::Manifest,
    pub obs: HashMap<State, StateInfo>,
}

pub struct StateInfo {
    pub occurs: usize,
    pub check_result: bool,
}

impl StateInfo {
    /// Computes the StateInfo resulting from increasing this StateInfo's
    /// occurs count by 1.
    pub fn inc(&self) -> StateInfo {
        StateInfo {
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
    pub fn observe_and_reset<T, C>(&mut self, env: &mut T, checker: &C)
    where
        T: env::AnEnv,
        C: Checker<Env = T>,
    {
        self.observe(env, checker);
        self.reset(env)
    }

    fn observe<T, C>(&mut self, env: &mut T, checker: &C)
    where
        T: env::AnEnv,
        C: Checker<Env = T>,
    {
        let state = self.current_state(env);
        let inc = self.obs.get(&state).map_or_else(
            || {
                let check_result = checker.check(env);
                StateInfo {
                    occurs: 1,
                    check_result,
                }
            },
            StateInfo::inc,
        );
        self.obs.insert(state, inc);
    }

    /// Gets the current state of the environment.
    /// Note that this is not thread-safe until all test threads are synchronised.
    fn current_state<T>(&self, env: &T) -> State
    where
        T: env::AnEnv,
    {
        // TODO(@MattWindsor91): work out a good state-machine-ish approach for
        // ensuring this can only be called when threads are quiescent.
        let mut s = State::new();
        // TODO(@MattWindsor91): have one great big iterator for values and collect it.
        s.extend(self.atomic_int_values(env));
        s.extend(self.int_values(env));
        s
    }

    fn atomic_int_values<T>(&self, env: &T) -> State
    where
        T: env::AnEnv,
    {
        self.manifest
            .atomic_int_names()
            .enumerate()
            .map(|(i, n)| (n.to_string(), env.atomic_int(i)))
            .collect()
    }

    fn int_values<T>(&self, env: &T) -> State
    where
        T: env::AnEnv,
    {
        self.manifest
            .int_names()
            .enumerate()
            .map(|(i, n)| (n.to_string(), env.int(i)))
            .collect()
    }

    /// Resets every variable in the environment to its initial value.
    fn reset<T>(&mut self, env: &mut T)
    where
        T: env::AnEnv,
    {
        // TODO(@MattWindsor91): this seems an odd inversion of control?

        for (i, (_, r)) in self.manifest.atomic_ints.iter().enumerate() {
            env.set_atomic_int(i, r.initial_value.unwrap_or(0))
        }
        for (i, (_, r)) in self.manifest.ints.iter().enumerate() {
            env.set_int(i, r.initial_value.unwrap_or(0))
        }
    }
}
