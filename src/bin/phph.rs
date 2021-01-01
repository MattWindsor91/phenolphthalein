#[macro_use]
extern crate clap;
extern crate ctrlc;
extern crate dlopen;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use phenolphthalein::{
    err, run,
    testapi::{abs::Test, c},
    ux,
    ux::obs::Dumper,
};

use clap::{App, Arg};

fn main() {
    let phenolphalein = App::new("phenolphthalein")
        .author(crate_authors!())
        .version(crate_version!())
        .about("Concurrency test runner")
        .arg(
            Arg::with_name("no_permute_threads")
                .help("Don't permute thread assignments each period")
                .short("-P")
                .long("--no-permute-threads"),
        )
        .arg(
            Arg::with_name("sync")
                .help("Synchronisation method to use")
                .short("-s")
                .long("--sync")
                .value_name("METHOD")
                .default_value(ux::args::SYNC_SPINNER)
                .possible_values(ux::args::SYNC_ALL),
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
    run_with_args(ux::args::Args::parse(&matches)?)
}

fn run_with_args(args: ux::args::Args) -> Result<()> {
    let test = c::Test::load(args.input)?;

    let mut conds = args.conds();
    conds.push(setup_ctrlc()?);

    let sync = args.sync_factory();

    let runner = run::Runner {
        conds,
        sync,
        entry: test.spawn(),
        permute_threads: args.permute_threads,
    };
    let obs = runner.run()?;

    // TODO(@MattWindsor91): don't hardcode this
    Ok(ux::obs::HistogramDumper {}.dump(obs)?)
}

fn setup_ctrlc() -> Result<run::halt::Condition> {
    let sigb = Arc::new(AtomicBool::new(false));
    let c = run::halt::Condition::OnSignal(sigb.clone(), run::halt::Type::Exit);
    ctrlc::set_handler(move || sigb.store(true, Ordering::Release))?;
    Ok(c)
}

/// A top-level error.
#[derive(Debug)]
enum Error {
    /// The user supplied the given string, which was a bad sync method.
    ParsingArgs(ux::args::Error),
    /// Error running the test.
    RunningTest(err::Error),
    /// There was a problem installing the control-C interrupt handler.
    CtrlC(ctrlc::Error),
}
type Result<T> = std::result::Result<T, Error>;

impl From<ux::args::Error> for Error {
    fn from(e: ux::args::Error) -> Self {
        Self::ParsingArgs(e)
    }
}

impl From<err::Error> for Error {
    fn from(e: err::Error) -> Self {
        Self::RunningTest(e)
    }
}

impl From<ctrlc::Error> for Error {
    fn from(e: ctrlc::Error) -> Self {
        Self::CtrlC(e)
    }
}
