extern crate crossbeam;
extern crate dlopen;
#[macro_use]
extern crate dlopen_derive;
extern crate libc;

mod c;
mod env;
mod err;
mod fsa;
mod manifest;
mod obs;
mod test;

use crossbeam::thread;
use std::sync::{Arc, Mutex};
use test::Test;

struct Thread<C> {
    checker: C,
    observer: Arc<Mutex<obs::Observer>>,
}

impl<C: obs::Checker> Thread<C> {
    fn run<T>(&self, t: fsa::Runnable<T, C::Env>)
    where
        T: test::Entry<Env = C::Env>,
    {
        let mut t = t;
        for _i in 0..=100 {
            match t.run() {
                fsa::RunOutcome::Wait(w) => t = w.wait(),
                fsa::RunOutcome::Observe(mut o) => {
                    self.observe_and_reset(o.env());
                    t = o.relinquish().wait()
                }
            }
        }
    }

    fn observe_and_reset(&self, env: &mut C::Env) {
        // TODO(@MattWindsor91): handle poisoning here
        let mut g = self.observer.lock().unwrap();
        g.observe_and_reset(env, &self.checker);
    }
}

fn main() {
    run().unwrap();
}

fn run() -> err::Result<()> {
    let test = c::Test::load("test.dylib")?;
    run_with_entry(test.spawn())
}

fn run_with_entry<T: test::Entry>(entry: T) -> err::Result<()> {
    let checker = entry.checker();

    let fsa::Bundle { handles, manifest } = fsa::build(entry)?;
    let observer = obs::Observer::new(manifest);
    let mob = Arc::new(Mutex::new(observer));

    thread::scope(|s| {
        let mut joins: Vec<thread::ScopedJoinHandle<()>> = Vec::with_capacity(handles.len());

        for (i, t) in handles.into_iter().enumerate() {
            let builder = s.builder().name(format!("P{0}", i));
            let thrd = Thread::<T::Checker> {
                checker: checker.clone(),
                observer: mob.clone(),
            };
            let h = builder.spawn(move |_| thrd.run(t.start())).unwrap();
            joins.push(h)
        }

        // TODO(@MattWindsor91): the observations should only be visible from the environment once we've joined these threads
        // in general, all of the thread-unsafe stuff should be hidden inside the environment
        for h in joins.into_iter() {
            h.join().unwrap();
        }
    })
    .unwrap();

    if let Ok(m) = Arc::try_unwrap(mob) {
        for (k, v) in m.into_inner().unwrap().obs {
            println!(
                "{1} {2}> {0:?}",
                k,
                v.occurs,
                if v.check_result { "*" } else { ":" }
            );
        }
        // nb: this needs to percolate into an error if it fails.
    }

    Ok(())
}
