//! C implementation of test entries and tests.
use dlopen::symbor::{Container, Ref, SymBorApi, Symbol};

use super::{env, manifest};
use crate::{api::abs, err, model};
use std::path;

/// Entry point for C-ABI tests coming from dynamically loaded libraries.
#[derive(SymBorApi, Clone)]
pub struct Entry<'a> {
    manifest: Ref<'a, manifest::Manifest>,

    test: Symbol<'a, unsafe extern "C" fn(tid: libc::size_t, env: *mut env::UnsafeEnv)>,
    check: Option<Symbol<'a, unsafe extern "C" fn(env: *const env::UnsafeEnv) -> bool>>,
}

/// A checker for C-ABI test environments.
#[derive(Clone)]
pub struct Checker<'a> {
    sym: Symbol<'a, unsafe extern "C" fn(env: *const env::UnsafeEnv) -> bool>,
}

impl<'a> abs::Checker<env::Env> for Checker<'a> {
    fn check(&self, e: &env::Env) -> model::Outcome {
        model::Outcome::from_pass_bool(unsafe { (self.sym)(e.p) })
    }
}

impl<'a> abs::Entry<'a> for Entry<'a> {
    type Env = env::Env;

    fn run(&self, tid: usize, e: &Self::Env) {
        unsafe { (self.test)(tid, e.p) }
    }

    fn make_manifest(&self) -> err::Result<model::manifest::Manifest> {
        self.manifest.to_manifest()
    }

    /// Gets a checker for this test.
    fn checker(&self) -> Box<dyn abs::Checker<Self::Env> + 'a> {
        if let Some(sym) = self.check {
            Box::new(Checker { sym })
        } else {
            Box::new(model::Outcome::Unknown)
        }
    }
}

/// A test that holds onto a dynamically loaded test library.
pub struct Test {
    c: Container<Entry<'static>>,
}

impl Test {
    /// Loads a test from a dynamic library at `file`.
    pub fn load(file: &path::Path) -> err::Result<Self> {
        let c = unsafe { Container::load(file) }?;
        // TODO(@MattWindsor91): perform basic safety checks.
        Ok(Test { c })
    }
}

impl<'a> abs::Test<'a> for Test {
    type Entry = Entry<'a>;

    fn spawn(&self) -> self::Entry<'a> {
        self.c.clone()
    }
}
