extern crate crossbeam;
extern crate dlopen;
#[macro_use]
extern crate clap;
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

use clap::{App, Arg};
use test::Test;

fn main() {
    let phenolphalein = App::new("phenolphthalein")
        .author(crate_authors!())
        .version(crate_version!())
        .about("Concurrency test runner")
        .arg(
            Arg::with_name("INPUT")
                .help("The input file (.so, .dylib) to use")
                .required(true)
                .index(1),
        );

    let matches = phenolphalein.get_matches();

    let input = matches.value_of("INPUT").unwrap();

    run(input).unwrap();
}

fn run(input: &str) -> err::Result<()> {
    let test = c::Test::load(input)?;
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
