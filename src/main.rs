extern crate crossbeam;
extern crate dlopen;
#[macro_use]
extern crate dlopen_derive;
extern crate libc;

mod c;
mod env;
mod err;
mod manifest;
mod obs;
mod test;

use crossbeam::thread;
use std::sync::{Arc, Mutex};
use test::Entry;

struct Thread<C> {
    checker: C,
    observer: Arc<Mutex<obs::Observer>>,
}

impl<C> Thread<C>
where
    C: obs::Checker,
{
    fn run<T>(&self, t: test::RunnableTest<T, C::Env>)
    where
        T: test::Entry<Env = C::Env>,
        C::Env: env::AnEnv,
    {
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

    fn observe_and_reset(&self, env: &mut C::Env)
    where
        C::Env: env::AnEnv,
    {
        // TODO(@MattWindsor91): handle poisoning here
        let mut g = self.observer.lock().unwrap();
        g.observe_and_reset(env, &self.checker);
    }
}

fn main() {
    run().unwrap();
}

fn run() -> err::Result<()> {
    let lib = dlopen::symbor::Library::open("test.dylib")?;
    let test = c::load_test(&lib)?;
    run_with_test(test)
}

fn run_with_test(entry: c::CTestApi) -> err::Result<()> {
    let test::Bundle { handles, manifest } = test::build(entry.clone())?;
    let observer = obs::Observer::new(manifest);
    let mob = Arc::new(Mutex::new(observer));

    thread::scope(|s| {
        let mut joins: Vec<thread::ScopedJoinHandle<()>> = Vec::with_capacity(handles.len());

        for (i, t) in handles.into_iter().enumerate() {
            let builder = s.builder().name(format!("P{0}", i));
            let thrd = Thread::<c::CChecker> {
                checker: entry.checker(),
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
