//! Build terminal output in stages: select, format, measure, wrap, then draw.
//! One logical row can become several screen lines when a cell wraps.

use super::Resolved;
use crate::width::widest_cluster;
use crate::*;
// Width is content + padding + separators. Two width-3 columns with one
// space on each side and three border bars occupy 3 + 3 + 4 + 3 = 13 columns.
fn overhead(r: &Resolved) -> usize {
    let n = r.visible.len();
    let (l, p) = r.opts.padding();
    if r.field_names.is_empty() {
        return if r.opts.border || r.opts.preserve_internal_border {
            2
        } else {
            1
        };
    }
    let borders = if r.opts.border {
        n + 1
    } else if r.opts.preserve_internal_border {
        n
    } else {
        0
    };
    n * (l + p) + borders
}
fn table_width(r: &Resolved, w: &[usize]) -> usize {
    r.visible.iter().map(|i| w[*i]).sum::<usize>() + overhead(r)
}
// Measure the widest kind of line we will emit. A title may need more room
// than the body, even when the body has no outer border.
fn emitted_width(r: &Resolved, w: &[usize]) -> usize {
    let body = table_width(r, w);
    let Some(title) = r.opts.title.as_ref().filter(|s| !s.is_empty()) else {
        return body;
    };
    let text = title.split('\n').map(display_width).max().unwrap_or(0);
    let title_width = if r.style == Some(TableStyle::Markdown) {
        text + 4
    } else {
        let (left, right) = r.opts.padding();
        let span = r
            .visible
            .iter()
            .map(|i| w[*i] + left + right + 1)
            .sum::<usize>()
            .saturating_sub(1);
        span.max(text + left + right) + 2
    };
    body.max(title_width)
}
fn widths(r: &Resolved, rows: &[Vec<String>]) -> Result<Vec<usize>> {
    let mut w = vec![0; r.field_names.len()];
    // Each column has a smallest usable width. A cell containing 中 needs
    // at least two columns because wrapping cannot split that character.
    let mut floor = w.clone();
    for &i in &r.visible {
        if r.opts.header && r.opts.use_header_width {
            w[i] = r.field_names[i]
                .split('\n')
                .map(display_width)
                .max()
                .unwrap_or(0)
        }
        for row in rows {
            let n = row[i].split('\n').map(display_width).max().unwrap_or(0);
            w[i] = w[i].max(r.max_width[i].map_or(n, |m| n.min(m)));
            floor[i] = floor[i].max(widest_cluster(&row[i]));
        }
        let style_min = if r.style == Some(TableStyle::Markdown) {
            if r.align[i] == Align::Center { 3 } else { 1 }
        } else {
            0
        };
        floor[i] = floor[i].max(r.min_width[i]).max(style_min);
        if let Some(max) = r.max_width[i] {
            if max < floor[i] {
                return Err(Error::CannotFit {
                    needed: floor[i],
                    limit: max,
                });
            }
        }
        w[i] = w[i].max(floor[i]);
    }
    let (l, p) = r.opts.padding();
    let title_width = r
        .opts
        .title
        .as_ref()
        .filter(|s| !s.is_empty())
        .map_or(0, |t| {
            t.split('\n').map(display_width).max().unwrap_or(0)
                + l
                + p
                + if matches!(r.opts.vrules, VRuleStyle::All | VRuleStyle::Frame) {
                    2
                } else {
                    0
                }
        });
    let needed = title_width.max(r.min_table_width.unwrap_or(0));
    if let Some(limit) = r.max_table_width {
        if needed > limit {
            return Err(Error::CannotFit { needed, limit });
        }
    }
    let current = table_width(r, &w);
    // Grow columns together to fit a title or minimum table width. Put any
    // rounding remainder in the last visible column so no space is lost.
    if current < needed && !r.visible.is_empty() {
        let wanted = needed.saturating_sub(overhead(r));
        let sum = r.visible.iter().map(|i| w[*i]).sum::<usize>();
        let scale = wanted as f64 / sum.max(1) as f64;
        for &i in &r.visible {
            w[i] = (w[i] as f64 * scale) as usize
        }
        let rem = wanted.saturating_sub(r.visible.iter().map(|i| w[*i]).sum());
        w[*r.visible.last().unwrap()] += rem;
    }
    // A fieldless table has no allocatable widths. Its rule/header/data shape
    // depends on which lines are emitted, so the final actual-line check below
    // determines feasibility instead of a constant body-overhead estimate.
    if let Some(limit) = r.max_table_width.filter(|_| !r.field_names.is_empty()) {
        let min = emitted_width(r, &floor);
        if min > limit {
            return Err(Error::CannotFit { needed: min, limit });
        }
        let current = emitted_width(r, &w);
        if current > limit {
            let markup = (l + p) * r.field_names.len() + r.field_names.len().saturating_sub(1);
            let scale = (limit as i64 - markup as i64) as f64
                / (current as i64 - markup as i64).max(1) as f64;
            for &i in &r.visible {
                w[i] = floor[i].max((w[i] as f64 * scale.max(0.0)) as usize)
            }
            // Shrink only columns that still have spare room. For data 中 / abcdef,
            // widths [2, 1] can fit where [1, 2] would cut the Chinese character.
            while emitted_width(r, &w) > limit {
                let Some(i) = r
                    .visible
                    .iter()
                    .copied()
                    .filter(|i| w[*i] > floor[*i])
                    .max_by_key(|i| w[*i])
                else {
                    return Err(Error::CannotFit {
                        needed: emitted_width(r, &w),
                        limit,
                    });
                };
                w[i] -= 1;
            }
        }
    }
    Ok(w)
}
#[derive(Clone, Copy)]
enum Where {
    Top,
    Middle,
    Bottom,
}
fn hrule(
    r: &Resolved,
    w: &[usize],
    pos: Where,
    vrules: VRuleStyle,
    horizontal: Option<&str>,
) -> String {
    if !r.opts.border && !r.opts.preserve_internal_border {
        return String::new();
    }
    let h = horizontal.unwrap_or(&r.opts.horizontal_char);
    // Corners depend on position: a Unicode top rule uses ┌, ┬, ┐;
    // a separator between rows uses ├, ┼, ┤.
    let (left, right, j) = match pos {
        Where::Top => (&r.top_left, &r.top_right, &r.top_junction),
        Where::Middle => (&r.left_junction, &r.right_junction, &r.opts.junction_char),
        Where::Bottom => (&r.bottom_left, &r.bottom_right, &r.bottom_junction),
    };
    let framed = matches!(vrules, VRuleStyle::All | VRuleStyle::Frame);
    let mut bits = vec![if framed { left.clone() } else { h.into() }];
    if r.field_names.is_empty() {
        bits.push(right.clone());
        return bits.concat();
    }
    let (l, p) = r.opts.padding();
    for &i in &r.visible {
        let n = w[i] + l + p;
        let mut line = h.repeat(n);
        if let Some(a) = &r.opts.horizontal_align_char {
            if n >= 2 {
                if matches!(r.align[i], Align::Left | Align::Center) {
                    line = format!(" {a}{}", h.repeat(n - 2))
                }
                if matches!(r.align[i], Align::Right | Align::Center) {
                    let prefix = if r.align[i] == Align::Center && n >= 4 {
                        format!(" {a}{}", h.repeat(n - 4))
                    } else {
                        h.repeat(n - 2)
                    };
                    line = format!("{prefix}{a} ")
                }
            }
        }
        bits.push(line);
        bits.push(if vrules == VRuleStyle::All {
            j.clone()
        } else {
            h.into()
        });
    }
    if framed {
        bits.pop();
        bits.push(right.clone())
    }
    if r.opts.preserve_internal_border && !r.opts.border && bits.len() > 1 {
        bits.remove(0);
        bits.pop();
    }
    bits.concat()
}
fn header_name(name: &str, style: Option<HeaderStyle>) -> String {
    match style {
        None => name.into(),
        Some(HeaderStyle::Upper) => name.to_uppercase(),
        Some(HeaderStyle::Lower) => name.to_lowercase(),
        Some(HeaderStyle::Cap) => {
            let mut c = name.chars();
            c.next().map_or_else(String::new, |f| {
                f.to_uppercase().to_string() + &c.as_str().to_lowercase()
            })
        }
        Some(HeaderStyle::Title) => {
            let mut s = String::new();
            let mut start = true;
            for c in name.chars() {
                if c.is_alphanumeric() {
                    if start {
                        s.extend(c.to_uppercase())
                    } else {
                        s.extend(c.to_lowercase())
                    }
                    start = false
                } else {
                    s.push(c);
                    start = true
                }
            }
            s
        }
    }
}
fn line(r: &Resolved, w: &[usize], values: &[String]) -> String {
    let (l, p) = r.opts.padding();
    let mut bits = vec![];
    if r.opts.border {
        bits.push(
            if matches!(r.opts.vrules, VRuleStyle::All | VRuleStyle::Frame) {
                r.opts.vertical_char.clone()
            } else {
                " ".into()
            },
        )
    }
    for (&i, v) in r.visible.iter().zip(values) {
        bits.push(format!(
            "{}{}{}",
            " ".repeat(l),
            justify(v, w[i], r.align[i]),
            " ".repeat(p)
        ));
        if r.opts.border || r.opts.preserve_internal_border {
            bits.push(if r.opts.vrules == VRuleStyle::All {
                r.opts.vertical_char.clone()
            } else {
                " ".into()
            })
        }
    }
    if r.field_names.is_empty() {
        bits.push(
            if matches!(r.opts.vrules, VRuleStyle::All | VRuleStyle::Frame) {
                r.opts.vertical_char.clone()
            } else {
                " ".into()
            },
        )
    }
    if !r.opts.border && r.opts.preserve_internal_border {
        bits.pop();
        bits.push(" ".into())
    }
    if r.opts.border && r.opts.vrules == VRuleStyle::Frame {
        bits.pop();
        bits.push(r.opts.vertical_char.clone())
    }
    bits.concat()
}
fn title(r: &Resolved, w: &[usize], title: &str) -> Vec<String> {
    if r.style == Some(TableStyle::Markdown) {
        let mut lines: Vec<_> = title.split('\n').map(|s| format!("**{s}**")).collect();
        lines.push(String::new());
        return lines;
    }
    let mut lines = vec![];
    if r.opts.border && matches!(r.opts.vrules, VRuleStyle::Frame | VRuleStyle::All) {
        lines.push(hrule(r, w, Where::Top, VRuleStyle::Frame, None))
    }
    let endpoint = if r.opts.border && matches!(r.opts.vrules, VRuleStyle::All | VRuleStyle::Frame)
    {
        r.opts.vertical_char.as_str()
    } else {
        " "
    };
    let (l, p) = r.opts.padding();
    let width = r
        .visible
        .iter()
        .map(|i| w[*i] + l + p + 1)
        .sum::<usize>()
        .saturating_sub(1);
    for s in title.split('\n') {
        let s = format!("{}{s}{}", " ".repeat(l), " ".repeat(p));
        lines.push(format!(
            "{endpoint}{}{endpoint}",
            justify(&s, width, Align::Center)
        ))
    }
    lines
}
fn top(r: &Resolved, w: &[usize]) -> String {
    let mut rule = hrule(r, w, Where::Top, r.opts.vrules, None);
    if r.opts.title.as_ref().is_some_and(|s| !s.is_empty())
        && matches!(r.opts.vrules, VRuleStyle::All | VRuleStyle::Frame)
        && rule.starts_with(&r.top_left)
        && rule.ends_with(&r.top_right)
    {
        rule = format!(
            "{}{}{}",
            r.left_junction,
            &rule[r.top_left.len()..rule.len() - r.top_right.len()],
            r.right_junction
        )
    }
    rule
}
/// Replace an edge glyph with a pipe while keeping its complete color commands.
/// Cutting the first byte of a colored edge would cut the ESC command itself.
fn orgmode_edge(mut s: &str, bars: &str) -> String {
    let mut out = String::new();
    let mut inserted = false;
    while !s.is_empty() {
        let escape = crate::width::escape_len(s);
        if escape > 0 {
            out.push_str(&s[..escape]);
            s = &s[escape..];
        } else {
            if !inserted {
                out.push_str(bars);
                inserted = true;
            }
            s = &s[s.chars().next().unwrap().len_utf8()..];
        }
    }
    if !inserted {
        out.push_str(bars);
    }
    out
}
fn orgmode_line(s: &str) -> String {
    let mut parts = crate::width::clusters(s);
    if parts.len() < 2 {
        return orgmode_edge(s, "||");
    }
    let last = parts.len() - 1;
    parts[0] = orgmode_edge(&parts[0], "|");
    parts[last] = orgmode_edge(&parts[last], "|");
    parts.concat()
}
/// Render one table with a single resolved configuration.
pub fn render(t: &Table, o: &Options) -> Result<String> {
    let r = t.resolved(o)?;
    if t.row_count() == 0 && (!o.print_empty || !o.border) {
        return Ok(if r.theme.is_some() {
            crate::color::RESET.into()
        } else {
            String::new()
        });
    }
    // Keep selection and display formatting separate: sorting sees numbers,
    // while layout sees their final text, such as "2.50".
    let selected = t.selected_rows(o)?;
    let rows: Vec<_> = selected.iter().map(|p| p.0.clone()).collect();
    let rows = t.format_rows(&rows)?;
    let w = widths(&r, &rows)?;
    let mut lines = vec![];
    if let Some(s) = r.opts.title.as_ref().filter(|s| !s.is_empty()) {
        lines.extend(title(&r, &w, s));
    }
    if r.opts.header {
        if r.opts.border && matches!(r.opts.hrules, HRuleStyle::All | HRuleStyle::Frame) {
            lines.push(top(&r, &w))
        }
        let values: Vec<_> = r
            .visible
            .iter()
            .map(|i| {
                crate::wrap::propagate_sgr(vec![truncate(
                    &header_name(&r.field_names[*i], r.header_style),
                    w[*i],
                )])
                .remove(0)
            })
            .collect();
        lines.push(line(&r, &w, &values));
        if (r.opts.border || r.opts.preserve_internal_border) && r.opts.hrules != HRuleStyle::None {
            lines.push(hrule(
                &r,
                &w,
                Where::Middle,
                r.opts.vrules,
                r.opts.header_horizontal_char.as_deref(),
            ))
        }
    } else if r.opts.border && matches!(r.opts.hrules, HRuleStyle::All | HRuleStyle::Frame) {
        lines.push(top(&r, &w))
    }
    for (j, row) in rows.iter().enumerate() {
        let mut cells = vec![];
        for &i in &r.visible {
            let mut ls = vec![];
            for s in crate::wrap::propagate_sgr(row[i].split('\n').map(str::to_owned).collect()) {
                if display_width(&s) > w[i] {
                    ls.extend(crate::wrap::wrap(&s, w[i], r.opts.break_on_hyphens)?)
                } else {
                    ls.push(s)
                }
            }
            cells.push(ls)
        }
        // ["Review", "first\nsecond"] needs two screen lines. Shorter cells
        // receive blank lines above/below according to their vertical alignment.
        let height = cells.iter().map(Vec::len).max().unwrap_or(1);
        for y in 0..height {
            let vals: Vec<_> = cells
                .iter()
                .zip(&r.visible)
                .map(|(ls, i)| {
                    let delta = height - ls.len();
                    let start = match r.valign[*i] {
                        VAlign::Top => 0,
                        VAlign::Middle => delta / 2,
                        VAlign::Bottom => delta,
                    };
                    if y < start || y >= start + ls.len() {
                        String::new()
                    } else {
                        ls[y - start].clone()
                    }
                })
                .collect();
            lines.push(line(&r, &w, &vals));
        }
        if r.opts.border && r.opts.hrules == HRuleStyle::All {
            lines.push(hrule(
                &r,
                &w,
                if j + 1 == rows.len() {
                    Where::Bottom
                } else {
                    Where::Middle
                },
                r.opts.vrules,
                None,
            ))
        }
        if j + 1 < rows.len() && selected[j].1 {
            lines.push(hrule(&r, &w, Where::Middle, r.opts.vrules, None))
        }
    }
    if r.opts.border && r.opts.hrules == HRuleStyle::Frame {
        lines.push(hrule(&r, &w, Where::Bottom, r.opts.vrules, None))
    }
    if r.orgmode {
        lines = lines
            .into_iter()
            .flat_map(|s| s.split('\n').map(orgmode_line).collect::<Vec<_>>())
            .collect()
    }
    // Check the actual emitted lines too, including titles and unusual border
    // combinations. A correct width estimate alone is not proof that output fits.
    if let Some(limit) = r.max_table_width {
        let needed = lines
            .iter()
            .flat_map(|s| s.split('\n'))
            .map(display_width)
            .max()
            .unwrap_or(0);
        if needed > limit {
            return Err(Error::CannotFit { needed, limit });
        }
    }
    let mut out = lines.join("\n");
    if r.theme.is_some() {
        out.push_str(crate::color::RESET)
    }
    Ok(out)
}
