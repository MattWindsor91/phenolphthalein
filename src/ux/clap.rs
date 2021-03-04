//! `clap` integration for config.

use std::{num::NonZeroUsize, path, str::FromStr};

use super::{err, out};
use crate::config::{self, check, io, iter, permute, sync, Config};

/// Clap names for various arguments.
pub mod arg {
    /// Name of the input file argument.
    pub const INPUT: &str = "INPUT";

    /// Name of the output type file argument.
    pub const OUTPUT_TYPE: &str = "input-type";

    /// Name of the dump-config argument.
    pub const DUMP_CONFIG: &str = "dump-config";
    /// Name of the dump-config-path argument.
    pub const DUMP_CONFIG_PATH: &str = "dump-config-path";
    /// Name of the config argument.
    pub const CONFIG: &str = "config";
    /// Name of the `check` argument.
    pub const CHECK: &str = "check";
    /// Name of the `permute` argument.
    pub const PERMUTE: &str = "permute";
    /// Name of the `sync` argument.
    pub const SYNC: &str = "sync";

    /// Name of the `iterations` argument.
    pub const ITERATIONS: &str = "iterations";
    /// Name of the `period` argument.
    pub const PERIOD: &str = "period";
}

/// Gets the config file mentioned on the command line, or the default file if
/// no such file was named.
pub fn config_file(matches: &clap::ArgMatches) -> anyhow::Result<path::PathBuf> {
    matches
        .value_of(arg::CONFIG)
        .map(|x| Ok(x.parse()?))
        .unwrap_or_else(|| Ok(io::default_file()))
}

/// Trait for things that can be updated from command line arguments taken from
/// `clap`.
pub trait Clappable: Sized {
    /// Merges configuration from a clap match dictionary into this, potentially
    /// replacing it entirely.
    fn parse_clap(self, matches: &clap::ArgMatches) -> err::Result<Self>;
}

/// We can fill a top-level config using clap.
impl Clappable for Config {
    fn parse_clap(self, matches: &clap::ArgMatches) -> err::Result<Self> {
        Ok(Self {
            check: self.check.parse_clap(matches)?,
            iter: self.iter.parse_clap(matches)?,
            sync: self.sync.parse_clap(matches)?,
            permute: self.permute.parse_clap(matches)?,
        })
    }
}

impl Clappable for check::Strategy {
    fn parse_clap(self, matches: &clap::ArgMatches) -> err::Result<Self> {
        Ok(parse_or(matches.value_of(arg::CHECK), self)?)
    }
}

/// We can fill an iteration strategy using clap.
impl Clappable for iter::Strategy {
    fn parse_clap(self, matches: &clap::ArgMatches) -> err::Result<Self> {
        let iterations = parse_or_else(matches.value_of(arg::ITERATIONS), || {
            as_usize(self.iterations())
        })
        .map_err(config::Error::BadIterationCount)?;
        let period = parse_or_else(matches.value_of(arg::PERIOD), || as_usize(self.period()))
            .map_err(config::Error::BadPeriod)?;

        Ok(iter::Strategy::from_ints(iterations, period))
    }
}

/// We can fill a thread permutation strategy using clap.
impl Clappable for permute::Strategy {
    fn parse_clap(self, matches: &clap::ArgMatches) -> err::Result<Self> {
        Ok(parse_or(matches.value_of(arg::PERMUTE), self)?)
    }
}

/// We can fill a sync strategy using clap.
impl Clappable for sync::Strategy {
    fn parse_clap(self, matches: &clap::ArgMatches) -> err::Result<Self> {
        Ok(parse_or(matches.value_of(arg::SYNC), self)?)
    }
}

/// We can fill an output choice using clap.
impl Clappable for out::Choice {
    fn parse_clap(self, matches: &clap::ArgMatches) -> err::Result<Self> {
        Ok(parse_or(matches.value_of(arg::OUTPUT_TYPE), self)?)
    }
}

/// We can fill an output config using clap.
impl Clappable for out::Config {
    fn parse_clap(mut self, matches: &clap::ArgMatches) -> err::Result<Self> {
        self.choice = self.choice.parse_clap(matches)?;
        // TODO(@MattWindsor91): outputs other than stdout
        Ok(self)
    }
}

fn as_usize(x: Option<NonZeroUsize>) -> usize {
    x.map_or(0, NonZeroUsize::get)
}

/// Parses a `T` from clap matches, or supplies the default.
fn clap_or_default<T: Default + Clappable>(matches: &clap::ArgMatches) -> err::Result<T> {
    T::default().parse_clap(matches)
}

fn parse_or<T: FromStr>(int_str: Option<&str>, default: T) -> std::result::Result<T, T::Err> {
    int_str.map_or(Ok(default), |s| s.parse())
}

fn parse_or_else<T: FromStr>(
    int_str: Option<&str>,
    default: impl FnOnce() -> T,
) -> std::result::Result<T, T::Err> {
    int_str.map_or_else(|| Ok(default()), |s| s.parse())
}

/// Actions that can be specified on the command line.
pub enum Action {
    /// Asks to run the test with a given path.
    RunTest(path::PathBuf, out::Config),
    /// Asks to dump the config.
    DumpConfig,
    /// Asks to dump the path to the config.
    DumpConfigPath,
}

impl Clappable for Action {
    fn parse_clap(self, matches: &clap::ArgMatches) -> err::Result<Self> {
        Ok(if matches.is_present(arg::DUMP_CONFIG) {
            Self::DumpConfig
        } else if matches.is_present(arg::DUMP_CONFIG_PATH) {
            Self::DumpConfigPath
        } else {
            let input = matches.value_of(arg::INPUT).ok_or(err::Error::NoInput)?;
            Self::RunTest(input.parse()?, clap_or_default(matches)?)
        })
    }
}
