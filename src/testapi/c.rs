//! Test-side support for the C ABI.
//!
//! This library exposes implementations of the various test primitives for
//! tests in C, or C-like languages.  The corresponding C support files are
//! in the same directory.

mod entry;
mod env;
mod manifest;

pub use entry::{Checker, Entry, Test};
