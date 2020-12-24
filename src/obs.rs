use crate::env;
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

    /// Checks the current state of the environment.
    fn check(&self, env: &Self::Env) -> CheckResult;
}

/// The result of running a checker.
#[derive(Copy, Clone)]
pub enum CheckResult {
    /// The observation passed its check.
    Passed,
    /// The observation failed its check.
    Failed,
}

/// An observer for the outcomes of a test.
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
    pub check_result: CheckResult,
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
        Observer {
            obs: HashMap::new(),
            iterations: 0,
        }
    }

    /// Observes a test environment into this runner's observations.
    pub fn observe<T: env::Env, C: Checker<Env = T>>(
        &mut self,
        env: &mut env::Manifested<T>,
        checker: &C,
    ) -> Summary {
        let info = self.observe_state(env, checker);
        self.iterations += 1;
        Summary {
            iterations: self.iterations,
            info,
        }
    }

    fn observe_state<T: env::Env, C: Checker<Env = T>>(
        &mut self,
        env: &mut env::Manifested<T>,
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
fn current_state<T: env::Env>(env: &env::Manifested<T>) -> Obs {
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
