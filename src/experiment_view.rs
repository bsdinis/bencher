use std::io::Write;

use crate::*;

#[derive(Debug, Copy, Clone)]
pub enum Bars {
    None,
    Linear(usize),
    X(usize),
    Y(usize),
    XY(usize, usize),
}

impl Default for Bars {
    fn default() -> Self {
        Bars::None
    }
}

impl Bars {
    /// Build a Bars object from Options on all three types of bars
    /// Will error on incompatible options
    ///
    pub fn from_optionals(
        bar: Option<usize>,
        xbar: Option<usize>,
        ybar: Option<usize>,
    ) -> BencherResult<Self> {
        match (bar, xbar, ybar) {
            (None, None, None) => Ok(Bars::None),
            (Some(c), None, None) => Ok(Bars::Linear(c)),
            (None, Some(x), None) => Ok(Bars::X(x)),
            (None, None, Some(y)) => Ok(Bars::Y(y)),
            (None, Some(x), Some(y)) => Ok(Bars::XY(x, y)),
            (Some(_), Some(_), _) => Err(BencherError::IncompatibleBarTypes),
            (Some(_), _, Some(_)) => Err(BencherError::IncompatibleBarTypes),
        }
    }

    /// Build a Bars object from bools, fill with dummy values
    /// If someone wants to build objects without specific confidence values probably they don't need the actual values
    ///
    /// Will error on incompatible bools
    ///
    pub fn from_bools(bar: bool, xbar: bool, ybar: bool) -> BencherResult<Self> {
        match (bar, xbar, ybar) {
            (false, false, false) => Ok(Bars::None),
            (true, false, false) => Ok(Bars::Linear(5)),
            (false, true, false) => Ok(Bars::X(5)),
            (false, false, true) => Ok(Bars::Y(5)),
            (false, true, true) => Ok(Bars::XY(5, 5)),
            (true, true, _) => Err(BencherError::IncompatibleBarTypes),
            (true, _, true) => Err(BencherError::IncompatibleBarTypes),
        }
    }
}

/// This trait represents an experiment that can be plotted, etc.
///
/// Represents a group of values/lines
pub trait ExperimentView {
    fn gnuplot(&self, prefix: &std::path::Path, bar: Bars) -> BencherResult<()>;
    fn dat(&self, prefix: &std::path::Path, bar: Bars) -> BencherResult<()>;

    fn plot(&self, prefix: &std::path::Path, bar: Bars) -> BencherResult<()> {
        self.gnuplot(prefix, bar)?;
        self.dat(prefix, bar)
    }

    fn table<W: Write>(&self, writer: &mut W) -> BencherResult<()>;
    fn latex_table<W: Write>(&self, writer: &mut W) -> BencherResult<()>;
}
