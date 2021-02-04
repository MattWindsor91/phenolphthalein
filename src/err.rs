use thiserror::Error;

/// Enumeration of errors that can happen in phenolphthalein.
#[derive(Debug, Error)]
pub enum Error {
    // TODO(@MattWindsor91): consider splitting these into package error types
    #[error("couldn't allocate the shared state")]
    EnvAllocFailed,

    #[error("test must have at least one thread")]
    NotEnoughThreads,

    /// Error returned when we try to construct a `Spinner` with more threads
    /// than can be stored in a `ssize_t`.  (Unlikely to happen in practice.)
    #[error("test has too many threads for 'spinner' sync method: {0}")]
    TooManyThreadsForSpinner(std::num::TryFromIntError),

    #[error("couldn't release the lock")]
    LockReleaseFailed,

    #[error("couldn't dynamically load the test library")]
    DlopenFailed(#[from] dlopen::Error),

    #[error("lock poisoned")]
    LockPoisoned,

    /// A thread panicked (we don't yet try to recover the specific error).
    #[error("thread panicked (FIXME: error unavailable)")]
    ThreadPanic,

    /// Miscellaneous I/O error.
    #[error("I/O error")]
    IoError(#[from] std::io::Error),
}
pub type Result<T> = std::result::Result<T, Error>;

impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        // TODO(@MattWindsor91): use the error somehow?
        Self::LockPoisoned
    }
}
