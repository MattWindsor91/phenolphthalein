//! The histogram outputter.

use super::{abs::Outputter, err};
use crate::model::{
    self,
    report::{Report, State},
};
use colored::Colorize;
use std::{
    collections::BTreeMap,
    io::{self, Write},
};

/// An outputter that provides Litmus-style histograms.
pub struct Histogram<W> {
    w: tabwriter::TabWriter<W>,
}

impl<W: Write> Outputter for Histogram<W> {
    fn output(mut self: Box<Self>, report: Report) -> err::Result<()> {
        self.dump_states(report.states)?;
        self.w.flush()?;
        Ok(())
    }
}

impl<W: Write> Histogram<W> {
    /// Constructs a new histogram writer.
    pub fn new(writer: W) -> Self {
        Self {
            w: tabwriter::TabWriter::new(writer).padding(1),
        }
    }

    fn dump_states(&mut self, states: std::vec::Vec<State>) -> err::Result<()> {
        for state in states {
            self.dump_state(state)?;
        }
        Ok(())
    }

    fn dump_state(&mut self, State { state, info }: State) -> io::Result<()> {
        Ok(writeln!(
            self.w,
            "{occ}\t{sigil}>\t{state}\t(iter {iter})",
            occ = info.occurs,
            sigil = self.check_sigil(info.outcome),
            state = stringify_valuation(state),
            iter = info.iteration,
        )?)
    }

    fn check_sigil(&self, r: model::Outcome) -> colored::ColoredString {
        match r {
            model::Outcome::Pass => "*".green(),
            model::Outcome::Fail => ":".red(),
            model::Outcome::Unknown => "?".yellow(),
        }
    }
}

/// Converts a state valuation to a stirng.
fn stringify_valuation(valuation: BTreeMap<String, model::state::Value>) -> String {
    /* TODO(@MattWindsor91): this should really be a Display impl, but
    valuations have no defined type off which to hang it. */
    valuation
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("\t")
}
