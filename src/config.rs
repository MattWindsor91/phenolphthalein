//! Top-level configuration for the tester.
//!
//! Individual parts of phenokphthalein can be used without pulling in this
//! configuration layer, but it provides a convenient substrate for handling
//! the configuration.

pub mod check;
pub mod clap;
pub mod err;
pub mod iter;
pub mod permute;
pub mod sync;
pub mod top;

pub use top::Config;
