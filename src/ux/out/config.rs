//! Enumerated selection of outputs, for use in command-line selection.

use super::{abs, err, histo, json};
use crate::model::Report;
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
    pub fn into_outputter<'a, W: Write + 'a>(self, writer: W) -> Box<dyn abs::Outputter + 'a> {
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

/// A complete definition of how to select an output.
pub struct Config {
    /// The choice of outputter.
    pub choice: Choice,
    /// The choice of writer.
    pub writer: Box<dyn std::io::Write>,
}

impl Config {
    /// Constructs the appropriate outputter for the spec.
    #[must_use]
    pub fn into_outputter<'a>(self) -> Box<dyn abs::Outputter + 'a> {
        self.choice.into_outputter(self.writer)
    }
}

/// The default config uses the default outputter choice, and stdout.
impl Default for Config {
    fn default() -> Self {
        Config {
            choice: Choice::default(),
            writer: Box::new(std::io::stdout()),
        }
    }
}

/// Trait used to add inline outputter methods to reports.
pub trait Outputtable {
    /// Outputs this item onto the outputter chosen by `on`.
    /// `on`.
    ///
    /// # Errors
    ///
    /// Generally carries any errors caused by trying to `output` to the
    /// outputter given by `on`.
    fn output(self, on: Config) -> err::Result<()>;
}

impl Outputtable for Report {
    fn output(self, on: Config) -> err::Result<()> {
        on.into_outputter().output(self)
    }
}
