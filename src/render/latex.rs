//! Build a LaTeX tabular block from formatted cells.
//! Cell content is emitted verbatim; callers escape LaTeX characters such as & when needed.

use crate::{Align, HRuleStyle, Options, Result, Table, VRuleStyle};

impl Table {
    /// Export raw cell content as a LaTeX tabular environment using CRLF lines.
    pub fn get_latex_string(&self) -> Result<String> {
        render(self, &self.opts)
    }
    /// Export LaTeX with temporary layout and selection options.
    pub fn get_latex_string_with(&self, configure: impl FnOnce(&mut Options)) -> Result<String> {
        let mut options = self.opts.clone();
        configure(&mut options);
        render(self, &options)
    }
}

/// Render a simple tabular environment or add the configured vertical and horizontal rules.
pub fn render(table: &Table, options: &Options) -> Result<String> {
    let resolved = table.resolved(options)?;
    let alignments: Vec<_> = resolved
        .visible
        .iter()
        .map(|&i| match resolved.align[i] {
            Align::Left => "l",
            Align::Center => "c",
            Align::Right => "r",
        })
        .collect();
    let internal = options.format
        && ((options.border && options.vrules == VRuleStyle::All)
            || (!options.border && options.preserve_internal_border));
    // LaTeX uses l/c/r for alignment. With internal rules, two columns
    // might have the declaration l|r; an outer frame turns it into |l|r|.
    let mut alignment = alignments.join(if internal { "|" } else { "" });
    if options.format
        && options.border
        && matches!(options.vrules, VRuleStyle::All | VRuleStyle::Frame)
    {
        alignment = format!("|{alignment}|");
    }
    let mut lines = vec![format!("\\begin{{tabular}}{{{alignment}}}")];
    if options.format
        && options.border
        && matches!(options.hrules, HRuleStyle::All | HRuleStyle::Frame)
    {
        lines.push("\\hline".into());
    }
    if options.header {
        let headers: Vec<_> = resolved
            .visible
            .iter()
            .map(|&i| resolved.field_names[i].as_str())
            .collect();
        lines.push(format!("{} \\\\", headers.join(" & ")));
    }
    if options.format
        && (options.border || options.preserve_internal_border)
        && matches!(options.hrules, HRuleStyle::All | HRuleStyle::Header)
    {
        lines.push("\\hline".into());
    }
    let rows: Vec<_> = table
        .selected_rows(options)?
        .into_iter()
        .map(|(row, _)| row)
        .collect();
    for row in table.format_rows(&rows)? {
        let cells: Vec<_> = resolved.visible.iter().map(|&i| row[i].as_str()).collect();
        lines.push(format!("{} \\\\", cells.join(" & ")));
        if options.format && options.border && options.hrules == HRuleStyle::All {
            lines.push("\\hline".into());
        }
    }
    if options.format && options.border && options.hrules == HRuleStyle::Frame {
        lines.push("\\hline".into());
    }
    lines.push("\\end{tabular}".into());
    Ok(lines.join("\r\n"))
}
