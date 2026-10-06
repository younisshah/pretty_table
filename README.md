# pretty_table

Turn rows of Rust data into readable tables. Customize their appearance, sort
and filter the output, or export the same data as HTML, CSV, JSON, LaTeX, or
MediaWiki markup.

This is a Rust port of [Python PrettyTable](https://github.com/prettytable/prettytable).
It requires **Rust 1.85 or newer**. Building and using it does not require Python.

- [Get started](#get-started)
- [Add data](#add-data)
- [Change the appearance](#change-the-appearance)
- [Sort and filter](#sort-and-filter)
- [Import and export](#import-and-export)
- [Errors and feature flags](#errors-and-feature-flags)
- [Development](#development)

## Get started

The crate is available locally and has not been published to crates.io.
Add a path dependency to your application's `Cargo.toml`:

```toml
[dependencies]
pretty_table = { path = "../pretty_table" }
```

Adjust the path to point to the folder containing this crate's `Cargo.toml`.
The example above assumes your application and `pretty_table` are sibling folders.

Put this complete example in your application's `src/main.rs`:

```rust
use pretty_table::{Result, Table, row};

fn main() -> Result<()> {
    let mut table = Table::with_fields(["Name", "Age"])?;
    table.add_row(row!["Alice", 30])?;
    table.add_row(row!["Bob", 8])?;

    println!("{}", table.get_string()?);
    Ok(())
}
```

Run `cargo run` in your application:

```text
+-------+-----+
|  Name | Age |
+-------+-----+
| Alice |  30 |
|  Bob  |  8  |
+-------+-----+
```

Three things are happening:

1. `with_fields` names the columns.
2. `row!` converts different value types into cells.
3. `get_string` builds the table text; `println!` prints it.

Most methods return `Result` because an operation can fail. The `?` passes an
error back to the caller, which is why these examples use `fn main() -> Result<()>`.
Each Rust example below can be used as a separate `main.rs`.

To see the included demo, run `cargo run --bin demo` from this crate's directory.

## Add data

You can add one row with `add_row` or a batch with `add_rows`:

```rust
use pretty_table::{Result, Table, row};

fn main() -> Result<()> {
    let mut table = Table::with_fields(["Name", "Age"])?;
    table.add_rows([row!["Alice", 30], row!["Bob", 8]], false)?;
    table.add_row(row!["Carol", 42])?;

    assert_eq!(table.row_count(), 3);
    println!("{}", table.get_string()?);
    Ok(())
}
```

The final `false` means **do not add a section divider after this batch**.
Pass `true` to add one. You can also call `table.add_divider()` after a row.

Each row must have the same number of cells as the table has columns. Column
names must be unique. `Table::new()` starts without column names and generates
`Field 1`, `Field 2`, and so on from the first added row.

Cells keep their original types:

| Input to `row!` | Stored meaning |
|---|---|
| `"Alice"` or a `String` | Text |
| `30` | Signed integer |
| `30_u64` | Unsigned integer, including values above `i64::MAX` |
| `2.5` | Floating-point number |
| `true` | Boolean |
| `None::<f64>` or `Cell::None` | Missing value |

This matters when sorting: the numbers `8` and `30` sort numerically. The
strings `"8"` and `"30"` sort as text. Convert other application types, such as
dates, to strings before adding them.

## Change the appearance

### Align columns

Columns are centered by default. Use `ColumnValue::Scalar` to set every column
and `ColumnValue::Map` to change named columns:

```rust
use pretty_table::{Align, ColumnValue, Result, Table, row};
use std::collections::HashMap;

fn main() -> Result<()> {
    let mut table = Table::with_fields(["Name", "Age"])?;
    table.add_rows([row!["Alice", 30], row!["Bob", 8]], false)?;

    table.set_align(ColumnValue::Scalar(Align::Left))?;
    table.set_align(ColumnValue::Map(HashMap::from([
        ("Age".into(), Align::Right),
    ])))?;

    println!("{}", table.get_string()?);
    Ok(())
}
```

Names now align left and ages align right. The same `Scalar`/`Map` pattern works
for widths, vertical alignment, and value formatting. `ColumnValue::None` clears
a setting. Scalar settings also provide defaults for columns added later.

### Format numbers and missing values

```rust
use pretty_table::{ColumnValue, Result, Table, row};

fn main() -> Result<()> {
    let mut table = Table::with_fields(["Item", "Price"])?;
    table.add_rows([
        row!["Tea", 2.5],
        row!["Coffee", 3.0],
        row!["Water", None::<f64>],
    ], false)?;

    table.set_float_format(ColumnValue::Scalar(".2".into()))?;
    table.set_none_format(ColumnValue::Scalar("-".into()))?;

    println!("{}", table.get_string()?);
    Ok(())
}
```

```text
+--------+-------+
|  Item  | Price |
+--------+-------+
|  Tea   |  2.50 |
| Coffee |  3.00 |
| Water  |   -   |
+--------+-------+
```

`".2"` means two decimal places. For integers, `"04"` prints `7` as `0007`.
Formatting changes the displayed text, while the stored numbers keep their
types and values. The literal string `"None"` is ordinary text and is never
replaced by the missing-value setting.

For application-specific formatting, `set_custom_format` accepts a
`CustomFormat` callback: `Rc<dyn Fn(&str, &Cell) -> String>`. It receives the
column name and original cell. Setting a custom formatter replaces conflicting
numeric/null formatters for that column.

### Wrap long text

```rust
use pretty_table::{Align, ColumnValue, Result, Table, row};
use std::collections::HashMap;

fn main() -> Result<()> {
    let mut table = Table::with_fields(["Task", "Note"])?;
    table.add_row(row!["Review", "Check the source code carefully"])?;
    table.set_align(ColumnValue::Scalar(Align::Left))?;
    table.set_max_width(ColumnValue::Map(HashMap::from([
        ("Note".into(), 12),
    ])))?;

    println!("{}", table.get_string()?);
    Ok(())
}
```

```text
+--------+--------------+
| Task   | Note         |
+--------+--------------+
| Review | Check the    |
|        | source code  |
|        | carefully    |
+--------+--------------+
```

This is one data row displayed across three lines. The column setting limits
content width; padding and borders take additional space.

Use `table.set_max_table_width(Some(40))?` to limit the complete table, including
padding and borders. Unicode characters and emoji are kept whole. A limit that
cannot fit the content returns `Error::CannotFit`.

### Choose a style and color theme

```rust
use pretty_table::{Result, Table, TableStyle, Theme, Themes, row};

fn main() -> Result<()> {
    let mut table = Table::with_fields(["Job", "Status"])?;
    table.add_row(row!["Build", "Done"])?;

    table.set_theme(Some(Theme::from(Themes::Ocean)))?;
    table.set_style(TableStyle::SingleBorder)?;
    println!("{}", table.get_string()?);
    Ok(())
}
```

Apply the theme first, then the style, to use the theme's colors with the chosen
border. A theme sets glyph defaults; later style or explicit glyph changes win.
`table.set_theme(None)?` removes colors while keeping the current glyphs.

| Style | Appearance |
|---|---|
| `TableStyle::Default` | Familiar `+`, `-`, and `|` borders |
| `TableStyle::SingleBorder` | Unicode borders such as `┌─┬─┐` |
| `TableStyle::DoubleBorder` | Double-line Unicode borders |
| `TableStyle::PlainColumns` | Spaced columns without borders |
| `TableStyle::Markdown` | Markdown pipe table |
| `TableStyle::Orgmode` | Org mode table |
| `TableStyle::Rst` | reStructuredText grid table |
| `TableStyle::MswordFriendly` | Pipe-separated columns for Word |

Themes: `Default`, `DyslexiaFriendly`, `Earth`, `GlareReduction`, `HighContrast`,
`Lavender`, `Ocean`, `OceanDeep`, and `Pastel`. Use `Theme::new` for custom colors
and glyphs. Cells may also contain ANSI color codes or OSC 8 hyperlinks.

### Change options for one call

Set `table.opts.border = false` for a persistent change. Use a `_with` method
for a temporary change:

```rust
use pretty_table::{Result, Table, row};

fn main() -> Result<()> {
    let mut table = Table::with_fields(["Name"])?;
    table.add_row(row!["Alice"])?;

    println!("{}", table.get_string_with(|opts| opts.border = false)?);
    println!("{}", table.get_string()?); // Borders are back for this render.
    assert!(table.opts.border);
    Ok(())
}
```

Other common options are `title`, `header`, `padding_width`, `hrules`, and
`vrules`. `preserve_internal_border` keeps separators between columns when
the outer border is disabled.

## Sort and filter

### Sort, then show part of the result

```rust
use pretty_table::{Result, Table, row};

fn main() -> Result<()> {
    let mut table = Table::with_fields(["Name", "Age"])?;
    table.add_rows([
        row!["Alice", 30], row!["Bob", 8], row!["Carol", 42],
    ], false)?;

    let output = table.get_string_with(|opts| {
        opts.sortby = Some("Age".into());
        opts.start = 0;
        opts.end = Some(2);
    })?;
    println!("{output}"); // Bob, then Alice.
    assert!(output.contains("Bob") && !output.contains("Carol"));
    assert_eq!(table.row_count(), 3);
    Ok(())
}
```

`end` is exclusive: `start = 0, end = Some(2)` selects two rows. Set
`reversesort = true` for descending order. To display selected columns, set
`fields = Some(vec!["Name".into()])`; columns retain their original schema order.

### Keep rows that match a condition

```rust
use pretty_table::{Cell, Result, Table, row};
use std::rc::Rc;

fn main() -> Result<()> {
    let mut table = Table::with_fields(["Name", "Age"])?;
    table.add_rows([row!["Alice", 30], row!["Bob", 8]], false)?;
    table.opts.row_filter = Some(Rc::new(|row| {
        matches!(row[1], Cell::Int(age) if age >= 18)
    }));

    let output = table.get_string()?;
    println!("{output}"); // Only Alice.
    assert!(output.contains("Alice") && !output.contains("Bob"));
    Ok(())
}
```

The filter receives a whole row; here `row[1]` is its age. `Rc::new` lets the
table retain the callback. Filtering and sorting affect the rendered view;
the original rows remain stored.

The usual order is **filter → sort → slice**. `oldsortslice = true` opts into
the older slice-first behavior. Sorting removes section dividers. By default,
equal sort-column values are compared using the remaining row values. Custom
sort keys are evaluated once per row and preserve the original order on ties.

### Split the output into pages

```rust
use pretty_table::{Result, Table, row};
use std::num::NonZeroUsize;

fn main() -> Result<()> {
    let mut table = Table::with_fields(["Name"])?;
    table.add_rows([row!["Alice"], row!["Bob"], row!["Carol"]], false)?;
    let page_size = NonZeroUsize::new(2).unwrap(); // A page must contain at least one row.
    let output = table.paginate(page_size, "\n\n--- next page ---\n\n")?;
    println!("{output}"); // Two rows on the first page; one on the second.
    assert_eq!(output.matches("--- next page ---").count(), 1);
    Ok(())
}
```

## Import and export

| Output | Method | Feature required |
|---|---|---|
| Terminal text | `get_string()` | None |
| HTML | `get_html_string()` | None |
| JSON | `get_json_string()` | None |
| CSV | `get_csv_string()` | `csv` (default) |
| LaTeX | `get_latex_string()` | None |
| MediaWiki | `get_mediawiki_string()` | None |

Each exporter has a `_with` method for temporary options. You can also select
the format with `get_formatted_string(Format::Json)`, for example.

### JSON keeps numbers as numbers

```rust
use pretty_table::{Result, Table, from_json, row};

fn main() -> Result<()> {
    let mut table = Table::with_fields(["Name", "Age"])?;
    table.add_row(row!["Alice", 30])?;

    let json = table.get_json_string()?;
    println!("{json}");
    let restored = from_json(&json)?;
    assert_eq!(restored.rows()[0], row!["Alice", 30]);
    Ok(())
}
```

The JSON contains a column-name array followed by row objects:

```json
[
    ["Name", "Age"],
    {"Age": 30, "Name": "Alice"}
]
```

JSON uses the original values, even when display formatters are active.
`JsonStyle` lets you choose indentation and separators. Other output formats
use formatted cell text.

### Read and write CSV

Requires the `csv` feature, which is enabled by default.

```rust,ignore
use pretty_table::{Cell, Result, from_csv};

fn main() -> Result<()> {
    let input = "Name,Age\r\nAlice,30\r\nBob,8\r\n";
    let table = from_csv(input, b',')?;

    println!("{}", table.get_string()?);
    assert_eq!(table.get_csv_string()?, input);
    assert_eq!(table.rows()[0][1], Cell::Str("30".into()));
    Ok(())
}
```

The first record supplies the column names. CSV imports cells as strings, so
`"30"` is text rather than a number. Pass `b'\t'` for tab-separated input.
Quoted commas, embedded quotes, and multiline fields are handled by the parser.

### Read an HTML table

Requires the `html` feature, which is enabled by default. HTML **output** is
available even when this feature is disabled.

```rust,ignore
use pretty_table::{Cell, Result, from_html_one};

fn main() -> Result<()> {
    let html = "<table><tr><th>Note</th></tr>\
                <tr><td>First<br>Second</td></tr></table>";
    let table = from_html_one(html)?;

    assert_eq!(table.rows()[0][0], Cell::Str("First\nSecond".into()));
    println!("{}", table.get_string()?);
    Ok(())
}
```

`from_html_one` requires exactly one table. Use `from_html` for a list of tables,
including nested ones. `from_mediawiki` reads the simple header-and-row format
produced by the MediaWiki exporter.

HTML output escapes text by default. Its options include `format = true` for
inline layout styling and `xhtml = true` for `<br/>` line breaks. LaTeX output
emits cell content verbatim; escape special LaTeX characters in your input.

## More table operations

| Operation | What it does |
|---|---|
| `add_column(name, values, align, valign)` | Adds a column with one value per existing row |
| `add_autoindex("Index")` | Prepends row numbers starting at 1 |
| `del_row(0)` | Deletes the first row |
| `del_column("Age")` | Deletes a named column |
| `set_field_names(["Person", "Years"])` | Renames columns by position |
| `clear_rows()` | Removes rows, keeping the columns and settings |
| `clear()` | Removes rows and columns, keeping options and scalar defaults |
| `slice(0..2)` | Creates an independent table containing the first two stored rows |
| `clone()` | Copies the table and its settings |

`slice` operates on stored rows. Render options such as `start` and `end`
normally apply after filtering and sorting.

## Errors and feature flags

Common errors explain what needs changing:

| Error | Example cause |
|---|---|
| `UnknownField` | Sorting by `"Agge"` when the column is named `"Age"` |
| `RowLength` | Adding three cells to a two-column table |
| `DuplicateField` | Giving two columns the same name |
| `CannotFit` | Requesting a width too small for the data and borders |
| `Incomparable` | Sorting incompatible value types or comparing NaN |
| `JsonNonFinite` | Exporting NaN or infinity to JSON |
| `IntOutOfRange` | Importing a JSON integer beyond the supported i64/u64 range |

The default features are `csv` and `html`. To use the core and lightweight
exporters without either parser:

```toml
[dependencies]
pretty_table = { path = "../pretty_table", default-features = false }
```

Add `features = ["csv"]` or `features = ["html"]` to enable either one separately.
JSON, LaTeX, MediaWiki, text, and HTML output remain available without them.

## Development

```sh
cargo run --bin demo
cargo test --all-features
cargo test --no-default-features
cargo test --doc --all-features -- --include-ignored
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
cargo doc --no-deps --open
```

The README is included in the crate documentation. The CSV and HTML import
examples are marked `ignore` so builds without their features can still test
the other examples. The `--include-ignored` command above tests those examples
with all features enabled too.

The port targets Python PrettyTable **3.18.0**, commit
`069405f00ea1075e28dc738b8c52fd3bff173520`. Saved fixtures contain **592 reference
output comparisons**, with documented corrections for upstream bugs. Normal
Rust tests use these saved files and need neither Python nor an `upstream/` checkout.

See [compatibility notes](docs/COMPATIBILITY.md), the
[upstream test mapping](tests/MANIFEST.md), and [verification record](docs/VERIFICATION.md)
for details. The optional scripts in `tools/` regenerate maintenance artifacts.

CI is configured for Linux, macOS, and Windows with stable Rust and Rust 1.85.0.

BSD-3-Clause. See [LICENSE](LICENSE) and [third-party notices](docs/THIRD_PARTY_NOTICES.md).
