/// Enumeration of errors that can happen in phenolphthalein.
#[derive(Debug)]
pub enum Error {
    EnvAllocFailed,
    NotEnoughThreads,

    /// Error returned when we try to construct a `Spinner` with more threads
    /// than can be stored in a `ssize_t`.  (Unlikely to happen in practice.)
    TooManyThreadsForSpinner(std::num::TryFromIntError),

    LockReleaseFailed,
    DlopenFailed(dlopen::Error),
    LockPoisoned,
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
