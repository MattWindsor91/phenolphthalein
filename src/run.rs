//! The test runner itself, including controls over how it synchronises and
//! halts.
mod fsa;
pub mod halt;
mod instance;
mod obs;
pub mod permute;
pub mod runner;
mod shared;
pub mod sync;
mod thread;

pub use permute::Permuter;
pub use runner::{Builder, Runner};
