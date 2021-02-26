//! Errors that can occur during configuration.
use thiserror::Error;

/// A configuration error.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    /// The user supplied the given string, which was a bad check strategy.
    #[error("unsupported checking strategy: {0}")]
    BadCheckStrategy(String),
    /// The user supplied a bad outcome for the exit-on check strategy.
    #[error("couldn't parse outcome in 'exit-on' checking strategy: {0}")]
    BadCheckOutcome(String),

    /// The user supplied the given string, which was a bad permute strategy.
    #[error("unsupported thread permutation strategy: {0}")]
    BadPermuteStrategy(String),

    /// The user supplied the given string, which was a bad sync strategy.
    #[error("unsupported synchronisation strategy: {0}")]
    BadSyncStrategy(String),

    /// The user supplied a bad iteration count.
    #[error("couldn't parse iteration count: {0}")]
    BadIterationCount(std::num::ParseIntError),
    /// The user supplied a bad period.
    #[error("couldn't parse period: {0}")]
    BadPeriod(std::num::ParseIntError),

    /// We couldn't deserialise the config from TOML.
    #[error("couldn't parse config: {0}")]
    Deserialize(#[from] toml::de::Error),

    /// We couldn't serialise the config to TOML.
    #[error("couldn't dump config: {0}")]
    Serialize(#[from] toml::ser::Error),
}

/// Results over [Error].
pub type Result<T> = std::result::Result<T, Error>;
