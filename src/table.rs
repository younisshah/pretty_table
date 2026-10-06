//! Store data and settings; renderers turn a selected view into output.
//! For fields ["Name", "Age"], the row ["Ada", 30] keeps that same column order.

use crate::*;
use std::{
    collections::HashMap,
    num::NonZeroUsize,
    ops::{Bound, RangeBounds},
};
// Remember scalar settings made before columns exist, such as an integer format.
// When a new column arrives, it inherits these defaults.
#[derive(Clone, Default)]
pub(crate) struct ColumnDefaults {
    align: Option<Align>,
    valign: Option<VAlign>,
    max_width: Option<usize>,
    min_width: Option<usize>,
    int_format: Option<String>,
    float_format: Option<String>,
    none_format: Option<String>,
    custom_format: Option<CustomFormat>,
}
/// An owned, typed table. Rendering is immutable and fallible.
#[derive(Clone)]
pub struct Table {
    pub(crate) field_names: Vec<String>,
    pub(crate) rows: Vec<Vec<Cell>>,
    // dividers[i] means "draw a separator after rows[i]"; keep their indices together.
    pub(crate) dividers: Vec<bool>,
    pub(crate) align: HashMap<String, Align>,
    pub(crate) valign: HashMap<String, VAlign>,
    pub(crate) max_width: HashMap<String, usize>,
    pub(crate) min_width: HashMap<String, usize>,
    pub(crate) int_format: HashMap<String, String>,
    pub(crate) float_format: HashMap<String, String>,
    pub(crate) none_format: HashMap<String, String>,
    pub(crate) custom_format: HashMap<String, CustomFormat>,
    pub(crate) base_align: Align,
    pub(crate) ctor: ColumnDefaults,
    pub(crate) style: Option<TableStyle>,
    pub(crate) orgmode: bool,
    pub(crate) header_style: Option<HeaderStyle>,
    pub(crate) min_table_width: Option<usize>,
    pub(crate) max_table_width: Option<usize>,
    pub(crate) theme: Option<Theme>,
    pub opts: Options,
}
impl Default for Table {
    fn default() -> Self {
        Self::new()
    }
}
impl Table {
    /// Create an empty table, inferring field names from the first added row.
    pub fn new() -> Self {
        Self {
            field_names: vec![],
            rows: vec![],
            dividers: vec![],
            align: HashMap::new(),
            valign: HashMap::new(),
            max_width: HashMap::new(),
            min_width: HashMap::new(),
            int_format: HashMap::new(),
            float_format: HashMap::new(),
            none_format: HashMap::new(),
            custom_format: HashMap::new(),
            base_align: Align::Center,
            ctor: ColumnDefaults::default(),
            style: None,
            orgmode: false,
            header_style: None,
            min_table_width: None,
            max_table_width: None,
            theme: None,
            opts: Options::default(),
        }
    }
    /// Create a table with unique column names.
    ///
    /// ```
    /// use pretty_table::{Table, row};
    /// let mut table = Table::with_fields(["Name", "Age"]).unwrap();
    /// table.add_row(row!["Ada", 30]).unwrap();
    /// assert_eq!(table.row_count(), 1);
    /// assert!(table.get_string().unwrap().contains("Ada"));
    /// ```
    pub fn with_fields<I, S>(names: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut t = Self::new();
        t.set_field_names(names)?;
        Ok(t)
    }
    fn check_names(names: &[String]) -> Result<()> {
        let mut seen = std::collections::HashSet::new();
        for name in names {
            if !seen.insert(name) {
                return Err(Error::DuplicateField(name.clone()));
            }
        }
        Ok(())
    }
    /// Rename columns by position, preserving every per-column setting.
    pub fn set_field_names<I, S>(&mut self, names: I) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let names: Vec<_> = names.into_iter().map(Into::into).collect();
        Self::check_names(&names)?;
        if (self.col_count() != 0 || self.row_count() != 0) && names.len() != self.col_count() {
            return Err(Error::RowLength {
                expected: self.col_count(),
                actual: names.len(),
            });
        }
        // Build new maps from old ones. Renaming A/B to B/A must not overwrite
        // B's old setting before we have copied it to its new name.
        fn remap<T: Clone>(
            m: &HashMap<String, T>,
            old: &[String],
            new: &[String],
            default: Option<T>,
        ) -> HashMap<String, T> {
            new.iter()
                .enumerate()
                .filter_map(|(i, n)| {
                    match old.get(i) {
                        Some(k) => m.get(k).cloned(),
                        None => default.clone(),
                    }
                    .map(|v| (n.clone(), v))
                })
                .collect()
        }
        let old = &self.field_names;
        self.align = remap(
            &self.align,
            old,
            &names,
            Some(self.ctor.align.unwrap_or(self.base_align)),
        );
        self.valign = remap(
            &self.valign,
            old,
            &names,
            Some(self.ctor.valign.unwrap_or(VAlign::Top)),
        );
        self.max_width = remap(&self.max_width, old, &names, self.ctor.max_width);
        self.min_width = remap(&self.min_width, old, &names, self.ctor.min_width);
        self.int_format = remap(&self.int_format, old, &names, self.ctor.int_format.clone());
        self.float_format = remap(
            &self.float_format,
            old,
            &names,
            self.ctor.float_format.clone(),
        );
        self.none_format = remap(
            &self.none_format,
            old,
            &names,
            self.ctor.none_format.clone(),
        );
        self.custom_format = remap(
            &self.custom_format,
            old,
            &names,
            self.ctor.custom_format.clone(),
        );
        self.field_names = names;
        Ok(())
    }
    /// Append one row without a divider.
    pub fn add_row<I, C>(&mut self, row: I) -> Result<()>
    where
        I: IntoIterator<Item = C>,
        C: Into<Cell>,
    {
        self.add_row_divider(row, false)
    }
    /// Append one row and its trailing divider atomically.
    pub fn add_row_divider<I, C>(&mut self, row: I, divider: bool) -> Result<()>
    where
        I: IntoIterator<Item = C>,
        C: Into<Cell>,
    {
        self.add_rows([row], divider)
    }
    /// Validate and append a batch atomically.
    pub fn add_rows<I, R, C>(&mut self, rows: I, divider: bool) -> Result<()>
    where
        I: IntoIterator<Item = R>,
        R: IntoIterator<Item = C>,
        C: Into<Cell>,
    {
        // Check the whole batch before changing the table. If its last row is
        // invalid, none of the earlier rows in this batch should be inserted.
        let rows: Vec<Vec<Cell>> = rows
            .into_iter()
            .map(|r| r.into_iter().map(Into::into).collect())
            .collect();
        let expected = if self.col_count() == 0 && self.row_count() == 0 {
            rows.first().map_or(0, Vec::len)
        } else {
            self.col_count()
        };
        for row in &rows {
            if row.len() != expected {
                return Err(Error::RowLength {
                    expected,
                    actual: row.len(),
                });
            }
        }
        if self.col_count() == 0 && !rows.is_empty() {
            self.set_field_names((1..=expected).map(|i| format!("Field {i}")))?
        }
        // A batch divider belongs after the last new row, not after every row.
        self.dividers
            .extend((0..rows.len()).map(|i| divider && i + 1 == rows.len()));
        self.rows.extend(rows);
        Ok(())
    }
    /// Attach a divider to the most recent row.
    pub fn add_divider(&mut self) {
        if let Some(d) = self.dividers.last_mut() {
            *d = true
        }
    }
    /// Append a column; its values must match the current row count.
    pub fn add_column<I, C>(
        &mut self,
        name: impl Into<String>,
        cells: I,
        align: Align,
        valign: VAlign,
    ) -> Result<()>
    where
        I: IntoIterator<Item = C>,
        C: Into<Cell>,
    {
        let name = name.into();
        if self.field_names.contains(&name) {
            return Err(Error::DuplicateField(name));
        }
        let cells: Vec<_> = cells.into_iter().map(Into::into).collect();
        if self.row_count() != 0 && self.row_count() != cells.len() {
            return Err(Error::RowLength {
                expected: self.row_count(),
                actual: cells.len(),
            });
        }
        // Existing headers still need cells: adding C = [7] to empty A/B headers
        // creates [None, None, 7], so every row remains the same width.
        if self.row_count() == 0 && !cells.is_empty() {
            self.rows = vec![vec![Cell::None; self.col_count()]; cells.len()];
            self.dividers = vec![false; cells.len()]
        }
        for (r, c) in self.rows.iter_mut().zip(cells) {
            r.push(c)
        }
        self.field_names.push(name.clone());
        self.align.insert(name.clone(), align);
        self.valign.insert(name.clone(), valign);
        macro_rules! apply_default {
            ($field:ident) => {
                if let Some(value) = self.ctor.$field.clone() {
                    self.$field.insert(name.clone(), value);
                }
            };
        }
        apply_default!(max_width);
        apply_default!(min_width);
        apply_default!(int_format);
        apply_default!(float_format);
        apply_default!(none_format);
        apply_default!(custom_format);
        Ok(())
    }
    /// Prepend a one-based automatic index column.
    pub fn add_autoindex(&mut self, name: &str) -> Result<()> {
        if self.field_names.iter().any(|n| n == name) {
            return Err(Error::DuplicateField(name.into()));
        }
        let mut names = vec![name.to_string()];
        names.extend(self.field_names.clone());
        for (i, row) in self.rows.iter_mut().enumerate() {
            row.insert(0, Cell::UInt((i + 1) as u64))
        }
        self.field_names = names;
        self.align
            .insert(name.into(), self.ctor.align.unwrap_or(self.base_align));
        self.valign
            .insert(name.into(), self.ctor.valign.unwrap_or(VAlign::Top));
        macro_rules! def {
            ($field:ident) => {
                if let Some(v) = self.ctor.$field.clone() {
                    self.$field.insert(name.into(), v);
                }
            };
        }
        def!(max_width);
        def!(min_width);
        def!(int_format);
        def!(float_format);
        def!(none_format);
        def!(custom_format);
        Ok(())
    }
    /// Remove a row by index.
    pub fn del_row(&mut self, i: usize) -> Result<()> {
        if i >= self.row_count() {
            return Err(Error::IndexOutOfRange(i));
        }
        self.rows.remove(i);
        self.dividers.remove(i);
        Ok(())
    }
    /// Remove a column and its settings. Dangling render selections are validated later.
    pub fn del_column(&mut self, name: &str) -> Result<()> {
        let i = self
            .field_names
            .iter()
            .position(|n| n == name)
            .ok_or_else(|| Error::UnknownField(name.into()))?;
        self.field_names.remove(i);
        for r in &mut self.rows {
            r.remove(i);
        }
        macro_rules! del{($($m:ident),*)=>{$(self.$m.remove(name);)*}}
        del!(
            align,
            valign,
            max_width,
            min_width,
            int_format,
            float_format,
            none_format,
            custom_format
        );
        Ok(())
    }
    /// Remove all rows, keeping columns and options.
    pub fn clear_rows(&mut self) {
        self.rows.clear();
        self.dividers.clear()
    }
    /// Remove rows and columns, keeping scalar defaults and options.
    pub fn clear(&mut self) {
        self.clear_rows();
        self.field_names.clear();
        self.align.clear();
        self.valign.clear();
        self.max_width.clear();
        self.min_width.clear();
        self.int_format.clear();
        self.float_format.clear();
        self.none_format.clear();
        self.custom_format.clear();
    }
    /// Clone a clamped row range with independent option and map storage.
    pub fn slice(&self, r: impl RangeBounds<usize>) -> Self {
        let start = match r.start_bound() {
            Bound::Included(i) => *i,
            Bound::Excluded(i) => i.saturating_add(1),
            Bound::Unbounded => 0,
        }
        .min(self.row_count());
        let end = match r.end_bound() {
            Bound::Included(i) => i.saturating_add(1),
            Bound::Excluded(i) => *i,
            Bound::Unbounded => self.row_count(),
        }
        .min(self.row_count())
        .max(start);
        let mut t = self.clone();
        t.rows = self.rows[start..end].to_vec();
        t.dividers = self.dividers[start..end].to_vec();
        t
    }
    /// Current ordered field names.
    pub fn field_names(&self) -> &[String] {
        &self.field_names
    }
    /// Original typed rows.
    pub fn rows(&self) -> &[Vec<Cell>] {
        &self.rows
    }
    /// Original row divider flags.
    pub fn dividers(&self) -> &[bool] {
        &self.dividers
    }
    /// Number of data rows.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }
    /// Number of fields.
    pub fn col_count(&self) -> usize {
        self.field_names.len()
    }
    /// Select rows through filter, exact stable sorting, and slicing.
    pub fn selected_rows(&self, o: &Options) -> Result<Vec<(Vec<Cell>, bool)>> {
        o.validate(&self.field_names)?;
        let slice = |r: Vec<(Vec<Cell>, bool)>| {
            let start = o.start.min(r.len());
            let end = o.end.unwrap_or(r.len()).min(r.len()).max(start);
            r[start..end].to_vec()
        };
        // Carry each divider with its row. Filtering out an earlier row must
        // not move a separator onto a different row.
        let mut pairs: Vec<_> = self
            .rows
            .iter()
            .cloned()
            .zip(self.dividers.iter().copied())
            .collect();
        if o.oldsortslice {
            pairs = slice(pairs)
        }
        if let Some(filter) = &o.row_filter {
            pairs.retain(|(r, _)| filter(r))
        }
        if let Some(name) = &o.sortby {
            let col = self.field_names.iter().position(|n| n == name).unwrap();
            // Sorting by Age temporarily turns ["Ada", 30] into [30, "Ada", 30].
            // The first value chooses the sort column; the rest break ties.
            let decorated: Vec<_> = pairs
                .iter()
                .map(|(r, _)| {
                    let mut d = vec![r[col].clone()];
                    d.extend(r.clone());
                    d
                })
                .collect();
            let perm = try_sort(&decorated, o.sort_key.as_ref(), o.reversesort)?;
            // Reorder only this view. Sorted rows lose section dividers because
            // their original groups may no longer be next to one another.
            pairs = perm.iter().map(|i| (pairs[*i].0.clone(), false)).collect()
        }
        if !o.oldsortslice {
            pairs = slice(pairs)
        }
        Ok(pairs)
    }
    /// Apply column-specific formatting and expand tabs.
    pub fn format_rows(&self, rows: &[Vec<Cell>]) -> Result<Vec<Vec<String>>> {
        rows.iter()
            .map(|r| {
                r.iter()
                    .zip(&self.field_names)
                    .map(|(c, n)| self.format_cell(n, c))
                    .collect()
            })
            .collect()
    }
    /// Format one cell using the column's active numeric/null/custom setting.
    pub fn format_cell(&self, n: &str, c: &Cell) -> Result<String> {
        // Formatting creates display text; it never replaces the stored Cell.
        // For example, Float(2.5) can display as "2.50" and still export as a JSON number.
        let text = if let Some(f) = self.custom_format.get(n) {
            f(n, c)
        } else if matches!(c, Cell::None) && self.none_format.contains_key(n) {
            self.none_format[n].clone()
        } else if let (Some(i), Some(f)) = (c.integer(), self.int_format.get(n)) {
            let w = f.parse::<usize>().unwrap_or(0);
            if f.starts_with('0') {
                format!("{i:0w$}")
            } else {
                format!("{i:>w$}")
            }
        } else if let (Cell::Float(v), Some(f)) = (c, self.float_format.get(n)) {
            let literal = f.ends_with('f');
            let spec = f.strip_suffix('f').unwrap_or(f);
            // "6.2" means at least six characters with two decimal places:
            // 2.5 becomes "  2.50".
            let (w, p) = if spec.is_empty() {
                (0, 6)
            } else if let Some((w, p)) = spec.split_once('.') {
                (w.parse().unwrap_or(0), p.parse().unwrap_or(0))
            } else {
                (spec.parse().unwrap_or(0), 6)
            };
            let mut s = if v.is_nan() {
                if spec.starts_with('0') {
                    format!("{:0>w$}", "nan")
                } else {
                    format!("{:>w$}", "nan")
                }
            } else if spec.starts_with('0') {
                format!("{v:0w$.p$}")
            } else {
                format!("{v:w$.p$}")
            };
            if literal {
                s.push('f')
            }
            s
        } else {
            c.text()
        };
        Ok(expand_tabs(&text))
    }
    /// Set case conversion for text headers.
    pub fn set_header_style(&mut self, s: Option<HeaderStyle>) -> Result<()> {
        self.header_style = s;
        Ok(())
    }
    /// Set minimum complete table width.
    pub fn set_min_table_width(&mut self, w: Option<usize>) -> Result<()> {
        self.min_table_width = w;
        Ok(())
    }
    /// Set maximum complete table width.
    pub fn set_max_table_width(&mut self, w: Option<usize>) -> Result<()> {
        self.max_table_width = w;
        Ok(())
    }
    /// Apply a theme's primary glyph defaults and ANSI colors, or clear its colors.
    /// Later style changes and stored/per-call glyph edits take precedence.
    /// Clearing a theme retains the current stored glyphs.
    pub fn set_theme(&mut self, t: Option<Theme>) -> Result<()> {
        if let Some(t) = &t {
            for s in [&t.vertical_char, &t.horizontal_char, &t.junction_char] {
                if display_width(s) != 1 {
                    return Err(Error::InvalidOption("theme glyph width must be one".into()));
                }
            }
            self.opts.vertical_char = t.vertical_char.clone();
            self.opts.horizontal_char = t.horizontal_char.clone();
            self.opts.junction_char = t.junction_char.clone();
        }
        self.theme = t;
        Ok(())
    }
    /// Apply a preset, resetting all preset-controlled options first.
    pub fn set_style(&mut self, s: TableStyle) -> Result<()> {
        let d = Options::default();
        let o = &mut self.opts;
        o.header = d.header;
        o.border = d.border;
        o.hrules = d.hrules;
        o.vrules = d.vrules;
        o.padding_width = 1;
        o.left_padding_width = Some(1);
        o.right_padding_width = Some(1);
        o.vertical_char = d.vertical_char;
        o.horizontal_char = d.horizontal_char;
        o.horizontal_align_char = None;
        o.header_horizontal_char = None;
        o.junction_char = d.junction_char;
        o.top_junction_char = None;
        o.bottom_junction_char = None;
        o.left_junction_char = None;
        o.right_junction_char = None;
        o.top_left_junction_char = None;
        o.top_right_junction_char = None;
        o.bottom_left_junction_char = None;
        o.bottom_right_junction_char = None;
        self.orgmode = false;
        self.style = Some(s);
        match s {
            TableStyle::Default => {}
            TableStyle::Orgmode => self.orgmode = true,
            TableStyle::Markdown => {
                o.hrules = HRuleStyle::Header;
                o.junction_char = "|".into();
                o.horizontal_align_char = Some(":".into())
            }
            TableStyle::MswordFriendly => o.hrules = HRuleStyle::None,
            TableStyle::PlainColumns => {
                o.border = false;
                o.left_padding_width = Some(0);
                o.right_padding_width = Some(8)
            }
            TableStyle::Rst => {
                o.hrules = HRuleStyle::All;
                o.header_horizontal_char = Some("=".into())
            }
            TableStyle::SingleBorder | TableStyle::DoubleBorder => {
                let chars = if s == TableStyle::SingleBorder {
                    ["─", "│", "┼", "┬", "┴", "┤", "├", "┐", "┌", "┘", "└"]
                } else {
                    ["═", "║", "╬", "╦", "╩", "╣", "╠", "╗", "╔", "╝", "╚"]
                };
                o.horizontal_char = chars[0].into();
                o.vertical_char = chars[1].into();
                o.junction_char = chars[2].into();
                o.top_junction_char = Some(chars[3].into());
                o.bottom_junction_char = Some(chars[4].into());
                o.right_junction_char = Some(chars[5].into());
                o.left_junction_char = Some(chars[6].into());
                o.top_right_junction_char = Some(chars[7].into());
                o.top_left_junction_char = Some(chars[8].into());
                o.bottom_right_junction_char = Some(chars[9].into());
                o.bottom_left_junction_char = Some(chars[10].into());
            }
        }
        Ok(())
    }
    /// Resolve a single immutable render configuration.
    pub fn resolved(&self, o: &Options) -> Result<crate::render::Resolved> {
        crate::render::resolve(self, o)
    }
    /// Render using stored options.
    pub fn get_string(&self) -> Result<String> {
        crate::render::text::render(self, &self.opts)
    }
    /// Change the appearance for this call without changing the stored options.
    ///
    /// ```
    /// use pretty_table::{Table, row};
    /// let mut table = Table::with_fields(["Name"]).unwrap();
    /// table.add_row(row!["Ada"]).unwrap();
    /// let plain = table.get_string_with(|o| o.border = false).unwrap();
    /// assert!(!plain.contains('|'));
    /// assert!(table.opts.border);
    /// ```
    pub fn get_string_with(&self, f: impl FnOnce(&mut Options)) -> Result<String> {
        let mut o = self.opts.clone();
        f(&mut o);
        crate::render::text::render(self, &o)
    }
    /// Render selected rows in pages, selecting and sorting only once.
    pub fn paginate(&self, page_length: NonZeroUsize, line_break: &str) -> Result<String> {
        self.paginate_with(page_length, line_break, |_| {})
    }
    /// Paginate with one-off option overrides.
    pub fn paginate_with(
        &self,
        page_length: NonZeroUsize,
        line_break: &str,
        f: impl FnOnce(&mut Options),
    ) -> Result<String> {
        let mut o = self.opts.clone();
        f(&mut o);
        // Select once, then split into pages. Otherwise filters and sort-key
        // callbacks would run again for every page and could give different results.
        let rows = self.selected_rows(&o)?;
        let mut pages = vec![];
        o.start = 0;
        o.end = None;
        o.sortby = None;
        o.row_filter = None;
        for page in rows.chunks(page_length.get()) {
            let mut t = self.clone();
            t.rows = page.iter().map(|p| p.0.clone()).collect();
            t.dividers = page.iter().map(|p| p.1).collect();
            pages.push(crate::render::text::render(&t, &o)?)
        }
        if pages.is_empty() {
            let mut t = self.clone();
            t.clear_rows();
            pages.push(crate::render::text::render(&t, &o)?)
        }
        Ok(pages.join(line_break))
    }
}
fn setting<T: Clone>(
    names: &[String],
    value: ColumnValue<T>,
    map: &mut HashMap<String, T>,
    default: &mut Option<T>,
) -> Result<Vec<String>> {
    let pairs = match value {
        ColumnValue::Scalar(v) => {
            *default = Some(v.clone());
            names.iter().map(|n| (n.clone(), v.clone())).collect()
        }
        ColumnValue::None => {
            map.clear();
            *default = None;
            return Ok(vec![]);
        }
        ColumnValue::Map(v) => {
            for n in v.keys() {
                if !names.contains(n) {
                    return Err(Error::UnknownField(n.clone()));
                }
            }
            v
        }
    };
    let keys = pairs.keys().cloned().collect();
    map.extend(pairs);
    Ok(keys)
}
macro_rules! setter {($name:ident,$field:ident,$ty:ty)=>{impl Table{#[doc=concat!("Set column ",stringify!($field)," values; scalar settings persist for future fields.") ]pub fn $name(&mut self,v:ColumnValue<$ty>)->Result<()>{setting(&self.field_names,v,&mut self.$field,&mut self.ctor.$field)?;Ok(())}}}}
setter!(set_valign, valign, VAlign);
setter!(set_max_width, max_width, usize);
setter!(set_min_width, min_width, usize);
impl Table {
    /// Set alignment values and the field-less scalar default.
    pub fn set_align(&mut self, v: ColumnValue<Align>) -> Result<()> {
        if let ColumnValue::Scalar(a) = &v {
            self.base_align = *a
        }
        if matches!(&v, ColumnValue::None) {
            self.base_align = Align::Center
        }
        setting(&self.field_names, v, &mut self.align, &mut self.ctor.align)?;
        for n in &self.field_names {
            self.align.entry(n.clone()).or_insert(self.base_align);
        }
        Ok(())
    }
    // A column has one active formatting path. Setting a numeric format
    // replaces a custom formatter for that column; setting a custom one does the reverse.
    /// Set integer format and remove custom formatters for affected fields.
    pub fn set_int_format(&mut self, v: ColumnValue<String>) -> Result<()> {
        validate_formats(&v, false)?;
        let scalar = matches!(&v, ColumnValue::Scalar(_));
        let keys = setting(
            &self.field_names,
            v,
            &mut self.int_format,
            &mut self.ctor.int_format,
        )?;
        for k in keys {
            self.custom_format.remove(&k);
        }
        if scalar {
            self.ctor.custom_format = None;
        }
        Ok(())
    }
    /// Set float format and remove custom formatters for affected fields.
    pub fn set_float_format(&mut self, v: ColumnValue<String>) -> Result<()> {
        validate_formats(&v, true)?;
        let scalar = matches!(&v, ColumnValue::Scalar(_));
        let keys = setting(
            &self.field_names,
            v,
            &mut self.float_format,
            &mut self.ctor.float_format,
        )?;
        for k in keys {
            self.custom_format.remove(&k);
        }
        if scalar {
            self.ctor.custom_format = None;
        }
        Ok(())
    }
    /// Set replacement text for actual null cells only.
    pub fn set_none_format(&mut self, v: ColumnValue<String>) -> Result<()> {
        let scalar = matches!(&v, ColumnValue::Scalar(_));
        let keys = setting(
            &self.field_names,
            v,
            &mut self.none_format,
            &mut self.ctor.none_format,
        )?;
        for k in keys {
            self.custom_format.remove(&k);
        }
        if scalar {
            self.ctor.custom_format = None;
        }
        Ok(())
    }
    /// Set custom callbacks and remove other formatting for affected fields.
    pub fn set_custom_format(&mut self, v: ColumnValue<CustomFormat>) -> Result<()> {
        let scalar = matches!(&v, ColumnValue::Scalar(_));
        let keys = setting(
            &self.field_names,
            v,
            &mut self.custom_format,
            &mut self.ctor.custom_format,
        )?;
        for k in keys {
            self.int_format.remove(&k);
            self.float_format.remove(&k);
            self.none_format.remove(&k);
        }
        if scalar {
            self.ctor.int_format = None;
            self.ctor.float_format = None;
            self.ctor.none_format = None;
        }
        Ok(())
    }
}
fn validate_formats(v: &ColumnValue<String>, float: bool) -> Result<()> {
    let values: Vec<_> = match v {
        ColumnValue::Scalar(v) => vec![v],
        ColumnValue::Map(v) => v.values().collect(),
        ColumnValue::None => vec![],
    };
    for v in values {
        let s = if float {
            v.strip_suffix('f').unwrap_or(v)
        } else {
            v.as_str()
        };
        let valid = if float {
            let mut parts = s.split('.');
            parts.next().unwrap().chars().all(|c| c.is_ascii_digit())
                && parts
                    .next()
                    .is_none_or(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
                && parts.next().is_none()
        } else {
            s.chars().all(|c| c.is_ascii_digit())
        };
        if !valid {
            return Err(Error::InvalidOption(format!("number format: {v}")));
        }
    }
    Ok(())
}
