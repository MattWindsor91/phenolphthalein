use super::{check, err, iter, permute, sync};
use crate::run::halt;
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
/// The top-level config structure.
pub struct Config<'a> {
    /// The input filename.
    pub input: &'a str,
    /// The strategy for thread permutation that the runner should take.
    pub permute: permute::Strategy,
    /// The synchronisation strategy.
    pub sync: sync::Strategy,
    /// The strategy for checking that the runner should take.
    pub check: check::Strategy,
    /// The test iteration strategy.
    pub iter: iter::Strategy,
}

impl<'a> Config<'a> {
    /// Gets the halting rules requested in this argument set.
    pub fn halt_rules(&self) -> impl Iterator<Item = halt::Rule> {
        let i_rules = self.iter.halt_rules();
        let c_rules = self.check.halt_rules();
        i_rules.chain(c_rules)
    }

    /// Tries to dump a config to a string.
    pub fn to_string(&self) -> err::Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }

    /// Tries to load a config from a string.
    pub fn from_str(s: &'a str) -> err::Result<Self> {
        Ok(toml::from_str(s)?)
    }
}
