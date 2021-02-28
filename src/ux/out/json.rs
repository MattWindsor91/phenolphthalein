//! The JSON outputter.

use super::{abs::Outputter, err};
use crate::model;
use std::io::Write;

/// An outputter that dumps reports as JSON.
pub struct Json<W: Write> {
    /// The writer.
    writer: W,
}

impl<W: Write> Outputter for Json<W> {
    fn output(self: Box<Self>, report: model::Report) -> err::Result<()> {
        serde_json::to_writer_pretty(self.writer, &report)?;
        Ok(())
    }
}

impl<W: Write> Json<W> {
    /// Constructs a new JSON writer.
    pub fn new(writer: W) -> Self {
        Self { writer }
    }
}
