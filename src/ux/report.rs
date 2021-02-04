//! Endpoints for outputting an observer's final state to the user.

use crate::model;
use colored::Colorize;
use std::{io, io::Write};
use tabwriter::TabWriter;

/// Traits for things that dump final observer reports.
pub trait Dumper {
    /// Dumps the report `r` into `W`, flushing and returning any I/O errors arising.
    fn dump<W: io::Write>(&self, w: W, r: model::obs::Report) -> io::Result<W>;
}

/// A dumper that provides Litmus-style histograms.
pub struct HistogramDumper {}

impl Dumper for HistogramDumper {
    fn dump<W: io::Write>(&self, w: W, o: model::obs::Report) -> io::Result<W> {
        let w = self.dump_set(w, o.obs)?;
        Ok(w)
    }
}

impl HistogramDumper {
    fn dump_set<W: io::Write>(&self, w: W, set: model::obs::Set) -> io::Result<W> {
        let mut tw = tabwriter::TabWriter::new(w).padding(1);

        for (state, obs) in set {
            self.dump_state(&mut tw, state, obs)?;
        }

        tw.into_inner().map_err(|w| try_clone_error(w.error()))
    }

    fn dump_state(
        &self,
        tw: &mut TabWriter<impl io::Write>,
        state: model::obs::State,
        obs: model::obs::Obs,
    ) -> io::Result<()> {
        let state_str = state
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("\t");
        Ok(writeln!(
            tw,
            "{occ}\t{sigil}>\t{state}\t(iter {iter})",
            occ = obs.occurs,
            sigil = self.check_sigil(obs.check_result),
            state = state_str,
            iter = obs.iteration,
        )?)
    }

    fn check_sigil(&self, r: model::check::Outcome) -> colored::ColoredString {
        match r {
            model::check::Outcome::Passed => "*".green(),
            model::check::Outcome::Failed => ":".red(),
            model::check::Outcome::Unknown => "?".yellow(),
        }
    }
}

/// Tries to copy as much of `e` as possible into a new error.
fn try_clone_error(e: &io::Error) -> io::Error {
    let kind = e.kind();
    if let Some(os_err) = e.raw_os_error() {
        io::Error::from_raw_os_error(os_err)
    } else {
        io::Error::from(kind)
    }
}
