#[macro_use]
extern crate clap;

use std::io;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use phenolphthalein::{
    model, run,
    testapi::{abs::Test, c},
    ux,
    ux::report::Dumper,
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
    if let Err(e) = run(matches) {
        eprintln!("{:#}", e);
        std::process::exit(1)
    }
}

fn run(matches: clap::ArgMatches) -> anyhow::Result<()> {
    run_with_args(ux::args::Args::parse(&matches)?)
}

fn run_with_args(args: ux::args::Args) -> anyhow::Result<()> {
    let test = c::Test::load(args.input)?;

    let mut conds = args.conds();
    conds.push(setup_ctrlc()?);

    let sync = args.sync_factory();

    let mut runner = run::Builder {
        conds,
        sync,
        entry: test.spawn(),
        permute_threads: args.permute_threads,
    }
    .build()?;
    runner.run()?;

    // TODO(@MattWindsor91): don't hardcode this
    dump_report(std::io::stdout(), runner.into_report()?)
}

fn dump_report<W: io::Write>(w: W, r: model::obs::Report) -> anyhow::Result<()> {
    ux::report::HistogramDumper {}.dump(w, r)?;
    Ok(())
}

fn setup_ctrlc() -> anyhow::Result<run::halt::Condition> {
    let sigb = Arc::new(AtomicBool::new(false));
    let c = run::halt::Condition::OnSignal(sigb.clone(), run::halt::Type::Exit);
    ctrlc::set_handler(move || sigb.store(true, Ordering::Release))?;
    Ok(c)
}
