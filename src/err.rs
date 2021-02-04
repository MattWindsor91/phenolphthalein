/// Enumeration of errors that can happen in phenolphthalein.
#[derive(Debug)]
pub enum Error {
    // TODO(@MattWindsor91): consider splitting these into package error types
    EnvAllocFailed,
    NotEnoughThreads,

    /// Error returned when we try to construct a `Spinner` with more threads
    /// than can be stored in a `ssize_t`.  (Unlikely to happen in practice.)
    TooManyThreadsForSpinner(std::num::TryFromIntError),

    LockReleaseFailed,
    DlopenFailed(dlopen::Error),
    LockPoisoned,

    /// A thread panicked (we don't yet try to recover the specific error).
    ThreadPanic,

    /// Miscellaneous I/O error.
    IoError(std::io::Error),
}
pub type Result<T> = std::result::Result<T, Error>;

impl From<dlopen::Error> for Error {
    fn from(e: dlopen::Error) -> Self {
        Self::DlopenFailed(e)
    }
}

impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        // TODO(@MattWindsor91): use the error somehow?
        Self::LockPoisoned
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}
