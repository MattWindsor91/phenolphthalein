//! Error types for outputting reports.
use thiserror::Error;

/// Enumeration of possible outputting errors.
#[derive(Debug, Error)]
pub enum Error {
    /// The user selected an outputter choice that doesn't exist.
    #[error("unknown outputter: {0}")]
    BadOutputter(String),

    /// A general I/O error.
    #[error("I/O error")]
    Io(#[from] std::io::Error),

    /// An error when converting a report to JSON.
    #[error("error outputting as JSON")]
    Json(#[from] serde_json::Error),
}

/// Shorthand for a result over [Error]s.
pub type Result<T> = std::result::Result<T, Error>;
