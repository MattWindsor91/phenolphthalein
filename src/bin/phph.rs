#[macro_use]
extern crate clap;

use std::{fs::File, io::Read, iter::once, path, str::FromStr};

use phenolphthalein::{
    api::{self, abs::Test, c},
    config, model, run,
    ux::{self, out::Outputtable},
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
            Arg::with_name(ux::clap::arg::CHECK)
                .help("Checking strategy to use")
                .short("-c")
                .long("--check")
                .value_name("STRATEGY")
                .possible_values(config::check::string::ALL),
        )
        .arg(
            Arg::with_name(ux::clap::arg::PERMUTE)
                .help("Permuting strategy to use")
                .short("-p")
                .long("--permute")
                .value_name("STRATEGY")
                .possible_values(config::permute::string::ALL),
        )
        .arg(
            Arg::with_name(ux::clap::arg::SYNC)
                .help("Synchronisation strategy to use")
                .short("-s")
                .long("--sync")
                .value_name("STRATEGY")
                .possible_values(config::sync::string::ALL),
        )
        .arg(
            Arg::with_name(ux::clap::arg::ITERATIONS)
                .help("Iterations to perform in total")
                .short("-i")
                .long("--iterations")
                .value_name("NUM"),
        )
        .arg(
            Arg::with_name(ux::clap::arg::PERIOD)
                .help("rotate threads after each NUM iterations")
                .short("-p")
                .long("--period")
                .value_name("NUM"),
        )
        .arg(
            Arg::with_name(ux::clap::arg::CONFIG)
                .help("Load config from this file")
                .long("--config")
                .short("-c"),
        )
        .arg(
            Arg::with_name(ux::clap::arg::DUMP_CONFIG)
                .help("Dump config instead of testing")
                .long("--dump-config"),
        )
        .arg(
            Arg::with_name(ux::clap::arg::DUMP_CONFIG_PATH)
                .help("Dump config path instead of testing")
                .conflicts_with(ux::clap::arg::DUMP_CONFIG)
                .long("--dump-config-path"),
        )
        .arg(
            Arg::with_name(ux::clap::arg::OUTPUT_TYPE)
                .help("Type of output to return")
                .long("--output-type")
                .short("-O")
                .value_name("TYPE")
                .possible_values(ux::out::config::string::ALL),
        )
        .arg(
            Arg::with_name(ux::clap::arg::INPUT)
                .help("The input file (.so, .dylib) to use")
                .conflicts_with_all(&[ux::clap::arg::DUMP_CONFIG, ux::clap::arg::DUMP_CONFIG_PATH])
                .index(1),
        )
}

fn run(matches: clap::ArgMatches) -> anyhow::Result<()> {
    use ux::clap::{Action, Clappable};

    let cpath = ux::clap::config_file(&matches)?;
    let config = load_config(&cpath)?.parse_clap(&matches)?;

    match ux::clap::Action::DumpConfig.parse_clap(&matches)? {
        Action::DumpConfig => config.dump()?,
        Action::DumpConfigPath => dump_config_path(&cpath),
        Action::RunTest(path, outputter) => run_test(config, &path)?.output(outputter)?,
    };
    Ok(())
}

fn load_config(path: &path::Path) -> anyhow::Result<config::Config> {
    if path.exists() {
        let mut buf = String::new();
        let _ = File::open(path)?.read_to_string(&mut buf)?;
        Ok(config::Config::from_str(&buf)?)
    } else {
        Ok(config::Config::default())
    }
}

fn dump_config_path(path: &path::Path) {
    println!("{}", path.to_string_lossy())
}

fn run_test(config: config::Config, input: &path::Path) -> anyhow::Result<model::Report> {
    let test = c::Test::load(input)?;
    Ok(run_entry(config, test.spawn())?)
}

fn run_entry<'a, E: api::abs::Entry<'a>>(
    config: config::Config,
    entry: E,
) -> anyhow::Result<model::Report> {
    Ok(run::Builder::new(entry)
        .add_halt_rules(config.halt_rules().chain(once(setup_ctrlc()?)))
        .with_checker(config.check.to_factory())
        .with_permuter(config.permute.to_factory())
        .with_sync(config.sync.to_factory())
        .build()?
        .run()?)
}

/// Creates a halt rule that exits the test if control-C is sent.
fn setup_ctrlc() -> anyhow::Result<run::halt::Rule> {
    let (rule, callback) = run::halt::Rule::on_callback(run::halt::Type::Exit);
    ctrlc::set_handler(callback)?;
    Ok(rule)
}
