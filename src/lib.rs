//! The phenolphthalein library top-level.

extern crate clap;
extern crate crossbeam;
extern crate dlopen;
#[macro_use]
extern crate dlopen_derive;
extern crate libc;
extern crate rand;

pub mod err;
pub mod model;
pub mod run;
pub mod testapi;
pub mod ux;
