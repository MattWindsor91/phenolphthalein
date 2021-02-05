//! Ways to temporarily or permanently halt a running test.

use super::obs;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// An halting condition for a test run.
#[derive(Clone)]
pub enum Condition {
    /// The test should rotate or exit when the iteration count reaches this
    /// a multiple of this number.
    EveryNIterations(usize, Type),
    /// The test should rotate or exit when this flag goes high.
    OnSignal(Arc<AtomicBool>, Type),
}

impl Condition {
    /// Constructs a halting condition that occurs when a callback is called.
    pub fn on_callback(ty: Type) -> (Condition, impl FnMut()) {
        let signal = Arc::new(AtomicBool::new(false));
        let c = Self::OnSignal(signal.clone(), ty);
        (c, move || signal.store(true, Ordering::Release))
    }

    /// Gets the sort of exit, if any, that should occur given this condition
    /// and the most recent observation os.
    pub fn exit_type(&self, os: &obs::Summary) -> Option<Type> {
        match self {
            Self::EveryNIterations(n, et) => et.exit_if(os.iterations % *n == 0),
            Self::OnSignal(s, et) => et.exit_if(s.load(Ordering::Acquire)),
        }
    }
}

/// Enumeration of ways the test can be halted.
///
/// `Type`s are ordered such that exiting is 'greater than' rotating.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Type {
    /// The test's threads should be torn down and reset.
    Rotate,
    /// The test should exit.
    Exit,
}

impl Type {
    /// Packs a ExitType into a state byte.
    pub(super) const fn to_u8(self) -> u8 {
        match self {
            Self::Rotate => 1,
            Self::Exit => 2,
        }
    }
    /// Unpacks a ExitType from a state byte.
    pub(super) const fn from_u8(x: u8) -> Option<Self> {
        match x {
            1 => Some(Self::Rotate),
            2 => Some(Self::Exit),
            _ => None,
        }
    }

    fn exit_if(self, p: bool) -> Option<Type> {
        if p {
            Some(self)
        } else {
            None
        }
    }
}
