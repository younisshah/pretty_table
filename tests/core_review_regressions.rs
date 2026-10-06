use pretty_table_rs::*;
use std::rc::Rc;
#[test]
fn zero_column_rows_reject_new_schema_atomically() {
    for deleted in [false, true] {
        let mut t = if deleted {
            Table::with_fields(["A"]).unwrap()
        } else {
            Table::new()
        };
        if deleted {
            t.add_row(row![1]).unwrap();
            t.del_column("A").unwrap();
        } else {
            t.add_row(Vec::<Cell>::new()).unwrap();
        }
        let before = t.rows().to_vec();
        assert_eq!(
            t.set_field_names(["A"]),
            Err(Error::RowLength {
                expected: 0,
                actual: 1
            })
        );
        assert_eq!(
            t.add_row(row![1]),
            Err(Error::RowLength {
                expected: 0,
                actual: 1
            })
        );
        assert!(t.field_names().is_empty());
        assert_eq!(t.rows(), before);
        assert!(t.get_string().is_ok());
        t.add_row(Vec::<Cell>::new()).unwrap();
        assert_eq!(t.row_count(), 2);
        t.add_column("A", [1, 2], Align::Center, VAlign::Top)
            .unwrap();
        assert_eq!(t.rows(), [row![1], row![2]]);
    }
}
#[test]
fn maximum_accounts_for_every_title_and_fieldless_line() {
    for border in [false, true] {
        for vrules in [VRuleStyle::All, VRuleStyle::Frame, VRuleStyle::None] {
            for limit in 1..14 {
                let mut t = Table::with_fields(["A"]).unwrap();
                t.add_row(row!["x"]).unwrap();
                t.opts.border = border;
                t.opts.vrules = vrules;
                t.opts.title = Some("abc".into());
                t.set_max_table_width(Some(limit)).unwrap();
                match t.get_string() {
                    Ok(s) => assert!(
                        s.lines().all(|line| display_width(line) <= limit),
                        "limit={limit}, border={border}, vrules={vrules:?}, {s:?}"
                    ),
                    Err(Error::CannotFit { needed, limit: l }) => {
                        assert!(needed > l);
                        assert_eq!(l, limit)
                    }
                    Err(e) => panic!("{e}"),
                }
            }
        }
    }
    let mut t = Table::new();
    t.set_max_table_width(Some(1)).unwrap();
    assert!(matches!(
        t.get_string(),
        Err(Error::CannotFit {
            needed: 2,
            limit: 1
        })
    ));
}
#[test]
fn colored_spaces_do_not_make_empty_wrap_lines() {
    assert_eq!(
        wrap::wrap("ab\x1b[31m cd", 2, true).unwrap(),
        ["ab", "\x1b[31mcd\x1b[0m"]
    );
    assert_eq!(
        wrap::wrap("ab\x1b[31m  \x1b[39mcd", 2, true).unwrap(),
        ["ab", "\x1b[31m\x1b[39mcd"]
    );
}
#[test]
fn truncated_header_closes_sgr_before_padding() {
    let mut t = Table::with_fields(["\x1b[31mABC\x1b[0m"]).unwrap();
    t.add_row(row!["x"]).unwrap();
    t.set_max_table_width(Some(5)).unwrap();
    assert!(t.get_string().unwrap().contains("| \x1b[31mA\x1b[0m |"));
}
#[test]
fn sgr_colon_rgb_and_intensity() {
    assert_eq!(
        wrap::wrap("\x1b[38:2::255:0:0mabcdef", 3, true).unwrap(),
        [
            "\x1b[38:2::255:0:0mabc\x1b[0m",
            "\x1b[38:2::255:0:0mdef\x1b[0m"
        ]
    );
    assert_eq!(
        wrap::propagate_sgr(vec![
            "\x1b[1;2mabc".into(),
            "def".into(),
            "\x1b[22mplain".into()
        ]),
        [
            "\x1b[1;2mabc\x1b[0m",
            "\x1b[1;2mdef\x1b[0m",
            "\x1b[1;2m\x1b[22mplain"
        ]
    );
}
#[test]
fn malformed_escape_preserves_utf8() {
    for text in ["\x1b中", "a\x1b🙂b", "\x1b\u{e9}"] {
        assert_eq!(
            display_width(text),
            display_width(&text.replace('\x1b', ""))
        );
        assert!(wrap::wrap(text, 3, true).is_ok());
        assert_eq!(expand_tabs(text), text);
    }
}
#[test]
fn defaults_apply_through_add_column() {
    let mut t = Table::new();
    t.set_int_format(ColumnValue::Scalar("04".into())).unwrap();
    t.set_float_format(ColumnValue::Scalar(".2".into()))
        .unwrap();
    t.set_none_format(ColumnValue::Scalar("NULL".into()))
        .unwrap();
    t.set_min_width(ColumnValue::Scalar(7)).unwrap();
    t.add_column(
        "A",
        [Cell::Int(7), Cell::Float(1.5), Cell::None],
        Align::Left,
        VAlign::Bottom,
    )
    .unwrap();
    assert_eq!(t.format_cell("A", &Cell::Int(7)).unwrap(), "0007");
    assert_eq!(t.format_cell("A", &Cell::Float(1.5)).unwrap(), "1.50");
    assert_eq!(t.format_cell("A", &Cell::None).unwrap(), "NULL");
    assert!(
        t.get_string()
            .unwrap()
            .lines()
            .all(|s| display_width(s) == 11)
    );
    let mut t = Table::new();
    t.set_custom_format(ColumnValue::Scalar(Rc::new(|_, _| "CUSTOM".into())))
        .unwrap();
    t.add_column("A", [1], Align::Center, VAlign::Top).unwrap();
    assert_eq!(t.format_cell("A", &Cell::Int(1)).unwrap(), "CUSTOM");
}
#[test]
fn tabs_follow_python_scalar_positions() {
    for (text, expected) in [
        ("中\tx", "中       x"),
        ("👨‍👩‍👧‍👦\tx", "👨‍👩‍👧‍👦 x"),
        ("ab\x1b[31m\tx", "ab\x1b[31m x"),
        ("ab\r中\tx", "ab\r中       x"),
        ("🙂\n\tx", "🙂\n        x"),
    ] {
        assert_eq!(expand_tabs(text), expected);
    }
}
#[test]
fn formatted_nan_matches_python() {
    let mut t = Table::with_fields(["A"]).unwrap();
    for (spec, want) in [
        (".2", "nan"),
        ("6.2", "   nan"),
        ("06.2", "000nan"),
        ("6.2f", "   nanf"),
    ] {
        t.set_float_format(ColumnValue::Scalar(spec.into()))
            .unwrap();
        assert_eq!(t.format_cell("A", &Cell::Float(f64::NAN)).unwrap(), want);
    }
}
#[test]
fn leading_discardable_spaces_do_not_block_a_long_word() {
    assert_eq!(wrap::wrap("  abc", 2, true).unwrap(), ["ab", "c"]);
    assert_eq!(
        wrap::wrap("\x1b[31m  abc", 2, true).unwrap(),
        ["\x1b[31mab\x1b[0m", "\x1b[31mc\x1b[0m"]
    );
    assert_eq!(
        wrap::wrap(" \x1b[31m abc", 2, true).unwrap(),
        ["\x1b[31mab\x1b[0m", "\x1b[31mc\x1b[0m"]
    );
    for text in ["  abc", "\x1b[31m  abc\x1b[0m"] {
        let mut t = Table::with_fields(["A"]).unwrap();
        t.add_row(row![text]).unwrap();
        t.opts.use_header_width = false;
        t.set_max_width(ColumnValue::Scalar(2)).unwrap();
        let s = t.get_string().unwrap();
        assert_eq!(s.lines().count(), 6);
        assert!(s.lines().all(|s| display_width(s) == 6));
        assert!(strip_ansi(&s).contains("ab"));
    }
    assert!(matches!(
        wrap::wrap(" 中", 1, true),
        Err(Error::CannotFit {
            needed: 2,
            limit: 1
        })
    ));
}
#[test]
fn fieldless_one_space_fits_one_column() {
    let mut t = Table::new();
    t.add_row(Vec::<Cell>::new()).unwrap();
    t.opts.border = false;
    t.opts.preserve_internal_border = true;
    t.opts.header = false;
    assert_eq!(t.get_string().unwrap(), " ");
    t.set_max_table_width(Some(1)).unwrap();
    assert_eq!(t.get_string().unwrap(), " ");
    t.opts.header = true;
    assert!(matches!(
        t.get_string(),
        Err(Error::CannotFit {
            needed: 2,
            limit: 1
        })
    ));
    let mut default = Table::new();
    default.set_max_table_width(Some(1)).unwrap();
    assert!(matches!(
        default.get_string(),
        Err(Error::CannotFit {
            needed: 2,
            limit: 1
        })
    ));
}
