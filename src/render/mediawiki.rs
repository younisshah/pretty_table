//! Write MediaWiki table markup: ! marks headers and | marks data rows.
//! For example, ! Name !! Age is the header and | Ada || 30 is a data row.

use crate::{Options, Result, Table};

impl Table {
    /// Export simple MediaWiki table markup.
    pub fn get_mediawiki_string(&self) -> Result<String> {
        render(self, &self.opts)
    }
    /// Export MediaWiki with temporary layout and selection options.
    pub fn get_mediawiki_string_with(
        &self,
        configure: impl FnOnce(&mut Options),
    ) -> Result<String> {
        let mut options = self.opts.clone();
        configure(&mut options);
        render(self, &options)
    }
}

/// Render attributes and a caption followed by header and body rows.
pub fn render(table: &Table, options: &Options) -> Result<String> {
    let resolved = table.resolved(options)?;
    // MediaWiki treats an explicitly empty field list as selecting no fields.
    let visible = if options.fields.as_ref().is_some_and(Vec::is_empty) {
        &[][..]
    } else {
        resolved.visible.as_slice()
    };
    let attributes = if options.attributes.is_empty() {
        "class=\"wikitable\"".into()
    } else {
        options
            .attributes
            .iter()
            .map(|(name, value)| format!("{name}=\"{value}\""))
            .collect::<Vec<_>>()
            .join(" ")
    };
    let mut lines = vec![format!("{{| {attributes}")];
    if let Some(title) = &options.title {
        if !title.is_empty() {
            lines.push(format!("|+ {title}"));
        }
    }
    if options.header {
        lines.push("|-".into());
        if !visible.is_empty() {
            let names: Vec<_> = visible
                .iter()
                .map(|&i| resolved.field_names[i].as_str())
                .collect();
            lines.push(format!("! {}", names.join(" !! ")));
        }
    }
    let rows: Vec<_> = table
        .selected_rows(options)?
        .into_iter()
        .map(|(row, _)| row)
        .collect();
    for row in table.format_rows(&rows)? {
        lines.push("|-".into());
        if !visible.is_empty() {
            let cells: Vec<_> = visible.iter().map(|&i| row[i].as_str()).collect();
            lines.push(format!("| {}", cells.join(" || ")));
        }
    }
    lines.push("|}".into());
    Ok(lines.join("\n"))
}
