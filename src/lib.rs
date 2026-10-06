//! Build a table from typed values, then turn it into text or another format.
//!
//! Start with [`Table::with_fields`] and [`row!`]. Sorting uses the original
//! values; each renderer decides how to display them. Rendering returns a
//! result because invalid fields or impossible width limits need an error.
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
mod cell;
mod color;
mod error;
mod import;
mod options;
pub mod render;
mod table;
mod width;
pub mod wrap;
pub use cell::{Cell, cmp_cells, try_sort};
pub use color::{Theme, Themes};
pub use error::{Error, Result};
pub use import::*;
pub use options::*;
pub use render::json::JsonStyle;
pub use table::Table;
pub use width::{display_width, expand_tabs, justify, strip_ansi, truncate};
/// Alias for callers familiar with Python PrettyTable.
pub type PrettyTable = Table;
