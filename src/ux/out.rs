//! Endpoints for outputting an observer's final state to the user.
//!
//! Generally, one will use [Choice] to make a selection (eg via command line)
//! of an [Outputter] to use, then instantiate it against a writer, then output
//! through it.

pub mod abs;
pub mod choice;
pub mod err;
pub mod histo;
pub mod json;

pub use abs::Outputter;
pub use choice::Choice;
