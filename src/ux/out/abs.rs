//! Abstract interface for outputters.

use super::err;
use crate::model;

/// Trait of things that can output a report.
pub trait Outputter {
    /// Outputs the report `r`, flushing and returning any errors arising.
    ///
    /// This trait consumes the outputter, as there is no guarantee that any
    /// underlying resources can be used multiple times.
    fn output(self: Box<Self>, r: model::report::Report) -> err::Result<()>;
}
