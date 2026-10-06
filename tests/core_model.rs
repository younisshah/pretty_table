use pretty_table_rs::{Align, Cell, ColumnValue, Error, Table, cmp_cells, row};
use std::cmp::Ordering;
#[test]
fn numeric_exact() {
    assert_eq!(
        cmp_cells(
            &Cell::Int(9007199254740993),
            &Cell::Float(9007199254740992.0)
        )
        .unwrap(),
        Ordering::Greater
    );
    assert_eq!(
        cmp_cells(&Cell::UInt(u64::MAX), &Cell::Int(-1)).unwrap(),
        Ordering::Greater
    );
    assert_eq!(
        cmp_cells(&Cell::UInt(1 << 63), &Cell::Float((1u64 << 63) as f64)).unwrap(),
        Ordering::Equal
    );
    assert!(cmp_cells(&Cell::Float(f64::NAN), &Cell::Int(1)).is_err());
    assert_eq!(
        cmp_cells(&Cell::Bool(true), &Cell::Int(1)).unwrap(),
        Ordering::Equal
    );
    assert_ne!(Cell::Int(1), Cell::UInt(1));
}
#[test]
fn mutation_transaction() {
    let mut t = Table::with_fields(["A", "B"]).unwrap();
    t.add_row(row![1, "x"]).unwrap();
    assert!(t.add_row(row![1]).is_err());
    assert_eq!(t.row_count(), 1);
    assert!(matches!(
        t.set_field_names(["X", "X"]),
        Err(Error::DuplicateField(_))
    ));
    assert_eq!(t.field_names(), ["A", "B"]);
    t.set_align(ColumnValue::Scalar(Align::Left)).unwrap();
    t.set_field_names(["B", "A"]).unwrap();
    let mut s = t.slice(..);
    s.opts.header = false;
    assert!(t.opts.header);
}
#[test]
fn sorting_fallible_and_stable() {
    let mut t = Table::with_fields(["A"]).unwrap();
    t.add_row(row![3]).unwrap();
    t.add_row(row!["a"]).unwrap();
    t.add_row(row![1]).unwrap();
    t.opts.sortby = Some("A".into());
    assert!(t.selected_rows(&t.opts).is_err());
    assert_eq!(t.rows()[0], row![3]);
}
#[test]
fn divider_selection() {
    let mut t = Table::with_fields(["A"]).unwrap();
    t.add_row(row![0]).unwrap();
    t.add_row_divider(row![1], true).unwrap();
    t.add_row(row![2]).unwrap();
    t.opts.row_filter = Some(std::rc::Rc::new(|r| r[0] != Cell::Int(0)));
    assert!(t.selected_rows(&t.opts).unwrap()[0].1);
}
