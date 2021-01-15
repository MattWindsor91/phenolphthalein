//! Endpoints for outputting an observer's final state to the user.

use crate::{err, model};

// TODO(@MattWindsor91): separate observer state and builder.

/// Traits for things that dump final observer reports.
pub trait Dumper {
    fn dump(&self, o: model::obs::Report) -> err::Result<()>;
}

/// A dumper that provides Litmus-style histograms.
pub struct HistogramDumper {}

impl Dumper for HistogramDumper {
    fn dump(&self, o: model::obs::Report) -> err::Result<()> {
        self.dump_set(o.obs);
        Ok(())
    }
}

impl HistogramDumper {
    fn dump_set(&self, set: model::obs::Set) {
        let pad = padding(&set);
        for (state, obs) in set {
            self.dump_state(state, obs, pad)
        }
    }

    fn dump_state(&self, state: model::obs::State, obs: model::obs::Obs, pad: usize) {
        println!(
            "{occ:pad$} {sigil}> {state:?} (iter {iter})",
            occ = obs.occurs,
            pad = pad,
            sigil = self.check_sigil(obs.check_result),
            state = state,
            iter = obs.iteration,
        );
    }

    fn check_sigil(&self, r: model::check::Outcome) -> &'static str {
        match r {
            model::check::Outcome::Passed => "*",
            model::check::Outcome::Failed => ":",
        }
    }
}

fn padding(obs: &model::obs::Set) -> usize {
    obs.iter()
        .map(|(_, v)| v.occurs.to_string().len())
        .max()
        .unwrap_or_default()
}
