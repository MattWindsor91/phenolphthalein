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
use fsa::Fsa;
use std::sync::{Arc, Mutex};
use test::Test;

struct Thread<C> {
    checker: C,
    observer: Arc<Mutex<obs::Observer>>,
}

impl<C: obs::Checker> Thread<C> {
    fn run<T>(&self, t: fsa::Runnable<T, C::Env>) -> fsa::Done
    where
        T: test::Entry<Env = C::Env>,
    {
        let mut t = t;
        loop {
            match t.run() {
                fsa::RunOutcome::Done(d) => return d,
                fsa::RunOutcome::Wait(w) => t = w.wait(),
                fsa::RunOutcome::Observe(mut o) => {
                    let (iter, _) = self.observe_and_reset(o.env());
                    let r = if 100 <= iter {
                        o.kill()
                    } else {
                        o.relinquish()
                    };
                    t = r.wait()
                }
            }
        }
    }

    fn observe_and_reset(&self, env: &mut C::Env) -> (usize, obs::Info) {
        // TODO(@MattWindsor91): handle poisoning here
        let mut g = self.observer.lock().unwrap();
        g.observe_and_reset(env, &self.checker)
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
        handles.run(
            |r: fsa::Ready<T, T::Env>| {
                let builder = s.builder().name(format!("P{0}", r.tid()));
                let thrd = Thread::<T::Checker> {
                    checker: checker.clone(),
                    observer: mob.clone(),
                };
                builder.spawn(move |_| thrd.run(r.start())).unwrap()
            },
            |h| {
                let x = h.join().unwrap();
                Ok(x)
            },
        )
    })
    .unwrap()?;

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
