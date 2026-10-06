//! Read tables from HTML while keeping each nested table independent.
//! HTML entities become text, and <br> becomes a newline inside its cell.

use crate::{Error, Result, Table};
use scraper::{ElementRef, Html, Node, Selector};
use std::collections::HashSet;

/// Read each HTML table independently, retaining only rows and cells owned by that table.
pub fn from_html(input: &str) -> Result<Vec<Table>> {
    let document = Html::parse_fragment(input);
    let tables = Selector::parse("table").expect("static table selector");
    let rows = Selector::parse("tr").expect("static row selector");
    let cells = Selector::parse("th, td").expect("static cell selector");
    document
        .select(&tables)
        .map(|table| {
            let mut parsed = Vec::new();
            let mut width = 0;
            for row in table.select(&rows).filter(|row| owned_by(*row, table)) {
                let mut values = Vec::new();
                let mut header = false;
                for cell in row.select(&cells).filter(|cell| owned_by(*cell, table)) {
                    header |= cell.value().name() == "th";
                    let span = if let Some(value) = cell.attr("colspan") {
                        let value = value.trim();
                        let digits = value.strip_prefix('+').unwrap_or(value);
                        if !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()) {
                            value
                                .parse::<usize>()
                                .map_err(|_| {
                                    Error::Parse(format!("HTML colspan is out of range: {value}"))
                                })?
                                .max(1)
                        } else {
                            1
                        }
                    } else {
                        1
                    };
                    // colspan=3 becomes ["value", "", ""] in our rectangular
                    // data model; it does not create a merged display cell.
                    values.push(cell_text(cell).trim().to_owned());
                    let length = values
                        .len()
                        .checked_add(span - 1)
                        .ok_or_else(|| Error::Parse("HTML colspan is too large".into()))?;
                    values.try_reserve(span - 1).map_err(|error| {
                        Error::Parse(format!("HTML colspan cannot fit in memory: {error}"))
                    })?;
                    values.resize(length, String::new());
                }
                width = width.max(values.len());
                parsed.push((values, header));
            }
            for (values, _) in &mut parsed {
                values.resize(width, String::new());
            }
            let header = parsed
                .iter()
                .rev()
                .find(|(_, header)| *header)
                .map(|(fields, _)| fields.clone());
            let mut result = if let Some(mut fields) = header {
                // Table columns need unique names: repeated "Name" headers
                // become "Name", "Name'", "Name''", and so on.
                let mut seen = HashSet::new();
                for field in &mut fields {
                    while !seen.insert(field.clone()) {
                        field.push('\'');
                    }
                }
                Table::with_fields(fields)?
            } else {
                Table::new()
            };
            for (values, header) in parsed {
                if !header {
                    result.add_row(values)?;
                }
            }
            Ok(result)
        })
        .collect()
}

/// Read exactly one HTML table; zero or multiple tables are an error.
///
/// ```
/// use pretty_table_rs::{Cell, from_html_one};
/// let table = from_html_one(
///     "<table><tr><th>Note</th></tr><tr><td>First<br>Second</td></tr></table>"
/// ).unwrap();
/// assert_eq!(table.rows()[0][0], Cell::Str("First\nSecond".into()));
/// ```
pub fn from_html_one(input: &str) -> Result<Table> {
    let mut tables = from_html(input)?;
    if tables.len() != 1 {
        return Err(Error::Parse(format!(
            "Expected one HTML table, found {}. Use from_html for multiple tables.",
            tables.len()
        )));
    }
    Ok(tables.remove(0))
}

// A row inside an inner table must not also become a row of its outer table.
fn owned_by(element: ElementRef<'_>, table: ElementRef<'_>) -> bool {
    element
        .ancestors()
        .filter_map(ElementRef::wrap)
        .find(|ancestor| ancestor.value().name() == "table")
        .is_some_and(|ancestor| ancestor == table)
}

fn cell_text(cell: ElementRef<'_>) -> String {
    let mut output = String::new();
    // Explicit depth-first traversal avoids recursive stack growth on deeply nested markup.
    // This is a stack, so push children backwards to read them left-to-right.
    // For <b>A</b><i>B</i>, the result should be "AB", not "BA".
    let mut pending: Vec<_> = cell.children().rev().collect();
    while let Some(node) = pending.pop() {
        match node.value() {
            Node::Text(text) => output.push_str(text),
            Node::Element(element) if element.name() == "table" => {}
            Node::Element(element) if element.name() == "br" => output.push('\n'),
            _ => pending.extend(node.children().rev()),
        }
    }
    output
}
