//! Model types for checks.

/// The result of running a checker.
#[derive(Copy, Clone)]
pub enum Outcome {
    /// The observation passed its check.
    Passed,
    /// The observation failed its check.
    Failed,
}

impl Outcome {
    /// Converts a pass/fail Boolean to an Outcome.
    pub fn from_pass_bool(is_pass: bool) -> Self {
        if is_pass {
            Self::Passed
        } else {
            Self::Failed
        }
    }
}
