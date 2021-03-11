//! Ways to temporarily or permanently halt a running test.

use super::obs;
use crate::model::Outcome;
use std::{
    num::NonZeroUsize,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

/// A pair of halt condition and halt type.
#[derive(Clone)]
pub struct Rule {
    /// The condition that must hold for the rule to fire.
    pub condition: Condition,
    /// The type of halt that this rule induces.
    pub halt_type: Type,
}

impl Rule {
    /// Constructs a halting rule that occurs when a callback is called.
    pub fn on_callback(ty: Type) -> (Self, impl FnMut()) {
        let (cond, cb) = Condition::on_callback();
        (cond.halt_with(ty), cb)
    }

    /// Gets the sort of exit, if any, that should occur given this condition
    /// and the most recent observation os.
    pub fn exit_type(&self, os: &obs::Summary) -> Option<Type> {
        self.condition.check(os).then(|| self.halt_type)
    }
}

/// An halting condition for a test run.
#[derive(Clone)]
pub enum Condition {
    /// The test should halt when the iteration count reaches this
    /// a multiple of this number.
    EveryNIterations(NonZeroUsize),
    /// The test should halt when this flag goes high.
    OnSignal(Arc<AtomicBool>),
    /// The test should halt when the first outcome of this type occurs.
    OnOutcome(Outcome),
}

impl Condition {
    /// Lifts this Condition to a Rule with halt type `halt_type`.
    pub fn halt_with(self, halt_type: Type) -> Rule {
        Rule {
            condition: self,
            halt_type,
        }
    }

    /// Lifts this condition to an exit Rule.
    pub fn exit(self) -> Rule {
        self.halt_with(Type::Exit)
    }

    /// Lifts this condition to a rotation Rule.
    pub fn rotate(self) -> Rule {
        self.halt_with(Type::Rotate)
    }

    /// Constructs a halting condition that occurs when a callback is called.
    pub fn on_callback() -> (Self, impl FnMut()) {
        let signal = Arc::new(AtomicBool::new(false));
        let c = Self::OnSignal(signal.clone());
        (c, move || signal.store(true, Ordering::Release))
    }

    /// Checks to see if this condition holds over `obs`.
    pub fn check(&self, os: &obs::Summary) -> bool {
        match self {
            Self::EveryNIterations(n) => os.iterations % n.get() == 0,
            Self::OnSignal(s) => s.load(Ordering::Acquire),
            Self::OnOutcome(o) => os.info.outcome == *o,
        }
    }
}

/// Enumeration of ways the test can be halted.
///
/// `Type`s are ordered such that exiting is 'greater than' rotating.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Type {
    /// The test's threads should be torn down and reset.
    Rotate,
    /// The test should exit.
    Exit,
}

/// An atomic signal that conveys a halt type.
pub struct Signal(std::sync::atomic::AtomicU8);

/// The default signal is a cleared one.
impl Default for Signal {
    fn default() -> Self {
        Self(std::sync::atomic::AtomicU8::default())
    }
}

impl Signal {
    /// Clears any existing halt signal.
    pub fn clear(&self) {
        self.0.store(0, Ordering::Release)
    }

    /// Sets the halt signal's value to `ty`.
    pub fn set(&self, ty: Type) {
        let value = match ty {
            Type::Rotate => 1,
            Type::Exit => 2,
        };
        self.0.store(value, Ordering::Release)
    }

    /// Gets the halt signal, if any.
    pub fn get(&self) -> Option<Type> {
        match self.0.load(Ordering::Acquire) {
            1 => Some(Type::Rotate),
            2 => Some(Type::Exit),
            _ => None,
        }
    }
}
