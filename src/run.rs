//! The test runner itself, including controls over how it synchronises and
//! halts.
mod fsa;
pub mod halt;
mod obs;
pub mod permute;
pub mod runner;
mod shared;
pub mod sync;

pub use permute::Permuter;
pub use runner::{Builder, Runner};
