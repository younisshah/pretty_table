//! Export original typed values: a number stays a JSON number.
//! This format starts with field names, followed by one object for each row.

use crate::{Cell, Error, Options, Result, Table};

/// Python-compatible JSON indentation and separators; object keys are always sorted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonStyle {
    pub indent: Option<usize>,
    pub item_sep: String,
    pub key_sep: String,
}

impl Default for JsonStyle {
    fn default() -> Self {
        Self {
            indent: Some(4),
            item_sep: ",".into(),
            key_sep: ": ".into(),
        }
    }
}

impl Table {
    /// Export typed values with ASCII escapes and four-space indentation.
    pub fn get_json_string(&self) -> Result<String> {
        self.get_json_string_with_options(&self.opts, &JsonStyle::default())
    }

    /// Export typed values with temporary selection options.
    pub fn get_json_string_with(&self, configure: impl FnOnce(&mut Options)) -> Result<String> {
        let mut options = self.opts.clone();
        configure(&mut options);
        self.get_json_string_with_options(&options, &JsonStyle::default())
    }

    /// Choose indentation and separators; None gives a single line.
    ///
    /// ```
    /// use pretty_table_rs::{JsonStyle, Table, row};
    /// let mut table = Table::with_fields(["Age"]).unwrap();
    /// table.add_row(row![30]).unwrap();
    /// let json = table.get_json_string_with_style(JsonStyle {
    ///     indent: None, item_sep: ",".into(), key_sep: ":".into(),
    /// }).unwrap();
    /// assert_eq!(json, r#"[["Age"],{"Age":30}]"#);
    /// ```
    pub fn get_json_string_with_style(&self, style: JsonStyle) -> Result<String> {
        self.get_json_string_with_options(&self.opts, &style)
    }

    /// Export with both selection options and a JSON layout.
    pub fn get_json_string_with_options(
        &self,
        options: &Options,
        style: &JsonStyle,
    ) -> Result<String> {
        render(self, options, style)
    }
}

/// Render JSON without changing cell types or formatting numbers as display strings.
pub fn render(table: &Table, options: &Options, style: &JsonStyle) -> Result<String> {
    let resolved = table.resolved(options)?;
    let mut objects = Vec::new();
    if options.header {
        let headers = resolved
            .visible
            .iter()
            .map(|&i| quote(&resolved.field_names[i]))
            .collect();
        objects.push(container('[', ']', headers, style, 1));
    }
    // Headers retain schema order, but object keys are sorted to match Python
    // and make the same table produce the same JSON ordering every time.
    let mut sorted = resolved.visible.clone();
    sorted.sort_by_key(|&i| &resolved.field_names[i]);
    for (row, _) in table.selected_rows(options)? {
        let fields = sorted
            .iter()
            .map(|&i| {
                Ok(format!(
                    "{}{}{}",
                    quote(&resolved.field_names[i]),
                    style.key_sep,
                    scalar(&row[i])?
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        objects.push(container('{', '}', fields, style, 1));
    }
    Ok(container('[', ']', objects, style, 0))
}

// Do not call the display formatter here: numeric 2.5 must stay 2.5,
// even if the table displays that cell as "$2.50".
fn scalar(cell: &Cell) -> Result<String> {
    Ok(match cell {
        Cell::None => "null".into(),
        Cell::Bool(value) => value.to_string(),
        Cell::Int(value) => value.to_string(),
        Cell::UInt(value) => value.to_string(),
        Cell::Float(value) if value.is_finite() => crate::cell::float_repr(*value),
        Cell::Float(_) => return Err(Error::JsonNonFinite),
        Cell::Str(value) => quote(value),
    })
}

fn container(
    open: char,
    close: char,
    parts: Vec<String>,
    style: &JsonStyle,
    depth: usize,
) -> String {
    if parts.is_empty() {
        return format!("{open}{close}");
    }
    if let Some(width) = style.indent {
        let inner = " ".repeat(width * (depth + 1));
        let outer = " ".repeat(width * depth);
        format!(
            "{open}\n{inner}{}\n{outer}{close}",
            parts.join(&format!("{}\n{inner}", style.item_sep))
        )
    } else {
        format!("{open}{}{close}", parts.join(&style.item_sep))
    }
}

fn quote(value: &str) -> String {
    let mut output = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\u{8}' => output.push_str("\\b"),
            '\u{c}' => output.push_str("\\f"),
            ch if (' '..='~').contains(&ch) => output.push(ch),
            ch => {
                // Python's ASCII-only JSON writes é as \u00e9. Characters above
                // U+FFFF, such as 😀, need two UTF-16 escapes (a surrogate pair).
                let mut units = [0; 2];
                for unit in ch.encode_utf16(&mut units) {
                    output.push_str(&format!("\\u{unit:04x}"));
                }
            }
        }
    }
    output.push('"');
    output
}
