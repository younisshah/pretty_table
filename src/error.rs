//! Report failures as values so callers can decide what to do.
//! For example, an unknown column can be reported to a user without panicking.

use std::fmt;
/// Errors produced by table mutation, comparison and rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A requested column does not exist, such as "Agge" instead of "Age".
    UnknownField(String),
    /// A two-column table was given a row with a different number of cells.
    RowLength {
        expected: usize,
        actual: usize,
    },
    DuplicateField(String),
    IndexOutOfRange(usize),
    /// Values such as the number 2 and the string "two" cannot be ordered together.
    Incomparable {
        left: String,
        right: String,
    },
    InvalidOption(String),
    /// A display-width limit is too small; for example, 中 alone already needs width 2.
    CannotFit {
        needed: usize,
        limit: usize,
    },
    /// JSON has no number representation for NaN or infinity.
    JsonNonFinite,
    /// A parsed integer is outside the supported i64/u64 range; it is not rounded.
    IntOutOfRange(String),
    Parse(String),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownField(s) => write!(f, "unknown field: {s}"),
            Self::RowLength { expected, actual } => {
                write!(f, "expected {expected} cells, got {actual}")
            }
            Self::DuplicateField(s) => write!(f, "duplicate field: {s}"),
            Self::IndexOutOfRange(i) => write!(f, "index out of range: {i}"),
            Self::Incomparable { left, right } => {
                write!(f, "incomparable cells: {left} and {right}")
            }
            Self::InvalidOption(s) => write!(f, "invalid option: {s}"),
            Self::CannotFit { needed, limit } => {
                write!(f, "table needs {needed} columns, limit is {limit}")
            }
            Self::JsonNonFinite => write!(f, "JSON cannot represent non-finite floats"),
            Self::IntOutOfRange(s) => write!(f, "integer out of range: {s}"),
            Self::Parse(s) => write!(f, "parse error: {s}"),
        }
    }
}
impl std::error::Error for Error {}
/// Result used by this crate.
pub type Result<T> = std::result::Result<T, Error>;
