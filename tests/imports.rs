use pretty_table::*;

#[test]
fn json_import_preserves_integer_limits_and_scalars() {
    let table = from_json(r#"[["lo","hi","bool","null","float","str"],{"lo":-9223372036854775808,"hi":18446744073709551615,"bool":false,"null":null,"float":1E+2,"str":"é😀"}]"#).unwrap();
    assert_eq!(
        table.rows()[0],
        row![i64::MIN, u64::MAX, false, None::<i64>, 100.0, "é😀"]
    );
    let roundtrip = from_json(&table.get_json_string().unwrap()).unwrap();
    assert_eq!(roundtrip.rows(), table.rows());
    assert_eq!(roundtrip.field_names(), table.field_names());
}

#[test]
fn json_import_rejects_out_of_range_integers_before_float_conversion() {
    for number in [
        "18446744073709551616",
        "-9223372036854775809",
        "999999999999999999999999999999999999999999",
    ] {
        assert!(
            matches!(from_json(&format!("[[\"n\"],{{\"n\":{number}}}]")), Err(Error::IntOutOfRange(value)) if value == number)
        );
    }
    for number in ["1e999", "1E999", "-1e999"] {
        assert!(matches!(
            from_json(&format!("[[\"n\"],{{\"n\":{number}}}]")),
            Err(Error::Parse(_))
        ));
    }
    assert_eq!(
        from_json("[[\"n\"],{\"n\":1e2}]").unwrap().rows()[0],
        row![100.0]
    );
}

#[test]
fn json_import_rejects_malformed_structure_and_nested_values() {
    for input in [
        "",
        "{}",
        "[]",
        "[{}]",
        "[[1]]",
        "[[\"x\"],[]]",
        "[[\"x\"],{}]",
        "[[\"x\"],{\"x\":[]} ]",
        "[[\"x\"],{\"x\":{}}]",
        "[[\"x\"],{\"x\":NaN}]",
    ] {
        assert!(matches!(from_json(input), Err(Error::Parse(_))), "{input}");
    }
    assert!(matches!(
        from_json("[[\"x\",\"x\"]]"),
        Err(Error::DuplicateField(_))
    ));
    assert_eq!(from_json("[[]]").unwrap().col_count(), 0);
}

#[cfg(feature = "csv")]
#[test]
fn csv_import_trims_cells_and_handles_quotes_multiline_and_delimiters() {
    let table = from_csv(" a , b \r\n\" say \"\"hi\"\"\nnext \", é\r\n", b',').unwrap();
    assert_eq!(table.field_names(), ["a", "b"]);
    assert_eq!(table.rows()[0], row!["say \"hi\"\nnext", "é"]);
    assert_eq!(
        from_csv(&table.get_csv_string().unwrap(), b',')
            .unwrap()
            .rows(),
        table.rows()
    );
    let tab = from_csv("a\tb\r\n1\t2\r\n", b'\t').unwrap();
    assert_eq!(tab.rows()[0], row!["1", "2"]);
}

#[cfg(feature = "csv")]
#[test]
fn csv_import_rejects_ragged_rows_and_missing_headers() {
    for input in ["a,b\n1\n", "a,b\n1,2,3\n", ""] {
        assert!(matches!(from_csv(input, b','), Err(Error::Parse(_))));
    }
    assert!(matches!(
        from_csv("x,x\n1,2", b','),
        Err(Error::DuplicateField(_))
    ));
    assert!(matches!(
        from_csv("a\n1", b'\r'),
        Err(Error::InvalidOption(_))
    ));
}

#[cfg(feature = "html")]
#[test]
fn html_import_decodes_utf8_entities_and_preserves_linebreaks() {
    let table = from_html_one("<table><tr><th>A &amp; B</th><th>é</th></tr><tr><td> x<b>y</b><br>z &lt; &#x1f600; </td><td> &#39; &quot; </td></tr></table>").unwrap();
    assert_eq!(table.field_names(), ["A & B", "é"]);
    assert_eq!(table.rows()[0], row!["xy\nz < 😀", "' \""]);
    let roundtrip = from_html_one(&table.get_html_string().unwrap()).unwrap();
    assert_eq!(roundtrip.rows(), table.rows());
}

#[cfg(feature = "html")]
#[test]
fn html_import_nested_tables_own_their_rows_and_cell_text() {
    let tables = from_html("<table><tr><th>outer</th></tr><tr><td>before<table><tr><th>inner</th></tr><tr><td>inside</td></tr></table>after</td></tr></table>").unwrap();
    assert_eq!(tables.len(), 2);
    assert_eq!(tables[0].field_names(), ["outer"]);
    assert_eq!(tables[0].rows()[0], row!["beforeafter"]);
    assert_eq!(tables[1].field_names(), ["inner"]);
    assert_eq!(tables[1].rows()[0], row!["inside"]);
}

#[cfg(feature = "html")]
#[test]
fn html_import_colspan_short_rows_and_duplicate_headers_are_fully_padded() {
    let table = from_html_one("<table><tr><th>A</th><th>A</th><th>A'</th><th>A</th></tr><tr><td colspan=3>x</td><td>y</td></tr><tr><td>z</td></tr></table>").unwrap();
    assert_eq!(table.field_names(), ["A", "A'", "A''", "A'''"]);
    assert_eq!(table.rows()[0], row!["x", "", "", "y"]);
    assert_eq!(table.rows()[1], row!["z", "", "", ""]);
    let short_header =
        from_html_one("<table><tr><th>A</th></tr><tr><td>1</td><td>2</td><td>3</td></tr></table>")
            .unwrap();
    assert_eq!(short_header.field_names(), ["A", "", "'"]);
    assert_eq!(short_header.rows()[0], row!["1", "2", "3"]);
}

#[cfg(feature = "html")]
#[test]
fn html_import_recovers_markup_and_checks_single_table_count() {
    let table = from_html_one("<table><tr><td>a<td>b<tr><td>c</table>").unwrap();
    assert_eq!(table.field_names(), ["Field 1", "Field 2"]);
    assert_eq!(table.rows(), &[row!["a", "b"], row!["c", ""]]);
    assert!(from_html("plain text").unwrap().is_empty());
    assert!(matches!(from_html_one("plain text"), Err(Error::Parse(_))));
    assert!(matches!(
        from_html_one("<table></table><table></table>"),
        Err(Error::Parse(_))
    ));
    assert_eq!(from_html_one("<table></table>").unwrap().row_count(), 0);
}

#[cfg(feature = "html")]
#[test]
fn html_import_does_not_silently_truncate_colspans() {
    let table = from_html_one("<table><tr><td colspan=1001>x</td></tr></table>").unwrap();
    assert_eq!(table.col_count(), 1001);
    assert_eq!(table.rows()[0][0], Cell::from("x"));
    assert!(
        table.rows()[0][1..]
            .iter()
            .all(|cell| *cell == Cell::from(""))
    );
    let huge = format!("<table><tr><td colspan={}>x</td></tr></table>", usize::MAX);
    assert!(matches!(from_html_one(&huge), Err(Error::Parse(_))));
}

#[cfg(feature = "html")]
#[test]
fn html_import_rejects_numeric_colspan_overflow_before_allocation() {
    let above_max = (usize::MAX as u128 + 1).to_string();
    for span in [
        above_max.as_str(),
        "999999999999999999999999999999999999999999",
        "+999999999999999999999999999999999999999999",
    ] {
        let html = format!("<table><tr><td colspan=\"{span}\">x</td><td>y</td></tr></table>");
        assert!(
            matches!(from_html_one(&html), Err(Error::Parse(message)) if message == format!("HTML colspan is out of range: {span}")),
            "numeric colspan {span} must fail before the allocation path"
        );
    }
}

#[cfg(feature = "html")]
#[test]
fn html_import_recovers_malformed_colspans_as_one_column() {
    for span in ["", "invalid", "-1", "0", "999999999999999999999999999999x"] {
        let html = format!("<table><tr><td colspan=\"{span}\">x</td><td>y</td></tr></table>");
        assert_eq!(from_html_one(&html).unwrap().rows()[0], row!["x", "y"]);
    }
}

#[test]
fn mediawiki_import_trims_separators_and_roundtrips() {
    let table = from_mediawiki(
        "ignore\n{| class=\"wikitable\"\n|+ caption\n|-\n! A!!  B\n|-\n| é  ||  x\n|}\nignored",
    )
    .unwrap();
    assert_eq!(table.field_names(), ["A", "B"]);
    assert_eq!(table.rows()[0], row!["é", "x"]);
    assert_eq!(
        from_mediawiki(&table.get_mediawiki_string().unwrap())
            .unwrap()
            .rows(),
        table.rows()
    );
}

#[test]
fn mediawiki_import_rejects_missing_headers_and_ragged_rows() {
    assert!(
        matches!(from_mediawiki("{|\n| x\n|}"), Err(Error::Parse(message)) if message == "No valid header found in the MediaWiki table.")
    );
    assert!(
        matches!(from_mediawiki("{|\n! A !! B\n| x\n|}"), Err(Error::Parse(message)) if message == "Row length mismatch between header and body.")
    );
}
