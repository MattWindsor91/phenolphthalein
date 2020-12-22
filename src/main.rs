extern crate libc;
mod env;
mod test;

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use std::thread;

type State<'a> = BTreeMap<&'a str, i32>;

struct Observer<'a> {
    atomic_ints: BTreeMap<&'a str, VarRecord<i32>>,
    ints: BTreeMap<&'a str, VarRecord<i32>>,

    pub obs: HashMap<State<'a>, usize>,
}

impl<'a> Observer<'a> {
    pub fn new(
        atomic_ints: BTreeMap<&'a str, VarRecord<i32>>,
        ints: BTreeMap<&'a str, VarRecord<i32>>,
    ) -> Self {
        Observer {
            atomic_ints,
            ints,
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
    fn current_state(&self, env: &dyn env::AnEnv) -> State<'a> {
        // TODO(@MattWindsor91): work out a good state-machine-ish approach for
        // ensuring this can only be called when threads are quiescent.
        let mut s = State::new();
        // TODO(@MattWindsor91): have one great big iterator for values and collect it.
        s.extend(self.atomic_int_values(env).iter());
        s.extend(self.int_values(env).iter());
        s
    }

    fn atomic_int_values(&self, env: &dyn env::AnEnv) -> BTreeMap<&'a str, i32> {
        self.atomic_ints
            .iter()
            .enumerate()
            .map(|(i, (n, _))| (*n, env.atomic_int(i)))
            .collect()
    }

    fn int_values(&self, env: &dyn env::AnEnv) -> BTreeMap<&'a str, i32> {
        self.ints
            .iter()
            .enumerate()
            .map(|(i, (n, _))| (*n, env.int(i)))
            .collect()
    }

    /// Resets every variable in the environment to its initial value.
    fn reset(&mut self, env: &mut dyn env::AnEnv) {
        for (i, (_, r)) in self.atomic_ints.iter().enumerate() {
            env.set_atomic_int(i, r.initial_value.unwrap_or(0))
        }
        for (i, (_, r)) in self.ints.iter().enumerate() {
            env.set_int(i, r.initial_value.unwrap_or(0))
        }
    }
}

struct Thread<'a> {
    observer: Arc<Mutex<Observer<'a>>>,
}

impl<'a> Thread<'a> {
    fn run(&self, t: test::RunnableTest) {
        let mut t = t;
        for _i in 0..=100 {
            match t.run() {
                test::RunOutcome::Wait(w) => t = w.wait(),
                test::RunOutcome::Observe(mut o) => {
                    self.observe_and_reset(o.env());
                    t = o.relinquish().wait()
                }
            }
        }
    }

    fn observe_and_reset(&self, e: &mut dyn env::AnEnv) {
        // TODO(@MattWindsor91): handle poisoning here
        let mut g = self.observer.lock().unwrap();
        g.observe_and_reset(e);
    }
}

struct VarRecord<T> {
    initial_value: Option<T>, // Space for rent
}

fn main() {
    let mut atomic_ints = BTreeMap::new();
    atomic_ints.insert(
        "x",
        VarRecord {
            initial_value: Some(0),
        },
    );
    atomic_ints.insert(
        "y",
        VarRecord {
            initial_value: Some(0),
        },
    );

    let mut ints = BTreeMap::new();
    ints.insert(
        "0:r0",
        VarRecord {
            initial_value: Some(0),
        },
    );
    ints.insert(
        "1:r0",
        VarRecord {
            initial_value: Some(0),
        },
    );

    let observer = Observer::new(atomic_ints, ints);

    let nthreads = 2;
    let mut handles: Vec<thread::JoinHandle<()>> = Vec::with_capacity(nthreads);

    let b = test::TestBuilder::new(nthreads, observer.atomic_ints.len(), observer.ints.len());
    let mob = Arc::new(Mutex::new(observer));

    let tests = b.build().unwrap();
    for (i, t) in tests.into_iter().enumerate() {
        let builder = thread::Builder::new().name(format!("P{0}", i));
        let thrd = Thread {
            observer: mob.clone(),
        };
        let h = builder.spawn(move || thrd.run(t.start())).unwrap();
        handles.push(h)
    }

    // TODO(@MattWindsor91): the observations should only be visible from the environment once we've joined these threads
    // in general, all of the thread-unsafe stuff should be hidden inside the environment
    for h in handles.into_iter() {
        h.join().unwrap();
    }

    if let Ok(m) = Arc::try_unwrap(mob) {
        for (k, v) in m.into_inner().unwrap().obs {
            println!("{0:?}: {1}", k, v);
        }
    }
}
