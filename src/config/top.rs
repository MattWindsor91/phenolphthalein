use super::{check, iter, permute, sync};
use crate::run::halt;

#[derive(Default)]
/// The top-level config structure.
pub struct Config<'a> {
    /// The input filename.
    pub input: &'a str,
    /// The strategy for checking that the runner should take.
    pub check: check::Strategy,
    /// The test iteration strategy.
    pub iter: iter::Strategy,
    /// The strategy for thread permutation that the runner should take.
    pub permute: permute::Strategy,
    /// The synchronisation strategy.
    pub sync: sync::Strategy,
}

impl<'a> Config<'a> {
    /// Gets the halting rules requested in this argument set.
    pub fn halt_rules(&self) -> impl Iterator<Item = halt::Rule> {
        let i_rules = self.iter.halt_rules();
        let c_rules = self.check.halt_rules();
        i_rules.chain(c_rules)
    }
}
