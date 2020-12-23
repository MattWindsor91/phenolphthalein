use crate::{env::Env, err, manifest, obs};

/// Trait of cloneable entry points into tests.
pub trait Entry: Clone {
    /* NOTE(@MattWindsor91): this will likely need a lifetime adding to it
    eventually; I think its lack of one thus far is a quirk of how
    dlopen Containers manage lifetimes. */

    /// Every test entry has an associated environment type, which implements
    /// a fairly basic API for inspection and resetting.
    type Env: Env;

    /// Test entries must also have an associated checker type, for checking
    /// environments uphold test conditions.
    type Checker: obs::Checker<Env = Self::Env>;

    /// Makes a manifest using information taken from the test entry point.
    fn make_manifest(&self) -> err::Result<manifest::Manifest>;

    /// Runs the entry point given a thread ID and handle to the environment.
    fn run(&self, tid: usize, e: &mut Self::Env);

    /// Gets a checker for this entry point's environments.
    fn checker(&self) -> Self::Checker;
}

/// Trait of top-level tests.
///
/// Each test can spawn multiple entry points into itself.
pub trait Test<'a> {
    /// The type of entry point into the test.
    type Entry: Entry;

    /// Spawns a new entry point into the test.
    fn spawn(&self) -> Self::Entry;
}
