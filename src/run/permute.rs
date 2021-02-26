//! Traits for thread permutation.

use rand::{prelude::SliceRandom, thread_rng};

/// Trait of things that have thread identifiers.
pub trait HasTid {
    /// Gets the ID of the test thread to which this item belongs.
    fn tid(&self) -> usize;
}

/// Trait for things that can permute threads.
pub trait Permuter<T: HasTid> {
    /// Permutes a set of ready automata.
    ///
    /// Given that the FSA set presents each automaton to the thread runner
    /// in order, this can be used to change thread ordering or affinity.
    fn permute(&mut self, threads: &mut [T]);
}

/// Any random number generator can be turned into a permuter.
impl<'a, R: rand::Rng + ?Sized, T: HasTid> Permuter<T> for R {
    fn permute(&mut self, threads: &mut [T]) {
        threads.shuffle(self)
    }
}

// A permuter that doesn't actually permute.
pub struct Nop;

impl<T: HasTid> Permuter<T> for Nop {
    fn permute(&mut self, _: &mut [T]) {}
}

/// Type alias of functions that return fully wrapped permuters.
pub type Factory<T> = fn() -> Box<dyn Permuter<T>>;

/// Makes a boxed permuter from the thread RNG.
pub fn make_thread_rng<T: HasTid>() -> Box<dyn Permuter<T>> {
    Box::new(thread_rng())
}

/// Makes a no-operation boxed permuter.
pub fn make_nop<T: HasTid>() -> Box<dyn Permuter<T>> {
    Box::new(Nop)
}
