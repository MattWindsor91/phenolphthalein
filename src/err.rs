/// Enumeration of errors that can happen with test creation.
#[derive(Debug)]
pub enum Error {
    EnvAllocFailed,
    NotEnoughThreads,
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
