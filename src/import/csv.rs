//! Read the first record as headers and the following records as string cells.
//! A CSV cell containing 30 stays text; CSV does not declare numeric types.

use crate::{Error, Result, Table};

/// Read a CSV header and string cells, trimming spaces and checking row lengths.
///
/// ```
/// use pretty_table::{Cell, from_csv};
/// let table = from_csv("Name,Age\nAda,30\n", b',').unwrap();
/// assert_eq!(table.rows()[0][1], Cell::Str("30".into()));
/// ```
pub fn from_csv(input: &str, delimiter: u8) -> Result<Table> {
    crate::render::csv::validate_delimiter(delimiter)?;
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .flexible(false)
        .from_reader(input.as_bytes());
    let fields = reader
        .headers()
        .map_err(|error| Error::Parse(error.to_string()))?;
    if fields.is_empty() {
        return Err(Error::Parse("CSV input has no header".into()));
    }
    let mut table = Table::with_fields(fields.iter().map(str::trim))?;
    for record in reader.records() {
        let record = record.map_err(|error| Error::Parse(error.to_string()))?;
        table.add_row(record.iter().map(str::trim))?;
    }
    Ok(table)
}
