#[macro_use]
extern crate clap;

use std::io;

use phenolphthalein::{
    api::{abs::Test, c},
    config, model, run,
    ux::report::{Dumper, HistogramDumper},
};

use clap::{App, Arg};

fn main() {
    if let Err(e) = run(app().get_matches()) {
        eprintln!("{:#}", e);
        std::process::exit(1)
    }
}

fn app<'a, 'b>() -> App<'a, 'b> {
    App::new("phenolphthalein")
        .author(crate_authors!())
        .version(crate_version!())
        .about("Concurrency test runner")
        .arg(
            Arg::with_name(config::clap::arg::CHECK)
                .help("Checking strategy to use")
                .short("-c")
                .long("--check")
                .value_name("STRATEGY")
                .possible_values(config::check::string::ALL),
        )
        .arg(
            Arg::with_name(config::clap::arg::PERMUTE)
                .help("Permuting strategy to use")
                .short("-p")
                .long("--permute")
                .value_name("STRATEGY")
                .possible_values(config::permute::string::ALL),
        )
        .arg(
            Arg::with_name(config::clap::arg::SYNC)
                .help("Synchronisation strategy to use")
                .short("-s")
                .long("--sync")
                .value_name("STRATEGY")
                .possible_values(config::sync::string::ALL),
        )
        .arg(
            Arg::with_name(config::clap::arg::ITERATIONS)
                .help("Iterations to perform in total")
                .short("-i")
                .long("--iterations")
                .value_name("NUM"),
        )
        .arg(
            Arg::with_name(config::clap::arg::PERIOD)
                .help("rotate threads after each NUM iterations")
                .short("-p")
                .long("--period")
                .value_name("NUM"),
        )
        .arg(
            Arg::with_name(config::clap::arg::INPUT)
                .help("The input file (.so, .dylib) to use")
                .required(true)
                .index(1),
        )
}

fn run(matches: clap::ArgMatches) -> anyhow::Result<()> {
    use config::clap::Clappable;

    let config = config::Config::default().parse_clap(&matches)?;
    run_with_config(config)
}

fn run_with_config(config: config::Config) -> anyhow::Result<()> {
    let test = c::Test::load(config.input)?;

    let mut halt_rules: Vec<run::halt::Rule> = config.halt_rules().collect();
    halt_rules.push(setup_ctrlc()?);

    let sync = config.sync.to_factory();

    let report = run::Builder {
        halt_rules,
        sync,
        entry: test.spawn(),
        check: config.check.to_factory(),
        permuter: config.permute.to_permuter(),
    }
    .build()?
    .run()?;

    // TODO(@MattWindsor91): don't hardcode this
    dump_report(std::io::stdout(), report)
}

fn dump_report<W: io::Write>(w: W, r: model::obs::Report) -> anyhow::Result<()> {
    HistogramDumper {}.dump(w, r)?;
    Ok(())
}

fn setup_ctrlc() -> anyhow::Result<run::halt::Rule> {
    let (rule, callback) = run::halt::Rule::on_callback(run::halt::Type::Exit);
    ctrlc::set_handler(callback)?;
    Ok(rule)
}
