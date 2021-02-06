use crate::{run, run::halt};
use thiserror::Error;

/// Name of the input file argument.
pub const ARG_INPUT: &str = "INPUT";
/// Name of the `no-permute-threads` argument.
pub const ARG_NO_PERMUTE_THREADS: &str = "no_permute_threads";
/// Name of the `no-check` argument.
pub const ARG_NO_CHECK: &str = "no_check";
/// Name of the `sync` argument.
pub const ARG_SYNC: &str = "sync";
/// Name of the `iterations` argument.
pub const ARG_ITERATIONS: &str = "iterations";
/// Name of the `period` argument.
pub const ARG_PERIOD: &str = "period";

/// Name of the `Spinner` synchronisation method.
pub const SYNC_SPINNER: &str = "spinner";
/// Name of the `Barrier` synchronisation method.
pub const SYNC_BARRIER: &str = "barrier";

/// Names of each valid sync argument.
pub const SYNC_ALL: &[&str] = &[SYNC_SPINNER, SYNC_BARRIER];

/// A (semi-)parsed argument structure.
pub struct Args<'a> {
    /// The parsed input filename.
    pub input: &'a str,
    /// The parsed synchronisation method.
    pub sync: SyncMethod,
    /// The parsed iteration count.
    pub iterations: usize,
    /// The parsed thread swap period.
    pub period: usize,
    /// Whether state postcondition checks should be enabled.
    /// (This may or may not be a negative flag in the actual clap parser.)
    pub check: bool,
    /// Whether threads should be permuted.
    /// (This may or may not be a negative flag in the actual clap parser.)
    pub permute_threads: bool,
}

impl<'a> Args<'a> {
    /// Parses an argument set from a clap match dictionary.
    pub fn parse(matches: &'a clap::ArgMatches) -> Result<Self> {
        let input = matches.value_of(ARG_INPUT).unwrap();

        let nstr = matches.value_of(ARG_ITERATIONS).unwrap();
        let iterations = nstr.parse().map_err(Error::BadIterationCount)?;
        let pstr = matches.value_of(ARG_PERIOD).unwrap();
        let period = pstr.parse().map_err(Error::BadPeriod)?;

        let sstr = matches.value_of(ARG_SYNC).unwrap();
        let sync = sstr.parse()?;

        let check = !matches.is_present(ARG_NO_CHECK);
        let permute_threads = !matches.is_present(ARG_NO_PERMUTE_THREADS);

        Ok(Self {
            input,
            iterations,
            period,
            sync,
            check,
            permute_threads,
        })
    }

    /// Gets the halting rules requested in this argument set.
    pub fn halt_rules(&self) -> Vec<halt::Rule> {
        let mut v = Vec::with_capacity(3);
        if self.iterations != 0 {
            v.push(halt::Condition::EveryNIterations(self.iterations).exit())
        }
        if 0 < self.period && self.period < self.iterations {
            v.push(halt::Condition::EveryNIterations(self.period).rotate())
        }
        v
    }

    /// Gets the correct factory method for the synchronisation primitive
    /// requested in this argument set.
    pub fn sync_factory(&self) -> run::sync::Factory {
        match self.sync {
            SyncMethod::Barrier => run::sync::make_barrier,
            SyncMethod::Spinner => run::sync::make_spinner,
        }
    }
}

/// An argument-parsing error.
#[derive(Debug, Error)]
pub enum Error {
    /// The user supplied the given string, which was a bad sync method.
    #[error("unsupported sync method: {0}")]
    BadSyncMethod(String),
    /// The user supplied a bad iteration count.
    #[error("couldn't parse iteration count: {0}")]
    BadIterationCount(std::num::ParseIntError),
    /// The user supplied a bad period.
    #[error("couldn't parse period: {0}")]
    BadPeriod(std::num::ParseIntError),
}
type Result<T> = std::result::Result<T, Error>;

/// Enumeration of synchronisation methods exported by the phenolphthalein
/// toplevel.
pub enum SyncMethod {
    /// Represents the spinner synchronisation method.
    Spinner,
    /// Represents the barrier synchronisation method.
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
