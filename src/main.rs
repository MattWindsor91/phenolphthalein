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
            Arg::with_name("iterations")
                .help("Iterations to perform in total")
                .short("-i")
                .long("--iterations")
                .value_name("NUM")
                .takes_value(true)
                .default_value("100000"),
        )
        .arg(
            Arg::with_name("period")
                .help("rotate threads after each NUM iterations")
                .short("-p")
                .long("--period")
                .value_name("NUM")
                .takes_value(true)
                .default_value("10000"),
        )
        .arg(
            Arg::with_name("INPUT")
                .help("The input file (.so, .dylib) to use")
                .required(true)
                .index(1),
        );

    let matches = phenolphalein.get_matches();
    run(matches).unwrap();
}

fn run(matches: clap::ArgMatches) -> Result<()> {
    run_with_args(Args::parse(&matches)?)
}

fn run_with_args(args: Args) -> Result<()> {
    let test = c::Test::load(args.input)?;

    let conds = build_conds(args);

    let runner = run::Runner { conds };
    let obs = runner.run(test.spawn())?;
    print_obs(obs);

    Ok(())
}

fn build_conds(args: Args) -> Vec<run::Condition> {
    let mut v = Vec::with_capacity(2);
    if args.iterations != 0 {
        v.push(run::Condition::EveryNIterations(
            args.iterations,
            fsa::ExitType::Exit,
        ))
    }
    if args.period != 0 {
        v.push(run::Condition::EveryNIterations(
            args.period,
            fsa::ExitType::Rotate,
        ))
    }
    v
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

struct Args<'a> {
    input: &'a str,
    iterations: usize,
    period: usize,
}

impl<'a> Args<'a> {
    fn parse(matches: &'a clap::ArgMatches) -> Result<Self> {
        let input = matches.value_of("INPUT").unwrap();
        // For now
        let nstr = matches.value_of("iterations").unwrap();
        let iterations = nstr.parse().map_err(Error::BadIterationCount)?;
        let period = nstr.parse().map_err(Error::BadParseCount)?;

        Ok(Self {
            input,
            iterations,
            period,
        })
    }
}

#[derive(Debug)]
enum Error {
    /// The user supplied a bad iteration count.
    BadIterationCount(std::num::ParseIntError),
    /// The user supplied a bad parse count.
    BadParseCount(std::num::ParseIntError),
    /// Error running the test.
    RunningTest(err::Error),
}
type Result<T> = std::result::Result<T, Error>;

impl From<err::Error> for Error {
    fn from(e: err::Error) -> Self {
        Self::RunningTest(e)
    }
}
