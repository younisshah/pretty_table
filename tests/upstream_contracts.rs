//! Native equivalents of upstream model/validation tests that do not render a snapshot.
use pretty_table_rs::*;
use std::{collections::HashMap, rc::Rc};

#[test]
fn rows_columns_counts_and_failed_mutations() {
    let mut t = Table::new();
    assert_eq!((t.row_count(), t.col_count()), (0, 0));
    t.add_row(row![1, "a"]).unwrap();
    t.add_row(row![2, "b"]).unwrap();
    assert_eq!(t.field_names(), ["Field 1", "Field 2"]);
    assert_eq!((t.row_count(), t.col_count()), (2, 2));
    assert_eq!(t.rows(), [row![1, "a"], row![2, "b"]]);
    let before = t.get_string().unwrap();
    assert!(t.add_row(row![1]).is_err());
    assert!(
        t.add_column("extra", [1], Align::Center, VAlign::Top)
            .is_err()
    );
    assert!(t.del_row(2).is_err());
    assert!(t.del_column("missing").is_err());
    assert_eq!(t.get_string().unwrap(), before);
    t.del_column("Field 2").unwrap();
    t.del_row(0).unwrap();
    assert_eq!(t.rows(), [row![2]]);
    assert_eq!(t.field_names(), ["Field 1"]);
}

#[test]
fn bulk_insert_is_atomic_and_empty_insert_is_noop() {
    let mut t = Table::with_fields(["A"]).unwrap();
    t.add_rows(Vec::<Vec<Cell>>::new(), true).unwrap();
    assert_eq!(t.row_count(), 0);
    assert!(t.add_rows(vec![row![1], row![2, 3]], false).is_err());
    assert_eq!(t.row_count(), 0);
    t.add_rows(vec![row![1], row![2]], true).unwrap();
    assert_eq!(t.dividers(), [false, true]);
}

#[test]
fn clear_operations_remove_dividers_and_preserve_style() {
    let mut t = Table::with_fields(["A"]).unwrap();
    t.opts.border = false;
    t.add_row_divider(row![1], true).unwrap();
    t.clear_rows();
    assert_eq!(t.field_names(), ["A"]);
    assert!(t.rows().is_empty() && t.dividers().is_empty());
    assert!(!t.opts.border);
    t.add_row_divider(row![2], true).unwrap();
    t.clear();
    assert!(t.field_names().is_empty() && t.rows().is_empty() && t.dividers().is_empty());
    assert!(!t.opts.border);
}

#[test]
fn invalid_selections_and_rule_characters_are_errors() {
    let mut t = Table::with_fields(["A"]).unwrap();
    t.add_row(row![1]).unwrap();
    assert!(matches!(
        t.get_string_with(|o| o.sortby = Some("missing".into())),
        Err(Error::UnknownField(_))
    ));
    assert!(matches!(
        t.get_string_with(|o| o.fields = Some(vec!["missing".into()])),
        Err(Error::UnknownField(_))
    ));
    assert!(
        t.get_string_with(|o| {
            o.start = 3;
            o.end = Some(1)
        })
        .is_err()
    );
    assert!(
        t.get_string_with(|o| o.vertical_char = "wide".into())
            .is_err()
    );
    assert!(t.get_string().is_ok());
}

#[test]
fn reverse_sort_keeps_equal_keys_stable_and_evaluates_key_once() {
    let mut t = Table::with_fields(["Key", "Value"]).unwrap();
    t.add_rows(
        vec![row![1, "a"], row![2, "b"], row![1, "c"], row![2, "d"]],
        false,
    )
    .unwrap();
    let calls = Rc::new(std::cell::Cell::new(0));
    let captured = calls.clone();
    t.opts.sortby = Some("Key".into());
    t.opts.reversesort = true;
    t.opts.sort_key = Some(Rc::new(move |row| {
        captured.set(captured.get() + 1);
        vec![row[0].clone()]
    }));
    let selected = t.selected_rows(&t.opts).unwrap();
    assert_eq!(
        selected.into_iter().map(|r| r.0).collect::<Vec<_>>(),
        vec![row![2, "b"], row![2, "d"], row![1, "a"], row![1, "c"]]
    );
    assert_eq!(calls.get(), 4);
    assert_eq!(t.rows()[0], row![1, "a"]);
}

#[test]
fn renamed_settings_follow_column_position_and_slices_are_independent() {
    let mut t = Table::with_fields(["A", "B"]).unwrap();
    t.add_row(row!["x", "y"]).unwrap();
    t.set_min_width(ColumnValue::Scalar(3)).unwrap();
    t.set_align(ColumnValue::Map(HashMap::from([
        ("A".into(), Align::Left),
        ("B".into(), Align::Right),
    ])))
    .unwrap();
    t.set_field_names(["B", "A"]).unwrap();
    assert!(t.get_string().unwrap().contains("| x   |   y |"));
    let mut copy = t.slice(..);
    copy.set_align(ColumnValue::Scalar(Align::Center)).unwrap();
    assert_ne!(copy.get_string().unwrap(), t.get_string().unwrap());
    assert!(t.get_string().unwrap().contains("| x   |   y |"));
}

#[test]
fn formatter_setters_override_conflicting_formats() {
    let mut t = Table::with_fields(["N"]).unwrap();
    t.add_row(row![2.5]).unwrap();
    t.set_float_format(ColumnValue::Scalar(".2".into()))
        .unwrap();
    assert!(t.get_string().unwrap().contains("2.50"));
    let custom: CustomFormat = Rc::new(|_, _| "CUSTOM".into());
    t.set_custom_format(ColumnValue::Scalar(custom)).unwrap();
    assert!(t.get_string().unwrap().contains("CUSTOM"));
    t.set_float_format(ColumnValue::Scalar(".1".into()))
        .unwrap();
    assert!(t.get_string().unwrap().contains("2.5"));
    assert!(!t.get_string().unwrap().contains("CUSTOM"));
}

#[test]
fn theme_codes_normalize_and_setting_theme_changes_output() {
    for (input, expected) in [
        ("", ""),
        ("   ", ""),
        ("34", "\x1b[34m"),
        ("38;5;22", "\x1b[38;5;22m"),
        ("\x1b[31m", "\x1b[31m"),
    ] {
        assert_eq!(Theme::format_code(input), expected);
    }
    let mut t = Table::with_fields(["A"]).unwrap();
    t.add_row(row![1]).unwrap();
    let plain = t.get_string().unwrap();
    t.set_theme(Some(Theme::from(Themes::Ocean))).unwrap();
    let colored = t.get_string().unwrap();
    assert_ne!(colored, plain);
    assert_eq!(
        colored
            .lines()
            .map(strip_ansi)
            .collect::<Vec<_>>()
            .join("\n"),
        plain
    );
    t.set_theme(None).unwrap();
    assert_eq!(t.get_string().unwrap(), plain);
}

#[test]
fn complete_upstream_width_cases() {
    for (text, expected) in [
        ("a", 1),
        ("abc", 3),
        ("abc def", 7),
        ("\x1b[34mblue\x1b[39m", 4),
        ("\x1b]8;;https://example.com\x1b\\link\x1b]8;;\x1b\\", 4),
        (
            "\x1b]8;;https://example.com\x1b\\\x1b[34mblue link\x1b[39m\x1b]8;;\x1b\\",
            9,
        ),
        (
            "\x1b[34m\x1b]8;;https://example.com\x1b\\blue link\x1b]8;;\x1b\\\x1b[39m",
            9,
        ),
        ("中文", 4),
        ("cafe\u{301}", 4),
        ("👨‍👩‍👧", 2),
        ("☺️", 2),
        ("🇺🇸", 2),
        ("abc\x07def", 6),
    ] {
        assert_eq!(display_width(text), expected, "{text:?}");
    }
}
