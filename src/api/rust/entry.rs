use super::env;
use crate::{api::abs, model};

/// Entry point for statically linked Rust tests.
///
/// Statically linked tests are mostly useful for internally testing
/// phenolphthalein, but could also be used by embedding the phenolphthalein
/// library into a test.  We don't (yet) directly have special support for
/// doing so, but it's theoretically possible.
#[derive(Clone)]
pub struct Static {
    pub manifest: model::Manifest,
    pub test: fn(tid: usize, env: &env::Env),
    pub check: Option<fn(env: &env::Env) -> model::Outcome>,
}

impl abs::Entry<'static> for Static {
    type Env = env::Env;

    fn make_manifest(&self) -> crate::err::Result<model::Manifest> {
        Ok(self.manifest.clone())
    }

    fn run(&self, tid: usize, e: &Self::Env) {
        (self.test)(tid, e)
    }

    fn checker(&self) -> Box<dyn abs::Checker<Self::Env>> {
        abs::check::make_optional(|f| Box::new(*f), &self.check)
    }
}
