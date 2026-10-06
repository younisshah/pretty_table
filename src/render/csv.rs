//! Write formatted values as CSV records, with quoting handled by the csv crate.
//! For example, the cell "Ada, Jr." must be quoted so its comma is not a separator.

use crate::{Error, Options, Result, Table};

impl Table {
    /// Export CSV using commas, necessary quoting, and CRLF record terminators.
    pub fn get_csv_string(&self) -> Result<String> {
        self.get_csv_string_with_options(&self.opts, b',')
    }
    /// Export CSV with temporary selection options.
    pub fn get_csv_string_with(&self, configure: impl FnOnce(&mut Options)) -> Result<String> {
        let mut options = self.opts.clone();
        configure(&mut options);
        self.get_csv_string_with_options(&options, b',')
    }
    /// Export CSV with a single-byte delimiter.
    pub fn get_csv_string_with_delimiter(&self, delimiter: u8) -> Result<String> {
        self.get_csv_string_with_options(&self.opts, delimiter)
    }
    /// Export CSV with both selection options and a delimiter.
    pub fn get_csv_string_with_options(&self, options: &Options, delimiter: u8) -> Result<String> {
        render(self, options, delimiter)
    }
}

pub(crate) fn validate_delimiter(delimiter: u8) -> Result<()> {
    if matches!(delimiter, b'\r' | b'\n' | b'"') {
        Err(Error::InvalidOption(
            "CSV delimiter must differ from quote and line terminators".into(),
        ))
    } else {
        Ok(())
    }
}

/// Render formatted cells as CSV, preserving the original visible field order.
pub fn render(table: &Table, options: &Options, delimiter: u8) -> Result<String> {
    validate_delimiter(delimiter)?;
    let resolved = table.resolved(options)?;
    let rows: Vec<_> = table
        .selected_rows(options)?
        .into_iter()
        .map(|(row, _)| row)
        .collect();
    let rows = table.format_rows(&rows)?;
    // csv quotes a zero-field record as a lone empty field; Python distinguishes them.
    if resolved.visible.is_empty() {
        return Ok("\r\n".repeat(rows.len() + usize::from(options.header)));
    }
    // Match Python's record endings (CRLF). Embedded quotes and line breaks
    // are escaped by the writer, rather than by joining cells with commas.
    let mut writer = csv::WriterBuilder::new()
        .delimiter(delimiter)
        .terminator(csv::Terminator::CRLF)
        .quote_style(csv::QuoteStyle::Necessary)
        .from_writer(Vec::new());
    if options.header {
        writer
            .write_record(resolved.visible.iter().map(|&i| &resolved.field_names[i]))
            .map_err(|error| Error::Parse(error.to_string()))?;
    }
    for row in rows {
        writer
            .write_record(resolved.visible.iter().map(|&i| &row[i]))
            .map_err(|error| Error::Parse(error.to_string()))?;
    }
    let bytes = writer
        .into_inner()
        .map_err(|error| Error::Parse(error.to_string()))?;
    String::from_utf8(bytes).map_err(|error| Error::Parse(error.to_string()))
}
