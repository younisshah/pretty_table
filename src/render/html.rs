//! Turn table values into HTML tags; normal text is escaped by default.
//! For example, a cell containing <b>Ada</b> displays those characters as text.

use crate::{Align, HRuleStyle, Options, Result, Table, VAlign, VRuleStyle};

impl Table {
    /// Export HTML, escaping headers, data, captions, and attributes by default.
    pub fn get_html_string(&self) -> Result<String> {
        render(self, &self.opts)
    }
    /// Export HTML with temporary layout and selection options.
    pub fn get_html_string_with(&self, configure: impl FnOnce(&mut Options)) -> Result<String> {
        let mut options = self.opts.clone();
        configure(&mut options);
        render(self, &options)
    }
}

fn escape(value: &str) -> String {
    let mut output = String::new();
    for ch in value.chars() {
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#x27;"),
            ch => output.push(ch),
        }
    }
    output
}

/// Render either simple HTML or inline styles corresponding to the table options.
pub fn render(table: &Table, options: &Options) -> Result<String> {
    let resolved = table.resolved(options)?;
    let mut open = String::from("<table");
    if options.format {
        if options.border {
            open.push_str(match (options.hrules, options.vrules) {
                (HRuleStyle::All, VRuleStyle::All) => " frame=\"box\" rules=\"all\"",
                (HRuleStyle::Frame, VRuleStyle::Frame) => " frame=\"box\"",
                (HRuleStyle::Frame, VRuleStyle::All) => " frame=\"box\" rules=\"cols\"",
                (HRuleStyle::Frame, _) => " frame=\"hsides\"",
                (HRuleStyle::All, _) => " frame=\"hsides\" rules=\"rows\"",
                (_, VRuleStyle::Frame) => " frame=\"vsides\"",
                (_, VRuleStyle::All) => " frame=\"vsides\" rules=\"cols\"",
                _ => "",
            });
        } else if options.preserve_internal_border {
            open.push_str(" rules=\"cols\"");
        }
    }
    for (name, value) in &options.attributes {
        open.push_str(&format!(" {}=\"{}\"", escape(name), escape(value)));
    }
    open.push('>');
    let mut lines = vec![open];
    let linebreak = if options.xhtml { "<br/>" } else { "<br>" };
    // Escape user text first, then add our own line-break tags. Doing this
    // in reverse would display the inserted <br> tag as text.
    let content = |value: &str, escaped| {
        if escaped {
            escape(value)
        } else {
            value.to_owned()
        }
        .replace('\n', linebreak)
    };
    if let Some(title) = &options.title {
        if !title.is_empty() {
            lines.push(format!("    <caption>{}</caption>", content(title, true)));
        }
    }
    let (left, right) = options.padding();
    if options.header {
        lines.extend(["    <thead>".into(), "        <tr>".into()]);
        for &i in &resolved.visible {
            let style = if options.format {
                format!(
                    " style=\"padding-left: {left}em; padding-right: {right}em; text-align: center\""
                )
            } else {
                String::new()
            };
            lines.push(format!(
                "            <th{style}>{}</th>",
                content(&resolved.field_names[i], options.escape_header)
            ));
        }
        lines.extend(["        </tr>".into(), "    </thead>".into()]);
    }
    lines.push("    <tbody>".into());
    let rows: Vec<_> = table
        .selected_rows(options)?
        .into_iter()
        .map(|(row, _)| row)
        .collect();
    for row in table.format_rows(&rows)? {
        lines.push("        <tr>".into());
        for &i in &resolved.visible {
            let style = if options.format {
                let align = match resolved.align[i] {
                    Align::Left => "left",
                    Align::Center => "center",
                    Align::Right => "right",
                };
                let valign = match resolved.valign[i] {
                    VAlign::Top => "top",
                    VAlign::Middle => "middle",
                    VAlign::Bottom => "bottom",
                };
                format!(
                    " style=\"padding-left: {left}em; padding-right: {right}em; text-align: {align}; vertical-align: {valign}\""
                )
            } else {
                String::new()
            };
            lines.push(format!(
                "            <td{style}>{}</td>",
                content(&row[i], options.escape_data)
            ));
        }
        lines.push("        </tr>".into());
    }
    lines.extend(["    </tbody>".into(), "</table>".into()]);
    Ok(lines.join("\n"))
}
