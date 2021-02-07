//! Synchronisation primitives for the test FSA.

use crate::err;
use std::convert::TryFrom;
use std::sync::{
    atomic::{AtomicIsize, Ordering},
    Arc, Barrier,
};

/// Trait of things that can serve as thread synchronisers in the FSA.
///
/// `Synchroniser` is an unsafe trait because there is a subtle invariant
/// that must be satisfied for it to be usable by the FSA for synchronisation,
/// and failure to satisfy it will result in unsafe behaviour by the FSA.
///
/// The invariant is this: given several threads calling into the `Synchroniser`
/// in the pattern [`run`, `obs` (if `run` true) or `wait` (`run` false)],
/// then the `Synchroniser` must guarantee that, at any time:
///
/// 1. either all threads are about to call `run`; or,
/// 2. precisely one is about to call `obs` and the others are about to call
///   `wait`.
///
/// This drives the FSA workflow that, at any point, all threads are either
/// running the concurrent test, or have elected one thread to do the
/// book-keeping for the results of that run while the others wait.
///
/// The synchroniser API is deliberately low level; for instance, it only tracks
/// whether runners can be observers, rather than using that fact to hold data
/// that only the observer can access.  We assume that the FSA itself does this,
/// using the above invariant as justification.
pub unsafe trait Synchroniser {
    /// Runner should call this after running;
    /// it returns whether the runner is an observer or not.
    fn run(&self) -> bool;

    /// Observer should call this after observing;
    /// it performs any necessary synchronisation.
    fn obs(&self);

    /// Waiters should call this after observing;
    /// it performs any necessary synchronisation.
    fn wait(&self);
}

/// Barriers are synchronisers; each phase corresponds to a barrier wait, and
/// observers are nominated through the barrier's own leader function.
unsafe impl Synchroniser for Barrier {
    fn run(&self) -> bool {
        self.wait().is_leader()
    }

    fn obs(&self) {
        self.wait();
    }

    fn wait(&self) {
        self.wait();
    }
}

/// A synchroniser based on a simple atomic counter and busy-waiting.
///
/// When the spinner is positive, we're waiting for runners to finish; when it's
/// negative, we're synchronising on the observer.
pub struct Spinner {
    nthreads: isize,
    inner: AtomicIsize,
}

impl Spinner {
    /// Constructs a new `Spinner` with room for `nthreads` threads.
    ///
    /// A `Spinner` can only hold enough threads that fit inside an `isize`,
    /// for implementation reasons; the constructor will return an error if this
    /// is not the case.
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

unsafe impl Synchroniser for Spinner {
    fn run(&self) -> bool {
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

    fn obs(&self) {
        /* TODO(@MattWindsor91): I'm convinced there must be some way of
        eliminating the synchronisation here, but I can't think of a
        smarter algorithm; if we have obs just set nthreads high here,
        we run the risk of an ABA problem with runners above, for instance */
        self.wait();
    }

    fn wait(&self) {
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
