//! Run with cargo run --bin demo to see every built-in style and theme.

use pretty_table::{Result, Table, TableStyle, Theme, Themes, row};

fn main() -> Result<()> {
    let mut cities = Table::with_fields(["City name", "Area", "Population", "Annual Rainfall"])?;
    cities.add_rows(
        [
            row!["Adelaide", 1295, 1158259, 600.5],
            row!["Brisbane", 5905, 1857594, 1146.4],
            row!["Darwin", 112, 120900, 1714.7],
            row!["Hobart", 1357, 205556, 619.5],
            row!["Sydney", 2058, 4336374, 1214.8],
            row!["Melbourne", 1566, 3806092, 646.9],
            row!["Perth", 5386, 1554769, 869.4],
        ],
        false,
    )?;
    // Reuse the data, but clone its settings for each example. One style
    // should not accidentally carry its border settings into the next demo.
    for style in [
        TableStyle::Default,
        TableStyle::MswordFriendly,
        TableStyle::PlainColumns,
        TableStyle::Markdown,
        TableStyle::Orgmode,
        TableStyle::DoubleBorder,
        TableStyle::SingleBorder,
        TableStyle::Rst,
    ] {
        let mut table = cities.clone();
        table.set_style(style)?;
        println!("{style:?}\n{}\n", table.get_string()?);
    }
    for theme in [
        Themes::Default,
        Themes::DyslexiaFriendly,
        Themes::Earth,
        Themes::GlareReduction,
        Themes::HighContrast,
        Themes::Lavender,
        Themes::Ocean,
        Themes::OceanDeep,
        Themes::Pastel,
    ] {
        let mut table = cities.clone();
        table.set_theme(Some(Theme::from(theme)))?;
        println!("{theme:?}\n{}\n", table.get_string()?);
    }
    Ok(())
}
