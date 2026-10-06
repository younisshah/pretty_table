//! Share one resolved set of settings across output helpers.
//! For example, an overridden border character must reach both headers and data rows.

use crate::*;
#[cfg(feature = "csv")]
pub mod csv;
pub mod html;
pub mod json;
pub mod latex;
pub mod mediawiki;
pub mod text;

impl Table {
    /// Export using the selected format and the table's current options.
    pub fn get_formatted_string(&self, format: Format) -> Result<String> {
        match format {
            Format::Text => self.get_string(),
            #[cfg(feature = "csv")]
            Format::Csv => self.get_csv_string(),
            #[cfg(not(feature = "csv"))]
            Format::Csv => Err(Error::InvalidOption("csv feature is disabled".into())),
            Format::Json => self.get_json_string(),
            Format::Html => self.get_html_string(),
            Format::Latex => self.get_latex_string(),
            Format::Mediawiki => self.get_mediawiki_string(),
        }
    }
}
/// A validated configuration and positional state shared by rendering helpers.
#[derive(Clone)]
pub struct Resolved {
    pub opts: Options,
    pub field_names: Vec<String>,
    pub visible: Vec<usize>,
    pub align: Vec<Align>,
    pub valign: Vec<VAlign>,
    pub min_width: Vec<usize>,
    pub max_width: Vec<Option<usize>>,
    pub style: Option<TableStyle>,
    pub orgmode: bool,
    pub header_style: Option<HeaderStyle>,
    pub min_table_width: Option<usize>,
    pub max_table_width: Option<usize>,
    pub theme: Option<Theme>,
    pub top_junction: String,
    pub bottom_junction: String,
    pub left_junction: String,
    pub right_junction: String,
    pub top_left: String,
    pub top_right: String,
    pub bottom_left: String,
    pub bottom_right: String,
}
/// Merge table state and per-call options exactly once.
pub fn resolve(t: &Table, o: &Options) -> Result<Resolved> {
    o.validate(&t.field_names)?;
    let mut o = o.clone();
    // The caller has already chosen the glyphs. The theme supplies their colors,
    // so a one-call override such as vertical_char = "!" remains visible.
    if let Some(theme) = &t.theme {
        o.vertical_char = theme.vertical(&o.vertical_char);
        o.horizontal_char = theme.horizontal(&o.horizontal_char);
        o.junction_char = theme.junction(&o.junction_char);
    }
    let fallback = |s: &Option<String>| s.clone().unwrap_or_else(|| o.junction_char.clone());
    // Keep indices into the original rows: selecting Age from [Name, Age]
    // gives index [1], without deleting Name from the stored data.
    let visible = t
        .field_names
        .iter()
        .enumerate()
        .filter(|(_, n)| {
            o.fields
                .as_ref()
                .is_none_or(|f| f.is_empty() || f.contains(n))
        })
        .map(|(i, _)| i)
        .collect();
    Ok(Resolved {
        top_junction: fallback(&o.top_junction_char),
        bottom_junction: fallback(&o.bottom_junction_char),
        left_junction: fallback(&o.left_junction_char),
        right_junction: fallback(&o.right_junction_char),
        top_left: fallback(&o.top_left_junction_char),
        top_right: fallback(&o.top_right_junction_char),
        bottom_left: fallback(&o.bottom_left_junction_char),
        bottom_right: fallback(&o.bottom_right_junction_char),
        field_names: t.field_names.clone(),
        visible,
        align: t
            .field_names
            .iter()
            .map(|n| t.align.get(n).copied().unwrap_or(t.base_align))
            .collect(),
        valign: t
            .field_names
            .iter()
            .map(|n| t.valign.get(n).copied().unwrap_or(VAlign::Top))
            .collect(),
        min_width: t
            .field_names
            .iter()
            .map(|n| t.min_width.get(n).copied().unwrap_or(0))
            .collect(),
        max_width: t
            .field_names
            .iter()
            .map(|n| t.max_width.get(n).copied())
            .collect(),
        style: t.style,
        orgmode: t.orgmode,
        header_style: t.header_style,
        min_table_width: t.min_table_width,
        max_table_width: t.max_table_width,
        theme: t.theme.clone(),
        opts: o,
    })
}
