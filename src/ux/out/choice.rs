//! Enumerated selection of outputs, for use in command-line selection.

use super::{abs, err, histo, json};
use std::{io::Write, str::FromStr};

/// Enumeration of outputter choices.
///
/// This is not (yet) serialisable or deserialisable as it is not stored in
/// tester config.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Choice {
    /// Selects the histogram outputter.
    Histogram,
    /// Selects the JSON outputter.
    Json,
}

impl Choice {
    /// Constructs the appropriate outputter for the choice, using the given
    /// writer.
    pub fn to_outputter<'a, W: Write + 'a>(self, writer: W) -> Box<dyn abs::Outputter + 'a> {
        match self {
            Self::Histogram => Box::new(histo::Histogram::new(writer)),
            Self::Json => Box::new(json::Json::new(writer)),
        }
    }
}

/// Strings used when mapping outputter choicers to command-line arguments.
pub mod string {
    /// The string representation for the histogram outputter.
    pub const HISTOGRAM: &str = "histogram";
    /// The string representation for the JSON outputter.
    pub const JSON: &str = "json";

    // TODO(@MattWindsor91): test this lines up properly

    /// List of all possible string representations of outputter choices.
    pub const ALL: &[&str] = &[HISTOGRAM, JSON];
}

/// The default outputter is the histogram.
impl Default for Choice {
    fn default() -> Self {
        Self::Histogram
    }
}

impl FromStr for Choice {
    type Err = err::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let su = s.to_lowercase();
        match &*su {
            string::HISTOGRAM => Ok(Self::Histogram),
            string::JSON => Ok(Self::Json),
            _ => Err(Self::Err::BadOutputter(su)),
        }
    }
}
