use pretty_table::*;
#[test]
fn basic_text() {
    let mut t = Table::with_fields(["A", "B"]).unwrap();
    t.add_row(row![1, "x"]).unwrap();
    assert_eq!(
        t.get_string().unwrap(),
        "+---+---+\n| A | B |\n+---+---+\n| 1 | x |\n+---+---+"
    );
    assert_eq!(
        t.get_string_with(|o| o.vertical_char = "!".into()).unwrap(),
        "+---+---+\n! A ! B !\n+---+---+\n! 1 ! x !\n+---+---+"
    );
    assert_eq!(t.opts.vertical_char, "|");
}
#[test]
fn unicode_width() {
    for (s, w) in [
        ("abc\x07def", 6),
        ("abc def\x1b(B", 7),
        ("\x1b[m", 0),
        ("👨‍👩‍👧‍👦", 2),
        ("🇮🇳", 2),
        ("café", 4),
        ("中", 2),
        ("e\u{301}", 1),
        ("\x1b]8;;https://x\x1b\\hello\x1b]8;;\x1b\\", 5),
    ] {
        assert_eq!(display_width(s), w, "{s:?}");
    }
}
#[test]
fn floor_and_shrink() {
    let mut t = Table::with_fields(["A", "B"]).unwrap();
    t.add_row(row!["中", "abcdef"]).unwrap();
    t.set_max_table_width(Some(10)).unwrap();
    for line in t.get_string().unwrap().lines() {
        assert_eq!(display_width(line), 10)
    }
    let mut t = Table::with_fields(["中文"]).unwrap();
    t.add_row(row!["中"]).unwrap();
    t.set_max_table_width(Some(6)).unwrap();
    assert!(t.get_string().unwrap().contains(" 中 "));
    t.set_max_table_width(Some(5)).unwrap();
    assert!(matches!(t.get_string(), Err(Error::CannotFit { .. })));
}
#[test]
fn nulls() {
    let mut t = Table::with_fields(["A"]).unwrap();
    t.set_none_format(ColumnValue::Scalar("X".into())).unwrap();
    t.add_row(row![Cell::None]).unwrap();
    t.add_row(row!["None"]).unwrap();
    let s = t.get_string().unwrap();
    assert!(s.contains("None"));
    assert!(s.contains(" X "));
}
#[test]
fn style_reset() {
    let mut t = Table::with_fields(["A"]).unwrap();
    t.add_row(row![1]).unwrap();
    let default = t.get_string().unwrap();
    t.set_style(TableStyle::Orgmode).unwrap();
    t.set_style(TableStyle::Default).unwrap();
    assert_eq!(t.get_string().unwrap(), default);
}
