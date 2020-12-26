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

const SYNC_SPINNER: &str = "spinner";
const SYNC_BARRIER: &str = "barrier";
const SYNC_ALL: &[&str] = &[SYNC_SPINNER, SYNC_BARRIER];

enum SyncMethod {
    Spinner,
    Barrier,
}

impl std::str::FromStr for SyncMethod {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            SYNC_SPINNER => Ok(Self::Spinner),
            SYNC_BARRIER => Ok(Self::Barrier),
            s => Err(Error::BadSyncMethod(s.to_owned())),
        }
    }
}

fn main() {
    let phenolphalein = App::new("phenolphthalein")
        .author(crate_authors!())
        .version(crate_version!())
        .about("Concurrency test runner")
        .arg(
            Arg::with_name("sync")
                .help("Synchronisation method to use")
                .short("-s")
                .long("--sync")
                .value_name("METHOD")
                .default_value(SYNC_SPINNER)
                .possible_values(SYNC_ALL),
        )
        .arg(
            Arg::with_name("iterations")
                .help("Iterations to perform in total")
                .short("-i")
                .long("--iterations")
                .value_name("NUM")
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

    let conds = args.conds();
    let sync = args.sync_factory();

    let runner = run::Runner { conds, sync };
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

struct Args<'a> {
    input: &'a str,
    sync: SyncMethod,
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

        let sstr = matches.value_of("sync").unwrap();
        let sync = sstr.parse()?;

        Ok(Self {
            input,
            iterations,
            period,
            sync,
        })
    }

    fn conds(&self) -> Vec<run::Condition> {
        let mut v = Vec::with_capacity(2);
        if self.iterations != 0 {
            v.push(run::Condition::EveryNIterations(
                self.iterations,
                fsa::ExitType::Exit,
            ))
        }
        if self.period != 0 {
            v.push(run::Condition::EveryNIterations(
                self.period,
                fsa::ExitType::Rotate,
            ))
        }
        v
    }

    fn sync_factory(&self) -> fsa::sync::Factory {
        match self.sync {
            SyncMethod::Barrier => fsa::sync::make_barrier,
            SyncMethod::Spinner => fsa::sync::make_spinner,
        }
    }
}

#[derive(Debug)]
enum Error {
    /// The user supplied the given string, which was a bad sync method.
    BadSyncMethod(String),

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
