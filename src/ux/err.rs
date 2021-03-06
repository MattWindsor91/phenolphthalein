//! Errors that can occur during configuration.
use std::convert::Infallible;

use super::out;
use crate::config;
use thiserror::Error;

/// A configuration error.
#[derive(Debug, Error)]
pub enum Error {
    /// The user supplied bad tester configuration on the command line.
    #[error("config error")]
    Config(#[from] config::err::Error),

    /// Something went wrong with report output, or the configuration thereof.
    #[error("output error")]
    Output(#[from] out::err::Error),

    /// We expected a test, but none was given.
    #[error("no input test given")]
    NoInput,
}

impl From<Infallible> for Error {
    fn from(i: Infallible) -> Self {
        match i {}
    }
}

/// Results over [Error].
pub type Result<T> = std::result::Result<T, Error>;
