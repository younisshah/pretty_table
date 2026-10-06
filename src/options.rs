//! Describe which rows to show and how the result should look.
//! Stored options last across renders; get_string_with changes a cloned copy.

use crate::{Cell, Error, Result};
use std::{collections::HashMap, rc::Rc};
/// Horizontal cell alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}
/// Vertical cell alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VAlign {
    Top,
    Middle,
    Bottom,
}
/// Horizontal rule placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HRuleStyle {
    Frame,
    All,
    None,
    Header,
}
/// Vertical rule placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VRuleStyle {
    Frame,
    All,
    None,
}
/// Deterministic text presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableStyle {
    Default,
    MswordFriendly,
    PlainColumns,
    Markdown,
    Orgmode,
    DoubleBorder,
    SingleBorder,
    Rst,
}
/// Header case conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderStyle {
    Cap,
    Title,
    Upper,
    Lower,
}
/// Export format selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Text,
    Csv,
    Json,
    Html,
    Latex,
    Mediawiki,
}
/// Apply one setting to all columns, selected columns, or clear it.
/// `Scalar` also supplies a default for future columns; `Map` changes named columns.
///
/// ```
/// use pretty_table_rs::{Align, ColumnValue, Table};
/// let mut table = Table::with_fields(["Name", "Age"]).unwrap();
/// table.set_align(ColumnValue::Scalar(Align::Left)).unwrap();
/// table.set_align(ColumnValue::Map(
///     [("Age".into(), Align::Right)].into_iter().collect()
/// )).unwrap();
/// ```
#[derive(Clone)]
pub enum ColumnValue<T> {
    Scalar(T),
    Map(HashMap<String, T>),
    None,
}
/// Custom cell formatter.
pub type CustomFormat = Rc<dyn Fn(&str, &Cell) -> String>;
/// Decorated-row sorting key, evaluated once per selected row.
pub type SortKey = Rc<dyn Fn(Vec<Cell>) -> Vec<Cell>>;
/// Row predicate.
pub type RowFilter = Rc<dyn Fn(&[Cell]) -> bool>;
/// Per-render options. Clone these for one-off rendering overrides.
#[derive(Clone)]
pub struct Options {
    pub title: Option<String>,
    /// First row to include; start = 1 skips the first selected row.
    pub start: usize,
    /// Exclusive end: start = 1, end = Some(3) selects two rows.
    pub end: Option<usize>,
    /// Columns to display, in the table's original order; the source data is retained.
    pub fields: Option<Vec<String>>,
    pub header: bool,
    pub use_header_width: bool,
    pub border: bool,
    /// Keep separators between columns even when the outer border is disabled.
    pub preserve_internal_border: bool,
    pub hrules: HRuleStyle,
    pub vrules: VRuleStyle,
    pub sortby: Option<String>,
    pub reversesort: bool,
    pub sort_key: Option<SortKey>,
    pub row_filter: Option<RowFilter>,
    pub padding_width: usize,
    pub left_padding_width: Option<usize>,
    pub right_padding_width: Option<usize>,
    pub vertical_char: String,
    pub horizontal_char: String,
    pub horizontal_align_char: Option<String>,
    pub header_horizontal_char: Option<String>,
    pub junction_char: String,
    pub top_junction_char: Option<String>,
    pub bottom_junction_char: Option<String>,
    pub left_junction_char: Option<String>,
    pub right_junction_char: Option<String>,
    pub top_left_junction_char: Option<String>,
    pub top_right_junction_char: Option<String>,
    pub bottom_left_junction_char: Option<String>,
    pub bottom_right_junction_char: Option<String>,
    pub print_empty: bool,
    /// Legacy mode slices before filtering and sorting instead of afterwards.
    pub oldsortslice: bool,
    /// Add layout styling to HTML/LaTeX; false emits their simpler table forms.
    pub format: bool,
    pub xhtml: bool,
    pub attributes: Vec<(String, String)>,
    pub escape_header: bool,
    pub escape_data: bool,
    pub break_on_hyphens: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            title: None,
            start: 0,
            end: None,
            fields: None,
            header: true,
            use_header_width: true,
            border: true,
            preserve_internal_border: false,
            hrules: HRuleStyle::Frame,
            vrules: VRuleStyle::All,
            sortby: None,
            reversesort: false,
            sort_key: None,
            row_filter: None,
            padding_width: 1,
            left_padding_width: None,
            right_padding_width: None,
            vertical_char: "|".into(),
            horizontal_char: "-".into(),
            horizontal_align_char: None,
            header_horizontal_char: None,
            junction_char: "+".into(),
            top_junction_char: None,
            bottom_junction_char: None,
            left_junction_char: None,
            right_junction_char: None,
            top_left_junction_char: None,
            top_right_junction_char: None,
            bottom_left_junction_char: None,
            bottom_right_junction_char: None,
            print_empty: true,
            oldsortslice: false,
            format: false,
            xhtml: false,
            attributes: vec![],
            escape_header: true,
            escape_data: true,
            break_on_hyphens: true,
        }
    }
}
impl Options {
    /// Validate selection and rule characters before rendering.
    pub fn validate(&self, names: &[String]) -> Result<()> {
        if let Some(s) = &self.sortby {
            if !names.contains(s) {
                return Err(Error::UnknownField(s.clone()));
            }
        }
        if let Some(fields) = &self.fields {
            for s in fields {
                if !names.contains(s) {
                    return Err(Error::UnknownField(s.clone()));
                }
            }
        }
        if self.end.is_some_and(|end| self.start > end) {
            return Err(Error::InvalidOption("start exceeds end".into()));
        }
        for s in [
            &self.vertical_char,
            &self.horizontal_char,
            &self.junction_char,
        ]
        .into_iter()
        .chain(
            [
                &self.horizontal_align_char,
                &self.header_horizontal_char,
                &self.top_junction_char,
                &self.bottom_junction_char,
                &self.left_junction_char,
                &self.right_junction_char,
                &self.top_left_junction_char,
                &self.top_right_junction_char,
                &self.bottom_left_junction_char,
                &self.bottom_right_junction_char,
            ]
            .into_iter()
            .filter_map(|s| s.as_ref()),
        ) {
            if crate::display_width(s) != 1 {
                return Err(Error::InvalidOption(format!(
                    "rule character must have width one: {s:?}"
                )));
            }
        }
        Ok(())
    }
    // Explicit left/right padding wins; unset sides use padding_width.
    pub(crate) fn padding(&self) -> (usize, usize) {
        (
            self.left_padding_width.unwrap_or(self.padding_width),
            self.right_padding_width.unwrap_or(self.padding_width),
        )
    }
}
