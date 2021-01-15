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
        self.dump_obs(o.obs);
        Ok(())
    }
}

impl HistogramDumper {
    fn dump_obs(&self, obs: model::obs::Set) {
        let pad = padding(&obs);

        for (state, v) in obs {
            println!(
                "{occ:pad$} {sigil}> {state:?} (iter {iter})",
                occ = v.occurs,
                pad = pad,
                sigil = self.check_sigil(v.check_result),
                state = state,
                iter = v.iteration,
            );
        }
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
