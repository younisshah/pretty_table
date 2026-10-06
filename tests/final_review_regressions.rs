use pretty_table::*;

fn plain_lines(text: &str) -> String {
    text.lines().map(strip_ansi).collect::<Vec<_>>().join("\n")
}

#[test]
fn theme_applies_custom_defaults_then_retains_later_stored_glyphs() {
    let mut t = Table::with_fields(["A"]).unwrap();
    t.add_row(row![1]).unwrap();
    t.set_theme(Some(Theme::new("96", "!", "34", "=", "34", "*", "36")))
        .unwrap();
    assert_eq!(t.opts.vertical_char, "!");
    assert_eq!(t.opts.horizontal_char, "=");
    assert_eq!(t.opts.junction_char, "*");
    assert_eq!(
        plain_lines(&t.get_string().unwrap()),
        "*===*\n! A !\n*===*\n! 1 !\n*===*"
    );

    t.opts.vertical_char = ":".into();
    t.opts.horizontal_char = "~".into();
    t.opts.junction_char = "#".into();
    let colored = t.get_string().unwrap();
    assert_eq!(plain_lines(&colored), "#~~~#\n: A :\n#~~~#\n: 1 :\n#~~~#");
    assert!(colored.contains("\x1b[34m:\x1b[0m\x1b[96m"));
    t.set_theme(None).unwrap();
    assert_eq!(t.get_string().unwrap(), plain_lines(&colored));
}

#[test]
fn themed_one_call_glyphs_color_all_inherited_junctions_without_mutation() {
    let mut t = Table::with_fields(["A", "B"]).unwrap();
    t.add_rows([row![1, 2], row![3, 4]], false).unwrap();
    t.opts.hrules = HRuleStyle::All;
    t.set_theme(Some(Theme::from(Themes::Ocean))).unwrap();
    let before = t.get_string().unwrap();
    let rendered = t
        .get_string_with(|o| {
            o.vertical_char = "!".into();
            o.horizontal_char = "=".into();
            o.junction_char = "*".into();
        })
        .unwrap();
    assert_eq!(
        plain_lines(&rendered),
        "*===*===*\n! A ! B !\n*===*===*\n! 1 ! 2 !\n*===*===*\n! 3 ! 4 !\n*===*===*"
    );
    for colored in [
        "\x1b[34m!\x1b[0m\x1b[96m",
        "\x1b[34m=\x1b[0m\x1b[96m",
        "\x1b[36m*\x1b[0m\x1b[96m",
    ] {
        assert!(rendered.contains(colored));
    }
    assert_eq!(t.get_string().unwrap(), before);
    assert_eq!(t.opts.vertical_char, "|");
    assert_eq!(t.opts.horizontal_char, "-");
    assert_eq!(t.opts.junction_char, "+");
}

#[test]
fn style_after_theme_wins_and_invalid_theme_keeps_state() {
    let mut t = Table::with_fields(["A", "B"]).unwrap();
    t.add_row(row![1, 2]).unwrap();
    t.set_theme(Some(Theme::from(Themes::Ocean))).unwrap();
    t.set_style(TableStyle::DoubleBorder).unwrap();
    let colored = t.get_string().unwrap();
    assert_eq!(
        plain_lines(&colored),
        "╔═══╦═══╗\n║ A ║ B ║\n╠═══╬═══╣\n║ 1 ║ 2 ║\n╚═══╩═══╝"
    );
    for glyph in ["\x1b[34m║", "\x1b[34m═", "\x1b[36m╬"] {
        assert!(colored.contains(glyph));
    }
    assert!(
        t.set_theme(Some(Theme::new("", "!", "", "=", "", "中", "")))
            .is_err()
    );
    assert_eq!(t.get_string().unwrap(), colored);
    t.set_theme(None).unwrap();
    assert_eq!(t.get_string().unwrap(), plain_lines(&colored));
}

#[test]
fn orgmode_preserves_complete_ansi_edges_in_both_application_orders() {
    let p = "\x1b[36m|\x1b[0m\x1b[96m";
    let h = "\x1b[34m-\x1b[0m\x1b[96m";
    let v = "\x1b[34m|\x1b[0m\x1b[96m";
    let expected =
        format!("{p}{h}{h}{h}{p}\n{v} A {v}\n{p}{h}{h}{h}{p}\n{v} 1 {v}\n{p}{h}{h}{h}{p}\x1b[0m");
    for theme_first in [false, true] {
        let mut t = Table::with_fields(["A"]).unwrap();
        t.add_row(row![1]).unwrap();
        if theme_first {
            t.set_theme(Some(Theme::from(Themes::Ocean))).unwrap();
            t.set_style(TableStyle::Orgmode).unwrap();
        } else {
            t.set_style(TableStyle::Orgmode).unwrap();
            t.set_theme(Some(Theme::from(Themes::Ocean))).unwrap();
        }
        assert_eq!(
            t.get_string().unwrap(),
            expected,
            "theme_first={theme_first}"
        );
        assert_eq!(plain_lines(&expected), "|---|\n| A |\n|---|\n| 1 |\n|---|");
    }
}

#[test]
fn orgmode_colored_edges_fit_valid_maximum_widths() {
    for theme_first in [false, true] {
        for limit in [5, 6, 10] {
            let mut t = Table::with_fields(["A"]).unwrap();
            t.add_row(row![1]).unwrap();
            if theme_first {
                t.set_theme(Some(Theme::from(Themes::Ocean))).unwrap();
                t.set_style(TableStyle::Orgmode).unwrap();
            } else {
                t.set_style(TableStyle::Orgmode).unwrap();
                t.set_theme(Some(Theme::from(Themes::Ocean))).unwrap();
            }
            t.set_max_table_width(Some(limit)).unwrap();
            let text = t.get_string().unwrap();
            assert_eq!(plain_lines(&text), "|---|\n| A |\n|---|\n| 1 |\n|---|");
            assert!(text.lines().all(|line| display_width(line) <= limit));
        }
    }
}

#[test]
fn orgmode_replaces_whole_ansi_wrapped_edge_graphemes() {
    let mut t = Table::with_fields(["A"]).unwrap();
    t.add_row(row![1]).unwrap();
    t.set_style(TableStyle::Orgmode).unwrap();
    t.set_theme(Some(Theme::new(
        "96", "e\u{301}", "34", "-", "34", "e\u{301}", "36",
    )))
    .unwrap();
    let rendered = t.get_string().unwrap();
    assert_eq!(plain_lines(&rendered), "|---|\n| A |\n|---|\n| 1 |\n|---|");
    assert!(rendered.starts_with("\x1b[36m|\x1b[0m\x1b[96m"));
}
