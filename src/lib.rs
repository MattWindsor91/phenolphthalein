//! The phenolphthalein library top-level.
#![warn(clippy::all, clippy::pedantic)]

#[macro_use]
extern crate dlopen_derive;

pub mod api;
pub mod config;
pub mod err;
pub mod model;
pub mod run;
pub mod ux;
