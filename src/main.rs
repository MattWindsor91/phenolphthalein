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
mod run;
mod test;

use crossbeam::thread;
use fsa::Fsa;
use std::sync::{Arc, Mutex};
use test::Test;



fn main() {
    run().unwrap();
}

fn run() -> err::Result<()> {
    let test = c::Test::load("test.dylib")?;
    run_with_entry(test.spawn())
}

fn run_with_entry<T: test::Entry>(entry: T) -> err::Result<()> {
    let checker = entry.checker();

    let fsa::Bundle { automata, manifest } = fsa::Bundle::new(entry)?;
    let observer = obs::Observer::new(manifest);
    let shin = run::SharedState{
        conds: run::ExitCondition::ExitOnNIterations(1000),
        observer,
        checker
    };
    let shared = Arc::new(Mutex::new(shin));

    thread::scope(|s| {
        automata.run(
            |r: fsa::Ready<T, T::Env>| {
                let builder = s.builder().name(format!("P{0}", r.tid()));
                let thrd = run::Thread::<T::Checker> {
                    shared: shared.clone(),
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

    if let Ok(s) = Arc::try_unwrap(shared) {
        for (k, v) in s.into_inner().unwrap().observer.obs {
            println!(
                "{1} {2}> {0:?}",
                k,
                v.occurs,
                match v.check_result {
                    obs::CheckResult::Passed => "*",
                    obs::CheckResult::Failed => ":",
                }
            );
        }
        // nb: this needs to percolate into an error if it fails.
    }

    Ok(())
}
