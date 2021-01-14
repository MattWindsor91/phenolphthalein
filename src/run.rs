//! The test runner itself, including controls over how it synchronises and
//! halts.
mod fsa;
pub mod halt;
mod obs;
pub mod runner;
mod shared;
pub mod sync;
mod thread;

pub use runner::Runner;
