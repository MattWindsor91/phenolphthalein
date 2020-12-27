extern crate dlopen;
#[macro_use]
extern crate clap;

use phenolphthalein::{
    err, model, run,
    testapi::{abs::Test, c},
    ux,
};

use clap::{App, Arg};

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

    let conds = args.conds();
    let sync = args.sync_factory();

    let runner = run::Runner {
        conds,
        sync,
        entry: test.spawn(),
    };
    let obs = runner.run()?;
    print_obs(obs);

    Ok(())
}

fn print_obs(observer: run::obs::Observer) {
    for (k, v) in observer.obs {
        println!(
            "{1} {2}> {0:?}",
            k,
            v.occurs,
            match v.check_result {
                model::check::Outcome::Passed => "*",
                model::check::Outcome::Failed => ":",
            }
        );
    }
}

/// A top-level error.
#[derive(Debug)]
enum Error {
    /// The user supplied the given string, which was a bad sync method.
    ParsingArgs(ux::args::Error),
    /// Error running the test.
    RunningTest(err::Error),
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
