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

use test::Test;

fn main() {
    run().unwrap();
}

fn run() -> err::Result<()> {
    let test = c::Test::load("test.dylib")?;
    let runner = run::Runner {
        conds: run::ExitCondition::ExitOnNIterations(1000),
    };
    let obs = runner.run(test.spawn())?;
    print_obs(obs);

    Ok(())
}

fn print_obs(observer: obs::Observer) {
    for (k, v) in observer.obs {
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
}
