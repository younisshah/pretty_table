//! Keep the original value until a renderer needs text.
//! For example, the number 30 sorts numerically and becomes a JSON number,
//! while the string "30" remains text.

use crate::{Error, Result};
use std::cmp::Ordering;
/// One value in a row: text, a number, a boolean, or a missing value.
///
/// The type is kept even when two cells would print the same text.
/// `Int(1)` and `UInt(1)` are different values for `==`, but sort equally.
///
/// ```
/// use pretty_table::Cell;
/// assert_eq!(Cell::from(30_i64), Cell::Int(30));
/// assert_eq!(Cell::from("30"), Cell::Str("30".into()));
/// assert_eq!(Cell::from(None::<u64>), Cell::None);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    None,
    Bool(bool),
    Int(i64),
    UInt(u64),
    Float(f64),
    Str(String),
}
impl From<&str> for Cell {
    fn from(v: &str) -> Self {
        Self::Str(v.into())
    }
}
impl From<String> for Cell {
    fn from(v: String) -> Self {
        Self::Str(v)
    }
}
impl From<bool> for Cell {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}
impl From<f32> for Cell {
    fn from(v: f32) -> Self {
        Self::Float(f64::from(v))
    }
}
impl From<f64> for Cell {
    fn from(v: f64) -> Self {
        Self::Float(v)
    }
}
macro_rules! signed {($($t:ty),*)=>{$(impl From<$t> for Cell {fn from(v:$t)->Self{Self::Int(i64::from(v))}})*}}
macro_rules! unsigned {($($t:ty),*)=>{$(impl From<$t> for Cell {fn from(v:$t)->Self{Self::UInt(u64::from(v))}})*}}
signed!(i8, i16, i32, i64);
unsigned!(u8, u16, u32, u64);
impl From<usize> for Cell {
    fn from(v: usize) -> Self {
        Self::UInt(u64::try_from(v).expect("usize fits u64 on supported targets"))
    }
}
impl<T: Into<Cell>> From<Option<T>> for Cell {
    fn from(v: Option<T>) -> Self {
        v.map(Into::into).unwrap_or(Self::None)
    }
}
impl Cell {
    // i128 can hold both negative i64 values and the entire positive u64 range.
    // Booleans join numeric comparisons as false = 0 and true = 1.
    pub(crate) fn integer(&self) -> Option<i128> {
        match self {
            Self::Bool(v) => Some(i128::from(*v)),
            Self::Int(v) => Some(i128::from(*v)),
            Self::UInt(v) => Some(i128::from(*v)),
            _ => None,
        }
    }
    /// Python-compatible string representation, without table-specific formatting.
    pub fn text(&self) -> String {
        match self {
            Self::None => "None".into(),
            Self::Bool(true) => "True".into(),
            Self::Bool(false) => "False".into(),
            Self::Int(v) => v.to_string(),
            Self::UInt(v) => v.to_string(),
            Self::Str(v) => v.clone(),
            Self::Float(v) => float_repr(*v),
        }
    }
}
pub(crate) fn float_repr(v: f64) -> String {
    if v.is_nan() {
        return "nan".into();
    }
    if v == f64::INFINITY {
        return "inf".into();
    }
    if v == f64::NEG_INFINITY {
        return "-inf".into();
    }
    // Debug keeps the decimal point in 3.0; ordinary Display would print 3.
    // Then match Python's exponent spelling: 1e-5 becomes 1e-05.
    let mut s = format!("{v:?}");
    if let Some(p) = s.find('e') {
        let e = s[p + 1..].parse::<i32>().unwrap();
        s.truncate(p);
        s.push_str(&format!(
            "e{}{:02}",
            if e >= 0 { "+" } else { "-" },
            e.abs()
        ));
    }
    s
}
fn bad(a: &Cell, b: &Cell) -> Error {
    Error::Incomparable {
        left: a.text(),
        right: b.text(),
    }
}
// Do not turn the integer into f64: 9_007_199_254_740_993 would round down.
// Compare whole parts first, then the fraction: 3 < 3.5, but 3 > 2.5.
fn int_float(i: i128, f: f64) -> Option<Ordering> {
    if f.is_nan() {
        None
    } else if f >= 18446744073709551616.0 {
        Some(Ordering::Less)
    } else if f < -9223372036854775808.0 {
        Some(Ordering::Greater)
    } else {
        Some(
            i.cmp(&(f.trunc() as i128))
                .then_with(|| 0.0f64.partial_cmp(&f.fract()).unwrap()),
        )
    }
}
/// Compare numbers without losing integer precision.
/// Text compares with text. A number and a string, or a NaN, produce an error.
///
/// ```
/// use pretty_table::{Cell, cmp_cells};
/// use std::cmp::Ordering;
/// assert_eq!(cmp_cells(&Cell::Int(10), &Cell::Float(2.5)).unwrap(), Ordering::Greater);
/// assert!(cmp_cells(&Cell::Int(2), &Cell::from("2")).is_err());
/// ```
pub fn cmp_cells(a: &Cell, b: &Cell) -> Result<Ordering> {
    if let (Some(x), Some(y)) = (a.integer(), b.integer()) {
        return Ok(x.cmp(&y));
    }
    let c = match (a, b) {
        (Cell::Float(x), Cell::Float(y)) => x.partial_cmp(y),
        (Cell::Float(x), _) => b
            .integer()
            .and_then(|y| int_float(y, *x))
            .map(Ordering::reverse),
        (_, Cell::Float(y)) => a.integer().and_then(|x| int_float(x, *y)),
        (Cell::Str(x), Cell::Str(y)) => Some(x.cmp(y)),
        (Cell::None, Cell::None) => Some(Ordering::Equal),
        _ => None,
    };
    c.ok_or_else(|| bad(a, b))
}
fn cmp_rows(a: &[Cell], b: &[Cell]) -> Result<Ordering> {
    for (a, b) in a.iter().zip(b) {
        let c = cmp_cells(a, b)?;
        if c != Ordering::Equal {
            return Ok(c);
        }
    }
    Ok(a.len().cmp(&b.len()))
}
/// Return row positions in sorted order, leaving the original rows untouched.
/// A custom key is computed once per row. Equal keys keep their original order.
///
/// ```
/// use pretty_table::{row, try_sort};
/// let rows = vec![row!["Bob"], row!["Ada"]];
/// assert_eq!(try_sort(&rows, None, false).unwrap(), vec![1, 0]);
/// assert_eq!(rows[0], row!["Bob"]);
/// ```
pub fn try_sort(
    rows: &[Vec<Cell>],
    key: Option<&crate::SortKey>,
    reverse: bool,
) -> Result<Vec<usize>> {
    let keys: Vec<_> = rows
        .iter()
        .map(|r| key.map_or_else(|| r.clone(), |k| k(r.clone())))
        .collect();
    fn merge(indices: &[usize], keys: &[Vec<Cell>], reverse: bool) -> Result<Vec<usize>> {
        if indices.len() < 2 {
            return Ok(indices.to_vec());
        }
        let n = indices.len() / 2;
        let a = merge(&indices[..n], keys, reverse)?;
        let b = merge(&indices[n..], keys, reverse)?;
        let (mut i, mut j, mut out) = (0, 0, Vec::with_capacity(indices.len()));
        while i < a.len() && j < b.len() {
            let mut c = cmp_rows(&keys[a[i]], &keys[b[j]])?;
            // Reverse the comparison, not the final list: equal keys must stay stable.
            if reverse {
                c = c.reverse()
            }
            // On a tie, take the left row first to preserve insertion order.
            if c != Ordering::Greater {
                out.push(a[i]);
                i += 1
            } else {
                out.push(b[j]);
                j += 1
            }
        }
        out.extend_from_slice(&a[i..]);
        out.extend_from_slice(&b[j..]);
        Ok(out)
    }
    merge(&(0..rows.len()).collect::<Vec<_>>(), &keys, reverse)
}
/// Build a row containing different supported value types.
///
/// ```
/// use pretty_table::{Cell, row};
/// let values = row!["Ada", 30, true];
/// assert_eq!(values[1], Cell::Int(30));
/// ```
#[macro_export]
macro_rules! row {($($v:expr),* $(,)?)=>{vec![$($crate::Cell::from($v)),*]};}
