use crate::{run, run::halt};
use thiserror::Error;

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
    /// Whether threads should be permuted.
    /// (This may or may not be a negative flag in the actual clap parser.)
    pub permute_threads: bool,
}

impl<'a> Args<'a> {
    /// Parses an argument set from a clap match dictionary.
    pub fn parse(matches: &'a clap::ArgMatches) -> Result<Self> {
        let input = matches.value_of("INPUT").unwrap();
        // For now
        let nstr = matches.value_of("iterations").unwrap();
        let iterations = nstr.parse().map_err(Error::BadIterationCount)?;
        let period = nstr.parse().map_err(Error::BadPeriod)?;

        let sstr = matches.value_of("sync").unwrap();
        let sync = sstr.parse()?;

        let permute_threads = !matches.is_present("no_permute_threads");

        Ok(Self {
            input,
            iterations,
            period,
            sync,
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
