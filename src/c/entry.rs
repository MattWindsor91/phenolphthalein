use dlopen::symbor::{Library, Ref, SymBorApi, Symbol};

use super::{env, manifest};
use crate::{err, manifest as m, obs, test};


/// Entry point for C-ABI tests coming from dynamically loaded libraries.
#[derive(SymBorApi, Clone)]
pub struct CTestApi<'a> {
    manifest: Ref<'a, manifest::Manifest>,

    test: Symbol<'a, unsafe extern "C" fn(tid: libc::size_t, env: *mut env::UnsafeEnv)>,
    check: Symbol<'a, unsafe extern "C" fn(env: *const env::UnsafeEnv) -> bool>,
}

pub struct CChecker<'a>(Symbol<'a, unsafe extern "C" fn(env: *const env::UnsafeEnv) -> bool>);

impl<'a> obs::Checker for CChecker<'a> {
    type Env = env::Env;

    fn check(&self, e: &Self::Env) -> bool {
        unsafe { (self.0)(e.p) }
    }
}

impl<'a> test::Entry for CTestApi<'a> {
    type Env = env::Env;
    type Checker = CChecker<'a>;

    fn run(&self, tid: usize, e: &mut Self::Env) {
        unsafe { (self.test)(tid, e.p) }
    }

    fn make_manifest(&self) -> err::Result<m::Manifest> {
        self.manifest.to_manifest()
    }

    /// Gets a checker for this test.
    fn checker(&self) -> Self::Checker {
        CChecker(self.check)
    }
}

pub fn load_test(lib: &Library) -> err::Result<CTestApi> {
    let c = unsafe { CTestApi::load(&lib) }?;
    Ok(c)
}
