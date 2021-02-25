//! `clap` integration for config.

use std::{num::NonZeroUsize, str::FromStr};

use super::{check, err, iter, permute, sync, top};

pub mod arg {
    /// Name of the input file argument.
    pub const INPUT: &str = "INPUT";
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

/// Trait for things that can be updated from command line arguments taken from
/// `clap`.
pub trait Clappable<'a>: Sized {
    /// Merges configuration from a clap match dictionary into this, potentially
    /// replacing it entirely.
    fn parse_clap(self, matches: &'a clap::ArgMatches) -> err::Result<Self>;
}

/// We can fill a top-level config using clap.
impl<'a> Clappable<'a> for top::Config<'a> {
    fn parse_clap(self, matches: &'a clap::ArgMatches) -> err::Result<Self> {
        Ok(Self {
            input: matches.value_of(arg::INPUT).unwrap_or(self.input),
            check: self.check.parse_clap(matches)?,
            iter: self.iter.parse_clap(matches)?,
            sync: self.sync.parse_clap(matches)?,
            permute: self.permute.parse_clap(matches)?,
        })
    }
}

impl<'a> Clappable<'a> for check::Strategy {
    fn parse_clap(self, matches: &'a clap::ArgMatches) -> err::Result<Self> {
        parse_or(matches.value_of(arg::CHECK), self)
    }
}

/// We can fill an iteration strategy using clap.
impl<'a> Clappable<'a> for iter::Strategy {
    fn parse_clap(self, matches: &'a clap::ArgMatches) -> err::Result<Self> {
        let iterations = parse_or_else(matches.value_of(arg::ITERATIONS), || {
            as_usize(self.iterations())
        })
        .map_err(err::Error::BadIterationCount)?;
        let period = parse_or_else(matches.value_of(arg::PERIOD), || as_usize(self.period()))
            .map_err(err::Error::BadPeriod)?;

        Ok(iter::Strategy::from_ints(iterations, period))
    }
}

/// We can fill a thread permutation strategy using clap.
impl<'a> Clappable<'a> for permute::Strategy {
    fn parse_clap(self, matches: &'a clap::ArgMatches) -> err::Result<Self> {
        parse_or(matches.value_of(arg::PERMUTE), self)
    }
}

/// We can fill a sync strategy using clap.
impl<'a> Clappable<'a> for sync::Strategy {
    fn parse_clap(self, matches: &'a clap::ArgMatches) -> err::Result<Self> {
        parse_or(matches.value_of(arg::SYNC), self)
    }
}

fn as_usize(x: Option<NonZeroUsize>) -> usize {
    x.map_or(0, NonZeroUsize::get)
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
