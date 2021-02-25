//! The APIs used to communicate with concurrency tests.
pub mod abs;
pub mod c;
pub mod rust;

// Expose the abstract ABI more directly, as it'll be used a lot.
pub use abs::{Checker, Entry, Env, Test};
