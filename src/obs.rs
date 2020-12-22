use crate::{env, manifest};
use std::collections::{BTreeMap, HashMap};

/* TODO(@MattWindsor91): morally, a State should only borrow the variable names,
   as they are held by the parent Observer's Manifest for the entire scope that
   States are available; trying to get this to work with borrowck has proven a
   little difficult.
*/

type State = BTreeMap<String, i32>;

pub struct Observer {
    manifest: manifest::Manifest,
    pub obs: HashMap<State, usize>,
}

impl Observer {
    pub fn new(manifest: manifest::Manifest) -> Self {
        Observer {
            manifest,
            obs: HashMap::new(),
        }
    }

    /// Observes a test environment into this runner's observations.
    pub fn observe_and_reset(&mut self, env: &mut dyn env::AnEnv) {
        self.observe(env);
        self.reset(env)
    }

    fn observe(&mut self, env: &dyn env::AnEnv) {
        let state = self.current_state(env);
        let inc = self.obs.get(&state).map_or(0, |k| k + 1);
        self.obs.insert(state, inc);
    }

    /// Gets the current state of the environment.
    /// Note that this is not thread-safe until all test threads are synchronised.
    fn current_state(&self, env: &dyn env::AnEnv) -> State {
        // TODO(@MattWindsor91): work out a good state-machine-ish approach for
        // ensuring this can only be called when threads are quiescent.
        let mut s = State::new();
        // TODO(@MattWindsor91): have one great big iterator for values and collect it.
        s.extend(self.atomic_int_values(env));
        s.extend(self.int_values(env));
        s
    }

    fn atomic_int_values(&self, env: &dyn env::AnEnv) -> State {
        self.manifest
            .atomic_int_names()
            .enumerate()
            .map(|(i, n)| (n.to_string(), env.atomic_int(i)))
            .collect()
    }

    fn int_values(&self, env: &dyn env::AnEnv) -> State {
        self.manifest
            .int_names()
            .enumerate()
            .map(|(i, n)| (n.to_string(), env.int(i)))
            .collect()
    }

    /// Resets every variable in the environment to its initial value.
    fn reset(&mut self, env: &mut dyn env::AnEnv) {
        for (i, (_, r)) in self.manifest.atomic_ints.iter().enumerate() {
            env.set_atomic_int(i, r.initial_value.unwrap_or(0))
        }
        for (i, (_, r)) in self.manifest.ints.iter().enumerate() {
            env.set_int(i, r.initial_value.unwrap_or(0))
        }
    }
}
