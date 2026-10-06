//! Independent Python-oracle fixtures. See tools/generate_fixtures.py for provenance.
use pretty_table::{
    Align, Cell, ColumnValue, HRuleStyle, HeaderStyle, Table, TableStyle, Theme, VAlign, VRuleStyle,
};
use serde_json::Value;
use std::collections::HashMap;

fn fixtures() -> Value {
    serde_json::from_str(include_str!("data/upstream_rendering.json")).unwrap()
}

fn cell(v: &Value) -> Cell {
    match v {
        Value::Null => Cell::None,
        Value::Bool(x) => Cell::Bool(*x),
        Value::String(x) => Cell::Str(x.clone()),
        Value::Number(x) => {
            if let Some(x) = x.as_i64() {
                Cell::Int(x)
            } else if let Some(x) = x.as_u64() {
                Cell::UInt(x)
            } else {
                Cell::Float(x.as_f64().unwrap())
            }
        }
        Value::Object(x) => Cell::Float(match x["special_float"].as_str().unwrap() {
            "nan" => f64::NAN,
            "inf" => f64::INFINITY,
            "-inf" => f64::NEG_INFINITY,
            other => panic!("unexpected float {other}"),
        }),
        other => panic!("unsupported fixture cell {other}"),
    }
}

fn string(v: &Value) -> Option<String> {
    v.as_str().map(str::to_owned)
}
fn number(v: &Value) -> Option<usize> {
    v.as_u64().map(|n| n as usize)
}

fn string_map(v: &Value, fields: &[String]) -> HashMap<String, String> {
    v.as_object()
        .unwrap()
        .iter()
        .filter_map(|(k, v)| {
            fields
                .contains(k)
                .then(|| string(v).map(|v| (k.clone(), v)))
                .flatten()
        })
        .collect()
}

fn size_map(v: &Value, fields: &[String]) -> HashMap<String, usize> {
    v.as_object()
        .unwrap()
        .iter()
        .filter_map(|(k, v)| {
            fields
                .contains(k)
                .then(|| number(v).map(|v| (k.clone(), v)))
                .flatten()
        })
        .collect()
}

fn build(f: &Value) -> pretty_table::Result<Table> {
    let fields: Vec<String> = f["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().into())
        .collect();
    let mut t = Table::with_fields(fields.clone())?;
    if let Some(s) = f["style"].as_i64() {
        t.set_style(match s {
            10 => TableStyle::Default,
            11 => TableStyle::MswordFriendly,
            12 => TableStyle::PlainColumns,
            13 => TableStyle::Markdown,
            14 => TableStyle::Orgmode,
            15 => TableStyle::DoubleBorder,
            16 => TableStyle::SingleBorder,
            17 => TableStyle::Rst,
            _ => panic!("unknown style {s}"),
        })?;
    }
    for (i, row) in f["rows"].as_array().unwrap().iter().enumerate() {
        t.add_row_divider(
            row.as_array().unwrap().iter().map(cell),
            f["dividers"][i].as_bool().unwrap_or(false),
        )?;
    }
    let o = &f["options"];
    macro_rules! bools { ($($key:ident),*) => {$(if let Some(v) = o[stringify!($key)].as_bool() { t.opts.$key=v; })*}; }
    bools!(
        header,
        use_header_width,
        border,
        preserve_internal_border,
        print_empty,
        format,
        xhtml,
        escape_header,
        escape_data,
        break_on_hyphens
    );
    t.opts.title = string(&o["title"]);
    t.opts.fields = o["fields"]
        .as_array()
        .map(|v| v.iter().map(|v| v.as_str().unwrap().into()).collect());
    t.opts.padding_width = number(&o["padding_width"]).unwrap();
    t.opts.left_padding_width = number(&o["left_padding_width"]);
    t.opts.right_padding_width = number(&o["right_padding_width"]);
    t.opts.hrules = match o["hrules"].as_i64().unwrap() {
        0 => HRuleStyle::Frame,
        1 => HRuleStyle::All,
        2 => HRuleStyle::None,
        3 => HRuleStyle::Header,
        _ => unreachable!(),
    };
    t.opts.vrules = match o["vrules"].as_i64().unwrap() {
        0 => VRuleStyle::Frame,
        1 => VRuleStyle::All,
        2 => VRuleStyle::None,
        _ => unreachable!(),
    };
    macro_rules! chars { ($($key:ident),*) => {$(t.opts.$key=o[stringify!($key)].as_str().unwrap().into();)*}; }
    chars!(vertical_char, horizontal_char, junction_char);
    macro_rules! optional_chars { ($($key:ident),*) => {$(t.opts.$key=string(&o[stringify!($key)]);)*}; }
    optional_chars!(
        horizontal_align_char,
        header_horizontal_char,
        top_junction_char,
        bottom_junction_char,
        left_junction_char,
        right_junction_char,
        top_left_junction_char,
        top_right_junction_char,
        bottom_left_junction_char,
        bottom_right_junction_char
    );
    t.opts.attributes = o["attributes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| (v[0].as_str().unwrap().into(), v[1].as_str().unwrap().into()))
        .collect();
    t.set_min_table_width(number(&o["min_table_width"]))?;
    t.set_max_table_width(number(&o["max_table_width"]))?;
    t.set_header_style(string(&o["header_style"]).map(|s| match s.as_str() {
        "cap" => HeaderStyle::Cap,
        "title" => HeaderStyle::Title,
        "upper" => HeaderStyle::Upper,
        "lower" => HeaderStyle::Lower,
        _ => unreachable!(),
    }))?;
    let aligns = string_map(&o["align"], &fields)
        .into_iter()
        .map(|(k, v)| {
            (
                k,
                match v.as_str() {
                    "l" => Align::Left,
                    "r" => Align::Right,
                    _ => Align::Center,
                },
            )
        })
        .collect();
    let valigns = string_map(&o["valign"], &fields)
        .into_iter()
        .map(|(k, v)| {
            (
                k,
                match v.as_str() {
                    "t" => VAlign::Top,
                    "b" => VAlign::Bottom,
                    _ => VAlign::Middle,
                },
            )
        })
        .collect();
    t.set_align(ColumnValue::Map(aligns))?;
    t.set_valign(ColumnValue::Map(valigns))?;
    t.set_min_width(ColumnValue::Map(size_map(&o["min_width"], &fields)))?;
    t.set_max_width(ColumnValue::Map(size_map(&o["max_width"], &fields)))?;
    t.set_int_format(ColumnValue::Map(string_map(&o["int_format"], &fields)))?;
    t.set_float_format(ColumnValue::Map(string_map(&o["float_format"], &fields)))?;
    t.set_none_format(ColumnValue::Map(string_map(&o["none_format"], &fields)))?;
    if let Some(theme) = f["theme"].as_object() {
        let get = |key: &str| theme[key].as_str().unwrap();
        t.set_theme(Some(Theme::new(
            get("default_color"),
            get("vertical_char"),
            get("vertical_color"),
            get("horizontal_char"),
            get("horizontal_color"),
            get("junction_char"),
            get("junction_color"),
        )))?;
        t.opts.vertical_char = get("vertical_char").into();
        t.opts.horizontal_char = get("horizontal_char").into();
        t.opts.junction_char = get("junction_char").into();
        macro_rules! reset_fallback { ($($key:ident),*) => {$(if o[stringify!($key)]==o["junction_char"] {t.opts.$key=None;})*}; }
        reset_fallback!(
            top_junction_char,
            bottom_junction_char,
            left_junction_char,
            right_junction_char,
            top_left_junction_char,
            top_right_junction_char,
            bottom_left_junction_char,
            bottom_right_junction_char
        );
    }
    Ok(t)
}

fn check_format(format: &str, render: impl Fn(&Table, &Value) -> pretty_table::Result<String>) {
    let data = fixtures();
    let corrections: Value = serde_json::from_str(include_str!("data/corrections.json")).unwrap();
    let filter = std::env::var("PRETTY_TABLE_FIXTURE_ID").ok();
    let mut count = 0;
    let mut errors = Vec::new();
    for f in data["fixtures"].as_array().unwrap() {
        if f["format"] != format
            || filter
                .as_ref()
                .is_some_and(|id| f["id"].as_str() != Some(id))
        {
            continue;
        }
        count += 1;
        let actual = build(f).and_then(|t| render(&t, f));
        let expected = corrections[f["id"].as_str().unwrap()]["expected"]
            .as_str()
            .unwrap_or_else(|| f["expected"].as_str().unwrap());
        if actual.as_deref() != Ok(expected) {
            errors.push(serde_json::json!({"id":f["id"],"source_tests":f["source_tests"],"expected":expected,"actual":actual.as_ref().ok(),"error":actual.err().map(|e|e.to_string())}));
        }
    }
    if let Ok(path) = std::env::var("PRETTY_TABLE_DUMP_DIFFS") {
        std::fs::write(path, serde_json::to_string_pretty(&errors).unwrap()).unwrap();
    }
    assert!(count > 0, "no matching {format} fixtures");
    let details = errors
        .iter()
        .take(8)
        .map(|e| format!("{} {}", e["id"], e["source_tests"]))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        errors.is_empty(),
        "{} of {count} {format} upstream snapshots differ:\n{details}",
        errors.len()
    );
}

#[test]
fn upstream_text_snapshots() {
    check_format("get_string", |t, _| t.get_string());
}

#[test]
fn upstream_html_snapshots() {
    check_format("get_html_string", |t, _| t.get_html_string());
}

#[test]
fn upstream_latex_snapshots() {
    check_format("get_latex_string", |t, _| t.get_latex_string());
}

#[test]
fn upstream_mediawiki_snapshots() {
    check_format("get_mediawiki_string", |t, _| t.get_mediawiki_string());
}

#[cfg(feature = "csv")]
#[test]
fn upstream_csv_snapshots() {
    check_format("get_csv_string", |t, f| {
        let delimiter = f["format_options"]["delimiter"]
            .as_str()
            .map(|s| s.as_bytes()[0])
            .unwrap_or(b',');
        t.get_csv_string_with_options(&t.opts, delimiter)
    });
}

#[test]
fn upstream_json_snapshots() {
    check_format("get_json_string", |t, f| {
        let opts = &f["format_options"];
        let mut style = pretty_table::JsonStyle::default();
        if opts.get("indent").is_some() {
            style.indent = number(&opts["indent"]);
        }
        if let Some(separators) = opts["separators"].as_array() {
            style.item_sep = separators[0].as_str().unwrap().into();
            style.key_sep = separators[1].as_str().unwrap().into();
        }
        t.get_json_string_with_options(&t.opts, &style)
    });
}
