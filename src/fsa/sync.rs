//! Synchronisation primitives for the test FSA.

use crate::err;
use std::convert::TryFrom;
use std::sync::{
    atomic::{AtomicIsize, Ordering},
    Arc, Barrier,
};

/// Trait of things that can serve as thread synchronisers in the FSA.
pub trait Synchroniser {
    /// Runner should call this after running;
    /// it returns whether the runner is an observer or not.
    fn after_run(&self, tid: usize) -> bool;

    /// Relinquishes any observer privileges and/or waits for the observer to
    /// finish.
    fn wait_for_obs(&self, tid: usize);
}

impl Synchroniser for Barrier {
    fn after_run(&self, _tid: usize) -> bool {
        self.wait().is_leader()
    }

    fn wait_for_obs(&self, _tid: usize) {
        self.wait();
    }
}

/// A synchroniser based on a simple atomic counter and busy-waiting.
pub struct Spinner {
    nthreads: isize,
    inner: AtomicIsize,
}

impl Spinner {
    pub fn new(nthreads: usize) -> err::Result<Self> {
        let initial: isize =
            isize::try_from(nthreads).map_err(err::Error::TooManyThreadsForSpinner)?;
        assert_ne!(initial, 0, "no threads?");

        Ok(Spinner {
            nthreads: initial,
            inner: AtomicIsize::new(initial),
        })
    }
}

impl Synchroniser for Spinner {
    fn after_run(&self, _tid: usize) -> bool {
        let count = self.inner.fetch_sub(1, Ordering::AcqRel);
        assert!(0 < count, "count negative after run (={})", count);

        if count == 1 {
            // We were the last thread to be waited upon.
            self.inner.store(-self.nthreads, Ordering::Release);
            true
        } else {
            // We need to wait until the last thread runs.
            while self.inner.load(Ordering::Acquire) >= 0 {
                // busy wait
            }
            false
        }
    }

    fn wait_for_obs(&self, _tid: usize) {
        let count = self.inner.fetch_add(1, Ordering::AcqRel);
        assert!(count < 0, "count positive while waiting (={})", count);

        if count == -1 {
            // We were the last thread to be waited upon.
            self.inner.store(self.nthreads, Ordering::Release);
        } else {
            // We need to wait until the last thread gets here.
            while self.inner.load(Ordering::Acquire) <= 0 {
                // busy wait
            }
        }
    }
}

/// Type alias of functions that return fully wrapped synchronisers.
pub type Factory = fn(usize) -> err::Result<Arc<dyn Synchroniser>>;

/// Wrapper function for making synchronisers out of barriers.
pub fn make_barrier(nthreads: usize) -> err::Result<Arc<dyn Synchroniser>> {
    if nthreads == 0 {
        Err(err::Error::NotEnoughThreads)
    } else {
        Ok(Arc::new(Barrier::new(nthreads)))
    }
}

/// Wrapper function for making synchronisers out of spinners.
pub fn make_spinner(nthreads: usize) -> err::Result<Arc<dyn Synchroniser>> {
    if nthreads == 0 {
        Err(err::Error::NotEnoughThreads)
    } else {
        let spin = Spinner::new(nthreads)?;
        Ok(Arc::new(spin))
    }
}
