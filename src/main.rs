extern crate crossbeam;
extern crate dlopen;
#[macro_use]
extern crate dlopen_derive;
extern crate libc;

mod c;
mod env;
mod manifest;
mod obs;
mod test;

use crossbeam::thread;
use std::sync::{Arc, Mutex};

struct Thread {
    observer: Arc<Mutex<obs::Observer>>,
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

fn run_with_test(test: c::CTestApi) -> c::Result<()> {
    let manifest = test.make_manifest()?;
    let observer = obs::Observer::new(manifest.clone());

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
    })
    .unwrap();

    if let Ok(m) = Arc::try_unwrap(mob) {
        for (k, v) in m.into_inner().unwrap().obs {
            println!("{0:?}: {1}", k, v);
        }
        // nb: this needs to percolate into an error if it fails.
    }

    Ok(())
}
