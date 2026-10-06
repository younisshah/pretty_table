//! Read the table JSON shape: [["Name", "Age"], {"Name": "Ada", "Age": 30}].
//! Read values in header order, even when object keys arrive in a different order.

use crate::{Cell, Error, Result, Table};
use serde_json::Value;

/// Read a field-name array followed by row objects, keeping scalar value types.
///
/// ```
/// use pretty_table_rs::{from_json, row};
/// let table = from_json(r#"[["Name","Age"],{"Age":30,"Name":"Ada"}]"#).unwrap();
/// assert_eq!(table.rows()[0], row!["Ada", 30]);
/// ```
pub fn from_json(input: &str) -> Result<Table> {
    let value: Value =
        serde_json::from_str(input).map_err(|error| Error::Parse(error.to_string()))?;
    let objects = value
        .as_array()
        .ok_or_else(|| Error::Parse("JSON table must be an array".into()))?;
    let headers = objects
        .first()
        .and_then(Value::as_array)
        .ok_or_else(|| Error::Parse("JSON table must start with a field-name array".into()))?;
    let fields = headers
        .iter()
        .map(|field| {
            field
                .as_str()
                .ok_or_else(|| Error::Parse("JSON field names must be strings".into()))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut table = Table::with_fields(fields)?;
    for object in &objects[1..] {
        let object = object
            .as_object()
            .ok_or_else(|| Error::Parse("JSON rows must be objects".into()))?;
        let row =
            table
                .field_names()
                .iter()
                .map(|name| {
                    scalar(object.get(name).ok_or_else(|| {
                        Error::Parse(format!("JSON row is missing field {name:?}"))
                    })?)
                })
                .collect::<Result<Vec<_>>>()?;
        table.add_row(row)?;
    }
    Ok(table)
}

fn scalar(value: &Value) -> Result<Cell> {
    Ok(match value {
        Value::Null => Cell::None,
        Value::Bool(value) => Cell::Bool(*value),
        Value::String(value) => Cell::Str(value.clone()),
        // Try exact integer storage first. u64::MAX fits UInt but not Int;
        // an even larger integer must error instead of being rounded to a float.
        Value::Number(number) => {
            if let Some(value) = number.as_i64() {
                Cell::Int(value)
            } else if let Some(value) = number.as_u64() {
                Cell::UInt(value)
            } else {
                let literal = number.to_string();
                if !literal.contains(['.', 'e', 'E']) {
                    return Err(Error::IntOutOfRange(literal));
                }
                let value = number
                    .as_f64()
                    .filter(|value| value.is_finite())
                    .ok_or_else(|| {
                        Error::Parse(format!("JSON float is out of range: {literal}"))
                    })?;
                Cell::Float(value)
            }
        }
        Value::Array(_) | Value::Object(_) => {
            return Err(Error::Parse("JSON cells must be scalar values".into()));
        }
    })
}
