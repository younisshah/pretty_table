use pretty_table::*;
use std::{cell::RefCell, cmp::Ordering, collections::HashMap, num::NonZeroUsize, rc::Rc};
#[test]
fn stable_reverse_key_once() {
    let mut t = Table::with_fields(["key", "id"]).unwrap();
    t.add_rows([row![1, "a"], row![1, "b"], row![2, "c"]], true)
        .unwrap();
    let calls = Rc::new(RefCell::new(0));
    let cc = calls.clone();
    t.opts.sortby = Some("key".into());
    t.opts.reversesort = true;
    t.opts.sort_key = Some(Rc::new(move |r| {
        *cc.borrow_mut() += 1;
        vec![r[0].clone()]
    }));
    let selected = t.selected_rows(&t.opts).unwrap();
    assert_eq!(*calls.borrow(), 3);
    assert_eq!(
        selected.iter().map(|r| r.0[1].text()).collect::<Vec<_>>(),
        ["c", "a", "b"]
    );
    assert!(selected.iter().all(|r| !r.1));
}
#[test]
fn pagination_select_once() {
    let mut t = Table::with_fields(["A"]).unwrap();
    t.add_rows([row![3], row![1], row![2]], false).unwrap();
    t.opts.sortby = Some("A".into());
    let n = Rc::new(RefCell::new(0));
    let calls = n.clone();
    t.opts.sort_key = Some(Rc::new(move |r| {
        *calls.borrow_mut() += 1;
        r
    }));
    let s = t.paginate(NonZeroUsize::new(2).unwrap(), "\x0c").unwrap();
    assert_eq!(*n.borrow(), 3);
    assert_eq!(s.split('\x0c').count(), 2);
}
#[test]
fn exact_boundaries() {
    let ints = [
        i64::MIN,
        -9007199254740993,
        -1,
        0,
        1,
        9007199254740993,
        i64::MAX,
    ];
    for i in ints {
        assert_eq!(
            cmp_cells(&Cell::Int(i), &Cell::Float(f64::INFINITY)).unwrap(),
            Ordering::Less
        );
        assert_eq!(
            cmp_cells(&Cell::Int(i), &Cell::Float(f64::NEG_INFINITY)).unwrap(),
            Ordering::Greater
        );
    }
    assert_eq!(
        cmp_cells(&Cell::UInt(u64::MAX), &Cell::Float(18446744073709551616.0)).unwrap(),
        Ordering::Less
    );
    assert_eq!(
        cmp_cells(&Cell::Int(-1), &Cell::Float(-1.5)).unwrap(),
        Ordering::Greater
    );
    assert_eq!(
        cmp_cells(&Cell::Int(-1), &Cell::Float(-0.5)).unwrap(),
        Ordering::Less
    );
}
#[test]
fn number_reprs() {
    for (v, s) in [
        (1.0, "1.0"),
        (600.5, "600.5"),
        (1e16, "1e+16"),
        (1e-5, "1e-05"),
        (-0.0, "-0.0"),
    ] {
        assert_eq!(Cell::Float(v).text(), s)
    }
}
#[test]
fn format_precedence() {
    let mut t = Table::with_fields(["A", "B"]).unwrap();
    t.set_int_format(ColumnValue::Scalar("05".into())).unwrap();
    assert_eq!(t.format_cell("A", &Cell::Int(-42)).unwrap(), "-0042");
    t.set_custom_format(ColumnValue::Map(HashMap::from([(
        "A".into(),
        Rc::new(|_: &str, _: &Cell| "x".into()) as CustomFormat,
    )])))
    .unwrap();
    assert_eq!(t.format_cell("A", &Cell::Int(2)).unwrap(), "x");
    assert_eq!(t.format_cell("B", &Cell::Int(2)).unwrap(), "00002");
    for (f, s) in [
        ("6.2", "  1.25"),
        (".5", "1.25000"),
        ("06.2", "001.25"),
        ("", "1.250000"),
        ("6.2f", "  1.25f"),
    ] {
        t.set_float_format(ColumnValue::Scalar(f.into())).unwrap();
        assert_eq!(t.format_cell("A", &Cell::Float(1.25)).unwrap(), s);
    }
}
#[test]
fn hidden_height_and_floor() {
    let mut t = Table::with_fields(["A", "B"]).unwrap();
    t.add_row(row!["x", "中\n中\n中"]).unwrap();
    t.opts.fields = Some(vec!["A".into()]);
    t.set_max_width(ColumnValue::Map(HashMap::from([("B".into(), 1)])))
        .unwrap();
    assert_eq!(t.get_string().unwrap().lines().count(), 5);
    t.opts.fields = None;
    assert!(matches!(t.get_string(), Err(Error::CannotFit { .. })));
}
#[test]
fn min_and_title_conflict() {
    let mut t = Table::with_fields(["A"]).unwrap();
    t.add_row(row!["x"]).unwrap();
    t.set_max_table_width(Some(8)).unwrap();
    t.set_min_table_width(Some(9)).unwrap();
    assert_eq!(
        t.get_string(),
        Err(Error::CannotFit {
            needed: 9,
            limit: 8
        })
    );
    t.set_min_table_width(None).unwrap();
    t.opts.title = Some("long title".into());
    assert!(matches!(t.get_string(), Err(Error::CannotFit { .. })));
}
// Intentionally reversed bounds verify that the public slice API clamps invalid ranges.
#[allow(clippy::reversed_empty_ranges)]
#[test]
fn validation_and_mutation() {
    let mut t = Table::with_fields(["A", "B"]).unwrap();
    t.add_row(row![1, 2]).unwrap();
    t.opts.sortby = Some("B".into());
    t.del_column("B").unwrap();
    assert_eq!(t.get_string(), Err(Error::UnknownField("B".into())));
    t.opts.sortby = None;
    assert!(t.get_string_with(|o| o.start = 3).is_ok());
    assert!(
        t.get_string_with(|o| {
            o.start = 3;
            o.end = Some(2)
        })
        .is_err()
    );
    assert!(t.add_autoindex("A").is_err());
    assert_eq!(t.col_count(), 1);
    assert_eq!(t.slice(100..200).row_count(), 0);
    assert_eq!(t.slice(1..0).row_count(), 0);
}
#[test]
fn wrap_graphemes_sgr() {
    for text in ["中中文", "e\u{301}e\u{301}e\u{301}", "👨‍👩‍👧‍👦👨‍👩‍👧‍👦", "🇮🇳🇮🇳"]
    {
        let lines = wrap::wrap(text, 2, true).unwrap();
        for line in lines {
            assert!(display_width(&line) <= 2)
        }
    }
    let lines = wrap::wrap("\x1b[31mabcdef", 3, true).unwrap();
    assert_eq!(lines, ["\x1b[31mabc\x1b[0m", "\x1b[31mdef\x1b[0m"]);
}
#[test]
fn wrap_words_hyphens() {
    assert_eq!(
        wrap::wrap("hello world abc", 8, true).unwrap(),
        ["hello", "world", "abc"]
    );
    assert_eq!(wrap::wrap("goof-ball", 6, true).unwrap(), ["goof-", "ball"]);
    assert_eq!(
        wrap::wrap("goof-ball", 6, false).unwrap(),
        ["goof-b", "all"]
    );
}
#[test]
fn ocean_exact() {
    let mut t = Table::with_fields(["A"]).unwrap();
    t.add_row(row![1]).unwrap();
    t.set_theme(Some(Theme::from(Themes::Ocean))).unwrap();
    let p = "\x1b[36m+\x1b[0m\x1b[96m";
    let h = "\x1b[34m-\x1b[0m\x1b[96m";
    let v = "\x1b[34m|\x1b[0m\x1b[96m";
    assert_eq!(
        t.get_string().unwrap(),
        format!("{p}{h}{h}{h}{p}\n{v} A {v}\n{p}{h}{h}{h}{p}\n{v} 1 {v}\n{p}{h}{h}{h}{p}\x1b[0m")
    );
}
#[test]
fn ansi_escape_families() {
    for s in [
        "\x1bPignored\x1b\\",
        "\x1b_hidden\x1b\\",
        "\x1b^private\x1b\\",
        "\x1bXstring\x1b\\",
        "\x1b]title\x07",
        "\x1b7",
        "\x1b(B",
        "\x1b[1;2H",
    ] {
        assert_eq!(display_width(&format!("a{s}b")), 2)
    }
}
#[test]
fn minimum_internal_border() {
    let mut t = Table::with_fields(["A", "B"]).unwrap();
    t.add_row(row!["x", "y"]).unwrap();
    t.opts.border = false;
    t.opts.preserve_internal_border = true;
    t.set_min_table_width(Some(15)).unwrap();
    let widths: Vec<_> = t.get_string().unwrap().lines().map(display_width).collect();
    assert_eq!(widths, [15, 14, 15]);
}
#[test]
fn defaults_before_fields_and_rename() {
    let mut t = Table::new();
    t.set_align(ColumnValue::Scalar(Align::Left)).unwrap();
    t.set_float_format(ColumnValue::Scalar(".2".into()))
        .unwrap();
    t.set_field_names(["A", "B"]).unwrap();
    t.set_align(ColumnValue::Map(HashMap::from([(
        "B".into(),
        Align::Right,
    )])))
    .unwrap();
    t.set_field_names(["B", "A"]).unwrap();
    t.add_row(row![1.0, 2.0]).unwrap();
    assert_eq!(t.format_cell("B", &Cell::Float(1.0)).unwrap(), "1.00");
    assert!(t.get_string().unwrap().contains("| 1.00 | 2.00 |"));
}
#[test]
fn sgr_selective_resets() {
    assert_eq!(
        wrap::propagate_sgr(vec!["\x1b[34mblue\x1b[39m".into()]),
        ["\x1b[34mblue\x1b[39m"]
    );
    let lines = wrap::propagate_sgr(vec!["\x1b[38;2;0;100;0mx".into(), "y".into()]);
    assert_eq!(
        lines,
        ["\x1b[38;2;0;100;0mx\x1b[0m", "\x1b[38;2;0;100;0my\x1b[0m"]
    );
}
#[test]
fn wrap_cpython_short_hyphens_and_emdashes() {
    for (text, width, expected) in [
        ("x a-b", 4, vec!["x", "a-b"]),
        ("x aa-b", 5, vec!["x", "aa-b"]),
        ("x a-bb", 5, vec!["x", "a-bb"]),
        ("x aa-bb", 6, vec!["x aa-", "bb"]),
        ("x a-b-c", 6, vec!["x", "a-b-c"]),
        ("x ab-cd", 6, vec!["x ab-", "cd"]),
        ("12-34 56-78", 7, vec!["12-34", "56-78"]),
        ("ab--cd", 4, vec!["ab--", "cd"]),
        ("a---b", 4, vec!["a---", "b"]),
        ("x-- y", 4, vec!["x--", "y"]),
        ("hello -- world", 8, vec!["hello --", "world"]),
        (" a  b  ", 4, vec![" a", "b"]),
        ("word   verylongword", 6, vec!["word", "verylo", "ngword"]),
    ] {
        assert_eq!(wrap::wrap(text, width, true).unwrap(), expected, "{text}");
    }
}
#[test]
fn emoji_ansi_inside_grapheme_is_indivisible() {
    let text = "👨\x1b[31m‍👩‍👧‍👦\x1b[0m👨‍👩‍👧‍👦";
    let lines = wrap::wrap(text, 2, true).unwrap();
    assert_eq!(lines.len(), 2);
    assert_eq!(strip_ansi(&lines[0]), "👨‍👩‍👧‍👦");
    assert_eq!(strip_ansi(&lines[1]), "👨‍👩‍👧‍👦");
}
#[test]
fn sgr_explicit_newline_no_bleed() {
    let mut t = Table::with_fields(["A"]).unwrap();
    t.add_row(row!["\x1b[31mred\nnext\x1b[0m"]).unwrap();
    let rendered = t.get_string().unwrap();
    assert!(rendered.contains("\x1b[31mred\x1b[0m"));
    assert!(rendered.contains("\x1b[31mnext\x1b[0m"));
}
#[test]
fn pagination_empty_selection_does_not_reintroduce_rows() {
    let mut t = Table::with_fields(["A"]).unwrap();
    t.add_row(row![123]).unwrap();
    t.opts.row_filter = Some(Rc::new(|_| false));
    let s = t.paginate(NonZeroUsize::new(2).unwrap(), "\x0c").unwrap();
    assert!(!s.contains("123"));
}
#[test]
fn partial_formatter_changes_preserve_future_defaults() {
    let mut t = Table::with_fields(["A"]).unwrap();
    t.set_int_format(ColumnValue::Scalar("04".into())).unwrap();
    t.set_custom_format(ColumnValue::Map(HashMap::from([(
        "A".into(),
        Rc::new(|_: &str, _: &Cell| "x".into()) as CustomFormat,
    )])))
    .unwrap();
    t.clear();
    t.set_field_names(["B"]).unwrap();
    assert_eq!(t.format_cell("B", &Cell::Int(7)).unwrap(), "0007");
}
#[test]
fn sort_slice_filter_pipeline_and_legacy_order() {
    let mut t = Table::with_fields(["A"]).unwrap();
    t.add_rows([row![4], row![3], row![2], row![1]], false)
        .unwrap();
    t.opts.sortby = Some("A".into());
    t.opts.start = 1;
    t.opts.end = Some(3);
    assert_eq!(
        t.selected_rows(&t.opts)
            .unwrap()
            .iter()
            .map(|(r, _)| r[0].text())
            .collect::<Vec<_>>(),
        ["2", "3"]
    );
    t.opts.oldsortslice = true;
    assert_eq!(
        t.selected_rows(&t.opts)
            .unwrap()
            .iter()
            .map(|(r, _)| r[0].text())
            .collect::<Vec<_>>(),
        ["2", "3"]
    );
    t.opts.start = 0;
    t.opts.end = Some(2);
    assert_eq!(
        t.selected_rows(&t.opts)
            .unwrap()
            .iter()
            .map(|(r, _)| r[0].text())
            .collect::<Vec<_>>(),
        ["3", "4"]
    );
    t.opts.row_filter = Some(Rc::new(|r| r[0] != Cell::Int(4)));
    assert_eq!(
        t.selected_rows(&t.opts)
            .unwrap()
            .iter()
            .map(|(r, _)| r[0].text())
            .collect::<Vec<_>>(),
        ["3"]
    );
    t.opts.oldsortslice = false;
    assert_eq!(
        t.selected_rows(&t.opts)
            .unwrap()
            .iter()
            .map(|(r, _)| r[0].text())
            .collect::<Vec<_>>(),
        ["1", "2"]
    );
}
#[test]
fn default_sort_ties_compare_whole_rows() {
    let mut t = Table::with_fields(["key", "data"]).unwrap();
    t.add_rows([row![1, "z"], row![1, "a"], row![0, "q"]], false)
        .unwrap();
    t.opts.sortby = Some("key".into());
    assert_eq!(
        t.selected_rows(&t.opts)
            .unwrap()
            .iter()
            .map(|(r, _)| r[1].text())
            .collect::<Vec<_>>(),
        ["q", "a", "z"]
    );
}
#[test]
fn filtered_and_sliced_divider_stays_attached() {
    let mut t = Table::with_fields(["A"]).unwrap();
    t.add_row(row![0]).unwrap();
    t.add_row(row![1]).unwrap();
    t.add_row_divider(row![2], true).unwrap();
    t.add_row(row![3]).unwrap();
    t.opts.row_filter = Some(Rc::new(|r| r[0] != Cell::Int(0)));
    t.opts.start = 1;
    assert_eq!(
        t.selected_rows(&t.opts).unwrap(),
        [(row![2], true), (row![3], false)]
    );
}
