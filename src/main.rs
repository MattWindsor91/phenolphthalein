extern crate crossbeam;
extern crate dlopen;
#[macro_use]
extern crate dlopen_derive;
extern crate libc;

mod c;
mod env;
mod test;
mod manifest;

use crossbeam::thread;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

/* TODO(@MattWindsor91): morally, a State should only borrow the variable names,
   as they are held by the parent Observer's Manifest for the entire scope that
   States are available; trying to get this to work with borrowck has proven a
   little difficult.
*/   

type State = BTreeMap<String, i32>;

struct Observer {
    pub manifest: manifest::Manifest,
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
        self.manifest.atomic_int_names()
            .enumerate()
            .map(|(i, n)| (n.to_string(), env.atomic_int(i)))
            .collect()
    }

    fn int_values(&self, env: &dyn env::AnEnv) -> State {
        self.manifest.int_names()
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

struct Thread {
    observer: Arc<Mutex<Observer>>,
}

impl<'a> Thread {
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


fn main() {
    run().unwrap();
}

fn run() -> c::Result<()> {
    let lib = dlopen::symbor::Library::open("test.dylib")?;
    let test = c::load_test(&lib)?;
    run_with_test(test)
}

fn run_with_test<'a>(test: c::CTestApi<'a>) -> c::Result<()> {
    let manifest = test.make_manifest()?;
    let observer = Observer::new(manifest.clone());

    let tests = test::build(test, manifest)?;
    let mob = Arc::new(Mutex::new(observer));

    thread::scope(|s| {
        let mut handles: Vec<thread::ScopedJoinHandle<()>> = Vec::with_capacity(tests.len());

        for (i, t) in tests.into_iter().enumerate() {
            let builder = s.builder().name(format!("P{0}", i));
            let thrd = Thread {
                observer: mob.clone(),
            };
            let h = builder.spawn(move |_| thrd.run(t.start())).unwrap();
            handles.push(h)
        }

        // TODO(@MattWindsor91): the observations should only be visible from the environment once we've joined these threads
        // in general, all of the thread-unsafe stuff should be hidden inside the environment
        for h in handles.into_iter() {
            h.join().unwrap();
        }
    }).unwrap();

    if let Ok(m) = Arc::try_unwrap(mob) {
        for (k, v) in m.into_inner().unwrap().obs {
            println!("{0:?}: {1}", k, v);
        }
        // nb: this needs to percolate into an error if it fails.
    }

    Ok(())
}
