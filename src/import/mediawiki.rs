//! Read the simple MediaWiki table syntax emitted by this crate.
//! Caption and separator lines are skipped; header and data cells supply the values.

use crate::{Error, Result, Table};

/// Read simple MediaWiki table markup with a header row and uniformly sized body rows.
pub fn from_mediawiki(input: &str) -> Result<Table> {
    let mut inside = false;
    let mut header = None;
    let mut rows = Vec::new();
    for line in input.lines().map(str::trim) {
        if line.starts_with("{|") {
            inside = true;
            continue;
        }
        if line.starts_with("|}") {
            break;
        }
        // Ignore text outside the table, row separators (|-), and captions (|+).
        if !inside || line.starts_with("|-") || line.starts_with("|+") {
            continue;
        }
        if let Some(fields) = line.strip_prefix('!') {
            header = Some(
                fields
                    .split("!!")
                    .map(|cell| cell.trim().to_owned())
                    .collect::<Vec<_>>(),
            );
        } else if let Some(cells) = line.strip_prefix('|') {
            rows.push(
                cells
                    .split("||")
                    .map(|cell| cell.trim().to_owned())
                    .collect::<Vec<_>>(),
            );
        }
    }
    let header = header
        .ok_or_else(|| Error::Parse("No valid header found in the MediaWiki table.".into()))?;
    let mut table = Table::with_fields(header)?;
    for row in rows {
        if row.len() != table.col_count() {
            return Err(Error::Parse(
                "Row length mismatch between header and body.".into(),
            ));
        }
        table.add_row(row)?;
    }
    Ok(table)
}
