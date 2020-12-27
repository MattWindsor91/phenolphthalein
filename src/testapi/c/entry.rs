use dlopen::symbor::{Container, Ref, SymBorApi, Symbol};

use super::{env, manifest};
use crate::{err, manifest as m, obs, testapi::abs};

/// Entry point for C-ABI tests coming from dynamically loaded libraries.
#[derive(SymBorApi, Clone)]
pub struct Entry<'a> {
    manifest: Ref<'a, manifest::Manifest>,

    test: Symbol<'a, unsafe extern "C" fn(tid: libc::size_t, env: *mut env::UnsafeEnv)>,
    check: Symbol<'a, unsafe extern "C" fn(env: *const env::UnsafeEnv) -> bool>,
}

/// A checker for C-ABI test environments.
#[derive(Clone)]
pub struct Checker<'a> {
    sym: Symbol<'a, unsafe extern "C" fn(env: *const env::UnsafeEnv) -> bool>,
}

impl<'a> obs::Checker for Checker<'a> {
    type Env = env::Env;

    fn check(&self, e: &Self::Env) -> obs::CheckResult {
        if unsafe { (self.sym)(e.p) } {
            obs::CheckResult::Passed
        } else {
            obs::CheckResult::Failed
        }
    }
}

impl<'a> abs::Entry for Entry<'a> {
    type Env = env::Env;
    type Checker = Checker<'a>;

    fn run(&self, tid: usize, e: &mut Self::Env) {
        unsafe { (self.test)(tid, e.p) }
    }

    fn make_manifest(&self) -> err::Result<m::Manifest> {
        self.manifest.to_manifest()
    }

    /// Gets a checker for this test.
    fn checker(&self) -> Self::Checker {
        Checker { sym: self.check }
    }
}

/// A test that holds onto a dynamically loaded test library.
pub struct Test {
    c: Container<Entry<'static>>,
}

impl Test {
    /// Loads a test from a dynamic library at `file`.
    pub fn load(file: &str) -> err::Result<Self> {
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
