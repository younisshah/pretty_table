use pretty_table_rs::*;
use std::rc::Rc;

fn sample() -> Table {
    let mut table = Table::with_fields(["B", "A"]).unwrap();
    table.add_row(row![2, "x"]).unwrap();
    table
}

#[test]
fn json_preserves_types_and_sorts_object_keys() {
    assert_eq!(
        sample().get_json_string().unwrap(),
        "[\n    [\n        \"B\",\n        \"A\"\n    ],\n    {\n        \"A\": \"x\",\n        \"B\": 2\n    }\n]"
    );
    let mut table = Table::with_fields(["null", "bool", "float", "uint"]).unwrap();
    table
        .add_row(row![None::<i64>, true, -0.0, u64::MAX])
        .unwrap();
    let json = table
        .get_json_string_with_style(JsonStyle {
            indent: None,
            item_sep: ",".into(),
            key_sep: ":".into(),
        })
        .unwrap();
    assert_eq!(
        json,
        "[[\"null\",\"bool\",\"float\",\"uint\"],{\"bool\":true,\"float\":-0.0,\"null\":null,\"uint\":18446744073709551615}]"
    );
}

#[test]
fn json_escapes_non_ascii_surrogates_and_controls() {
    let mut table = Table::with_fields(["é"]).unwrap();
    table
        .add_row(row!["😀\"\\\n\r\t\u{8}\u{c}\0\u{1f}"])
        .unwrap();
    assert_eq!(
        table
            .get_json_string_with_style(JsonStyle {
                indent: None,
                ..JsonStyle::default()
            })
            .unwrap(),
        "[[\"\\u00e9\"],{\"\\u00e9\": \"\\ud83d\\ude00\\\"\\\\\\n\\r\\t\\b\\f\\u0000\\u001f\"}]"
    );
}

#[test]
fn json_custom_indentation_and_separators() {
    assert_eq!(
        sample()
            .get_json_string_with_style(JsonStyle {
                indent: Some(0),
                item_sep: ", ".into(),
                key_sep: " = ".into()
            })
            .unwrap(),
        "[\n[\n\"B\", \n\"A\"\n], \n{\n\"A\" = \"x\", \n\"B\" = 2\n}\n]"
    );
    assert_eq!(
        Table::new()
            .get_json_string_with(|o| o.header = false)
            .unwrap(),
        "[]"
    );
    assert_eq!(Table::new().get_json_string().unwrap(), "[\n    []\n]");
}

#[test]
fn json_escapes_delete_and_non_ascii() {
    let mut table = Table::with_fields(["x"]).unwrap();
    table.add_row(row!["\u{7f}\u{80}"]).unwrap();
    assert_eq!(
        table
            .get_json_string_with_style(JsonStyle {
                indent: None,
                ..JsonStyle::default()
            })
            .unwrap(),
        "[[\"x\"],{\"x\": \"\\u007f\\u0080\"}]"
    );
}

#[test]
fn json_keeps_tilde_and_escapes_delete_in_field_names_and_cells() {
    let mut table = Table::with_fields(["~\u{7f}"]).unwrap();
    table.add_row(row!["~\u{7f}\u{80}"]).unwrap();
    assert_eq!(
        table
            .get_json_string_with_style(JsonStyle {
                indent: None,
                item_sep: ",".into(),
                key_sep: ":".into(),
            })
            .unwrap(),
        "[[\"~\\u007f\"],{\"~\\u007f\":\"~\\u007f\\u0080\"}]"
    );
}

#[test]
fn mediawiki_explicit_empty_selection_keeps_row_separators_only() {
    assert_eq!(
        sample()
            .get_mediawiki_string_with(|o| o.fields = Some(vec![]))
            .unwrap(),
        "{| class=\"wikitable\"\n|-\n|-\n|}"
    );
}

#[test]
fn json_rejects_nonfinite_values_without_losing_data() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut table = Table::with_fields(["x"]).unwrap();
        table.add_row(row![value]).unwrap();
        assert_eq!(table.get_json_string(), Err(Error::JsonNonFinite));
        assert_eq!(table.row_count(), 1);
    }
}

#[test]
fn json_uses_raw_cells_while_display_formats_use_custom_formatters() {
    let mut table = sample();
    table
        .set_custom_format(ColumnValue::Scalar(Rc::new(|_, _| "formatted".into())))
        .unwrap();
    assert!(table.get_json_string().unwrap().contains("\"B\": 2"));
    for format in [Format::Html, Format::Latex, Format::Mediawiki] {
        assert!(
            table
                .get_formatted_string(format)
                .unwrap()
                .contains("formatted")
        );
    }
    #[cfg(feature = "csv")]
    assert_eq!(
        table.get_csv_string().unwrap(),
        "B,A\r\nformatted,formatted\r\n"
    );
}

#[cfg(feature = "csv")]
#[test]
fn csv_quotes_multiline_quotes_delimiters_and_lone_empty_fields() {
    let mut table = Table::with_fields(["a", "b"]).unwrap();
    table.add_row(row!["a,b", "say \"hi\"\nnext"]).unwrap();
    assert_eq!(
        table.get_csv_string().unwrap(),
        "a,b\r\n\"a,b\",\"say \"\"hi\"\"\nnext\"\r\n"
    );
    let mut empty = Table::with_fields([""]).unwrap();
    empty.add_row(row![""]).unwrap();
    assert_eq!(empty.get_csv_string().unwrap(), "\"\"\r\n\"\"\r\n");
    assert_eq!(Table::new().get_csv_string().unwrap(), "\r\n");
}

#[cfg(feature = "csv")]
#[test]
fn csv_supports_delimiter_options_and_core_numeric_formatting() {
    let mut table = sample();
    table
        .set_int_format(ColumnValue::Scalar("03".into()))
        .unwrap();
    assert_eq!(
        table.get_csv_string_with_delimiter(b'\t').unwrap(),
        "B\tA\r\n002\tx\r\n"
    );
    let options = Options {
        header: false,
        fields: Some(vec!["B".into()]),
        ..Options::default()
    };
    assert_eq!(
        table.get_csv_string_with_options(&options, b';').unwrap(),
        "002\r\n"
    );
    assert!(matches!(
        table.get_csv_string_with_delimiter(b'\n'),
        Err(Error::InvalidOption(_))
    ));
}

#[test]
fn html_simple_output_escapes_caption_attributes_headers_and_data() {
    let mut table = Table::with_fields(["<&\"'\n"]).unwrap();
    table.add_row(row!["<&\"'\n"]).unwrap();
    let html = table
        .get_html_string_with(|o| {
            o.xhtml = true;
            o.title = Some("<&\"'\ncaption".into());
            o.attributes = vec![("a&\"'".into(), "<&\"'".into())];
        })
        .unwrap();
    assert_eq!(
        html,
        "<table a&amp;&quot;&#x27;=\"&lt;&amp;&quot;&#x27;\">\n    <caption>&lt;&amp;&quot;&#x27;<br/>caption</caption>\n    <thead>\n        <tr>\n            <th>&lt;&amp;&quot;&#x27;<br/></th>\n        </tr>\n    </thead>\n    <tbody>\n        <tr>\n            <td>&lt;&amp;&quot;&#x27;<br/></td>\n        </tr>\n    </tbody>\n</table>"
    );
}

#[test]
fn html_raw_options_keep_markup_and_still_replace_linebreaks() {
    let mut table = Table::with_fields(["<b>A</b>\n"]).unwrap();
    table.add_row(row!["<i>x</i>\n"]).unwrap();
    let html = table
        .get_html_string_with(|o| {
            o.escape_header = false;
            o.escape_data = false;
        })
        .unwrap();
    assert!(html.contains("<th><b>A</b><br></th>"));
    assert!(html.contains("<td><i>x</i><br></td>"));
}

#[test]
fn formatted_html_preserves_attribute_order_and_alignment() {
    let mut table = sample();
    table.set_align(ColumnValue::Scalar(Align::Right)).unwrap();
    table
        .set_valign(ColumnValue::Scalar(VAlign::Bottom))
        .unwrap();
    let html = table
        .get_html_string_with(|o| {
            o.format = true;
            o.left_padding_width = Some(2);
            o.right_padding_width = Some(3);
            o.attributes = vec![("id".into(), "one".into()), ("class".into(), "two".into())];
        })
        .unwrap();
    assert!(html.starts_with("<table frame=\"box\" rules=\"cols\" id=\"one\" class=\"two\">"));
    assert!(html.contains(
        "<th style=\"padding-left: 2em; padding-right: 3em; text-align: center\">B</th>"
    ));
    assert!(html.contains("<td style=\"padding-left: 2em; padding-right: 3em; text-align: right; vertical-align: bottom\">2</td>"));
}

#[test]
fn formatted_html_rule_combinations_match_the_approved_contract() {
    for (h, v, expected) in [
        (
            HRuleStyle::All,
            VRuleStyle::All,
            " frame=\"box\" rules=\"all\"",
        ),
        (HRuleStyle::Frame, VRuleStyle::Frame, " frame=\"box\""),
        (
            HRuleStyle::Frame,
            VRuleStyle::All,
            " frame=\"box\" rules=\"cols\"",
        ),
        (HRuleStyle::Frame, VRuleStyle::None, " frame=\"hsides\""),
        (
            HRuleStyle::All,
            VRuleStyle::None,
            " frame=\"hsides\" rules=\"rows\"",
        ),
        (HRuleStyle::None, VRuleStyle::Frame, " frame=\"vsides\""),
        (
            HRuleStyle::None,
            VRuleStyle::All,
            " frame=\"vsides\" rules=\"cols\"",
        ),
        (HRuleStyle::None, VRuleStyle::None, ""),
    ] {
        let html = sample()
            .get_html_string_with(|o| {
                o.format = true;
                o.hrules = h;
                o.vrules = v;
            })
            .unwrap();
        assert!(html.starts_with(&format!("<table{expected}>")));
    }
    assert!(
        sample()
            .get_html_string_with(|o| {
                o.format = true;
                o.border = false;
                o.preserve_internal_border = true;
            })
            .unwrap()
            .starts_with("<table rules=\"cols\">")
    );
}

#[test]
fn latex_simple_output_is_crlf_and_preserves_raw_content() {
    assert_eq!(
        sample().get_latex_string().unwrap(),
        "\\begin{tabular}{cc}\r\nB & A \\\\\r\n2 & x \\\\\r\n\\end{tabular}"
    );
    assert_eq!(
        sample()
            .get_latex_string_with(|o| {
                o.header = false;
                o.fields = Some(vec!["A".into()]);
            })
            .unwrap(),
        "\\begin{tabular}{c}\r\nx \\\\\r\n\\end{tabular}"
    );
}

#[test]
fn latex_formatted_rules_follow_border_and_internal_border_options() {
    assert_eq!(
        sample()
            .get_latex_string_with(|o| {
                o.format = true;
                o.hrules = HRuleStyle::All;
            })
            .unwrap(),
        "\\begin{tabular}{|c|c|}\r\n\\hline\r\nB & A \\\\\r\n\\hline\r\n2 & x \\\\\r\n\\hline\r\n\\end{tabular}"
    );
    assert_eq!(
        sample()
            .get_latex_string_with(|o| {
                o.format = true;
                o.border = false;
                o.preserve_internal_border = true;
                o.hrules = HRuleStyle::Header;
            })
            .unwrap(),
        "\\begin{tabular}{c|c}\r\nB & A \\\\\r\n\\hline\r\n2 & x \\\\\r\n\\end{tabular}"
    );
}

#[test]
fn mediawiki_supports_caption_attributes_and_header_toggle() {
    assert_eq!(
        sample().get_mediawiki_string().unwrap(),
        "{| class=\"wikitable\"\n|-\n! B !! A\n|-\n| 2 || x\n|}"
    );
    assert_eq!(
        sample()
            .get_mediawiki_string_with(|o| {
                o.header = false;
                o.title = Some("Caption".into());
                o.attributes = vec![("id".into(), "one".into()), ("class".into(), "two".into())];
                o.fields = Some(vec!["A".into()]);
            })
            .unwrap(),
        "{| id=\"one\" class=\"two\"\n|+ Caption\n|-\n| x\n|}"
    );
}

#[test]
fn all_exports_use_immutable_shared_selection() {
    let mut table = Table::with_fields(["id", "data"]).unwrap();
    table
        .add_rows([row![3, "c"], row![1, "a"], row![2, "b"]], false)
        .unwrap();
    table.opts.sortby = Some("id".into());
    table.opts.row_filter = Some(Rc::new(|row| row[0] != Cell::Int(1)));
    table.opts.start = 1;
    table.opts.fields = Some(vec!["data".into()]);
    table.opts.header = false;
    assert_eq!(
        table.get_json_string().unwrap(),
        "[\n    {\n        \"data\": \"c\"\n    }\n]"
    );
    assert!(table.get_html_string().unwrap().contains("<td>c</td>"));
    assert_eq!(
        table.get_latex_string().unwrap(),
        "\\begin{tabular}{c}\r\nc \\\\\r\n\\end{tabular}"
    );
    assert_eq!(
        table.get_mediawiki_string().unwrap(),
        "{| class=\"wikitable\"\n|-\n| c\n|}"
    );
    #[cfg(feature = "csv")]
    assert_eq!(table.get_csv_string().unwrap(), "c\r\n");
    assert_eq!(table.rows()[0][0], Cell::Int(3));
}

#[test]
fn dispatch_and_invalid_option_errors_work_in_every_build() {
    let table = sample();
    assert_eq!(
        table.get_formatted_string(Format::Text).unwrap(),
        table.get_string().unwrap()
    );
    for format in [Format::Json, Format::Html, Format::Latex, Format::Mediawiki] {
        assert!(!table.get_formatted_string(format).unwrap().is_empty());
    }
    assert!(matches!(
        table.get_html_string_with(|o| o.fields = Some(vec!["missing".into()])),
        Err(Error::UnknownField(_))
    ));
    assert!(table.get_json_string_with(|o| o.start = 2).is_ok());
    #[cfg(not(feature = "csv"))]
    assert!(
        matches!(table.get_formatted_string(Format::Csv), Err(Error::InvalidOption(message)) if message.contains("csv") && message.contains("disabled"))
    );
    #[cfg(feature = "csv")]
    assert_eq!(
        table.get_formatted_string(Format::Csv).unwrap(),
        table.get_csv_string().unwrap()
    );
}
