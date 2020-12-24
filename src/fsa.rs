use super::{env::Env, err, manifest, test};
use std::sync::{Arc, Barrier};

/// A test handle that is ready to send to its thread.
pub struct Ready<T, E>(Inner<T, E>);

/// We can 'safely' send ReadyTests across thread boundaries.
///
/// Of course, the entire point of concurrency testing is to find concurrency
/// bugs, and these can often manifest as a violation of the sorts of rules
/// that implementing Send is supposed to guarantee.
///
/// The main rationale for this being 'mostly ok' to send across thread
/// boundaries is that the test wrappers constrain the operations we can perform
/// in respect to the thread barriers.
unsafe impl<T, E> Send for Ready<T, E> {}

/// We can 'safely' send references to Envs across thread boundaries.
///
/// See the Sync implementation for the handwave.
unsafe impl<T, E> Sync for Ready<T, E> {}

impl<T, E> Ready<T, E> {
    pub fn start(self) -> Runnable<T, E> {
        Runnable(self.0)
    }
}

/// A test handle that is in the runnable position.
pub struct Runnable<T, E>(Inner<T, E>);

impl<T: test::Entry> Runnable<T, T::Env> {
    pub fn run(mut self) -> RunOutcome<T, T::Env> {
        self.0.entry.run(self.0.tid, &mut self.0.env);
        let bwr = self.0.b.wait();
        if bwr.is_leader() {
            RunOutcome::Observe(Observable(self.0))
        } else {
            RunOutcome::Wait(Waiting(self.0))
        }
    }
}

/// A test handle that is in the waiting position.
pub struct Waiting<T, E>(Inner<T, E>);

impl<T, E> Waiting<T, E> {
    pub fn wait(self) -> Runnable<T, E> {
        self.0.b.wait();
        Runnable(self.0)
    }
}

/// A test handle that is in the observable position.
pub struct Observable<T, E>(Inner<T, E>);

pub enum RunOutcome<T, E> {
    /// This thread should wait until it can run again.
    Wait(Waiting<T, E>),
    /// This thread should read the current state, then wait until it can run again.
    Observe(Observable<T, E>),
}

impl<T, E> Observable<T, E> {
    /// Borrows access to the test's shared environment.
    pub fn env(&mut self) -> &mut E {
        &mut self.0.env
    }

    /// Relinquishes the ability to observe the environment, and returns to a
    /// waiting state.
    pub fn relinquish(self) -> Waiting<T, E> {
        Waiting(self.0)
    }
}

/// Hidden implementation of all the various test handles.
#[derive(Clone)]
struct Inner<T, E> {
    tid: usize,
    env: E,
    entry: T,
    b: Arc<Barrier>,
}

/// A bundle of prepared test data, ready to be run.
pub struct Bundle<T, E> {
    /// The test manifest.
    pub manifest: manifest::Manifest,

    pub handles: ReadySet<T, E>,
}

/// A set of test FSAs, ready to be sent to threads and run.
///
/// `ReadySet`s may be `clone`d; one reason you may want to do this is to run
/// instances of a test across multiple thread constructions.
#[derive(Clone)]
pub struct ReadySet<T, E> {
    vec: Vec<Inner<T, E>>,
}

impl<T, E> ReadySet<T, E> {
    /// The number of FSAs in this set.
    pub fn len(&self) -> usize {
        self.vec.len()
    }
}

/// We can consume a ReadySet into an iterator over Ready FSA handles.
impl<T, E> IntoIterator for ReadySet<T, E> {
    type Item = Ready<T, E>;

    type IntoIter = SetIter<T, E>;

    fn into_iter(self) -> Self::IntoIter {
        SetIter(self.vec.into_iter().map(Ready))
    }
}

/// Type alias of taking `Ready` as a function.
type Readier<T, E> = fn(Inner<T, E>) -> Ready<T, E>;

/// Iterator produced by iterating on `ReadySet`s.
pub struct SetIter<T, E>(
    // This mainly just exists so that we don't leak `Inner`, which we would
    // do if we set this to a type alias.
    std::iter::Map<std::vec::IntoIter<Inner<T, E>>, Readier<T, E>>,
);

impl<T, E> Iterator for SetIter<T, E> {
    type Item = Ready<T, E>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

pub fn build<T: test::Entry>(entry: T) -> err::Result<Bundle<T, T::Env>> {
    let manifest = entry.make_manifest()?;
    let env = T::Env::for_manifest(&manifest)?;
    let b = Arc::new(Barrier::new(manifest.n_threads));
    let inner = Inner {
        tid: manifest.n_threads - 1,
        env,
        b,
        entry,
    };

    let mut handles = ReadySet {
        vec: Vec::with_capacity(manifest.n_threads),
    };
    for tid in 0..manifest.n_threads - 1 {
        let mut tc = inner.clone();
        tc.tid = tid;
        handles.vec.push(tc);
    }
    handles.vec.push(inner);
    Ok(Bundle { manifest, handles })
}
