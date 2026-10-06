//! Wrap text without separating a character from its accent or emoji partners.
//! Terminal color commands travel with the text but take no display columns.

use crate::width::{clusters, escape_len};
use crate::{Error, Result, display_width, expand_tabs};
fn whitespace(g: &str) -> bool {
    crate::strip_ansi(g)
        .chars()
        .all(|c| matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0b' | '\x0c'))
        && display_width(g) > 0
}
fn word(g: &str) -> bool {
    g.chars()
        .next()
        .is_some_and(|c| c.is_alphanumeric() || c == '_')
}
fn letter(g: &str) -> bool {
    g.chars()
        .next()
        .is_some_and(|c| c.is_alphabetic() || c == '_')
}
// Find word/space boundaries before filling lines. Python also permits
// selected hyphen breaks, for example inside a long hyphenated word.
fn chunks(text: &str, hyphens: bool) -> Vec<String> {
    let g = clusters(text);
    let plain: Vec<_> = g.iter().map(|s| crate::strip_ansi(s)).collect();
    let mut cuts = vec![false; g.len() + 1];
    if hyphens {
        let mut i = 0;
        while i < g.len() {
            if plain[i] != "-" {
                i += 1;
                continue;
            }
            let mut end = i + 1;
            while end < g.len() && plain[end] == "-" {
                end += 1
            }
            if end - i >= 2 {
                let before = i > 0
                    && (word(&plain[i - 1])
                        || plain[i - 1].chars().next().is_some_and(|c| {
                            matches!(c, '!' | '"' | '\'' | '&' | '.' | ',' | '?')
                        }));
                if before && end < g.len() && word(&plain[end]) {
                    cuts[i] = true;
                    cuts[end] = true
                }
            } else {
                let left = (i >= 2 && letter(&plain[i - 1]) && letter(&plain[i - 2]))
                    || (i >= 3
                        && letter(&plain[i - 1])
                        && plain[i - 2] == "-"
                        && letter(&plain[i - 3]));
                let right = i + 2 < g.len()
                    && letter(&plain[i + 1])
                    && (letter(&plain[i + 2])
                        || (plain[i + 2] == "-" && i + 3 < g.len() && letter(&plain[i + 3])));
                if left && right {
                    cuts[i + 1] = true
                }
            }
            i = end;
        }
    }
    let mut out = vec![];
    let mut current = String::new();
    for i in 0..g.len() {
        if !current.is_empty() && (cuts[i] || (i > 0 && whitespace(&g[i]) != whitespace(&g[i - 1])))
        {
            out.push(std::mem::take(&mut current));
        }
        current.push_str(&g[i]);
    }
    if !current.is_empty() {
        out.push(current)
    }
    out
}
// SGR commands control colors and effects such as bold. Remember which
// effects are active so wrapping can close and reopen them around a line break.
#[derive(Default)]
struct SgrState {
    active: Vec<(String, String)>,
}
impl SgrState {
    fn opening(&self) -> String {
        if self.active.is_empty() {
            String::new()
        } else {
            format!(
                "\x1b[{}m",
                self.active
                    .iter()
                    .map(|(_, part)| part.as_str())
                    .collect::<Vec<_>>()
                    .join(";")
            )
        }
    }
    fn consume(&mut self, line: &str) {
        let mut pos = 0;
        while pos < line.len() {
            let rest = &line[pos..];
            let n = escape_len(rest);
            if n == 0 {
                pos += rest.chars().next().unwrap().len_utf8();
                continue;
            }
            let seq = &rest[..n];
            pos += n;
            if !seq.starts_with("\x1b[") || !seq.ends_with('m') {
                continue;
            }
            let body = &seq[2..seq.len() - 1];
            let parts: Vec<_> = body.split(';').collect();
            let mut i = 0;
            while i < parts.len() {
                let part = parts[i];
                let code = if part.is_empty() {
                    Some(0)
                } else {
                    part.split(':').next().unwrap().parse::<u32>().ok()
                };
                if code == Some(0) {
                    self.active.clear();
                    i += 1;
                    continue;
                }
                // RGB parameters belong together: the zeroes in 38;2;255;0;0
                // mean "no green/blue", not "reset every active text effect".
                let end = if matches!(code, Some(38 | 48)) && !part.contains(':') {
                    match parts.get(i + 1).and_then(|p| p.parse::<u32>().ok()) {
                        Some(5) => (i + 3).min(parts.len()),
                        Some(2) => (i + 5).min(parts.len()),
                        _ => i + 1,
                    }
                } else {
                    i + 1
                };
                let (group, reset) = match code {
                    Some(1) => ("1".into(), false),
                    Some(2) => ("2".into(), false),
                    Some(3) => ("23".into(), false),
                    Some(4 | 21) => ("24".into(), false),
                    Some(5 | 6) => ("25".into(), false),
                    Some(7) => ("27".into(), false),
                    Some(8) => ("28".into(), false),
                    Some(9) => ("29".into(), false),
                    Some(30..=38 | 90..=97) => ("39".into(), false),
                    Some(40..=48 | 100..=107) => ("49".into(), false),
                    Some(22 | 23 | 24 | 25 | 27 | 28 | 29 | 39 | 49) => {
                        (code.unwrap().to_string(), true)
                    }
                    Some(c) => (c.to_string(), false),
                    None => (format!("unknown:{part}"), false),
                };
                // Bold and dim can coexist; code 22 switches both off.
                if code == Some(22) {
                    self.active.retain(|(g, _)| g != "1" && g != "2")
                } else {
                    self.active.retain(|(g, _)| g != &group)
                }
                if !reset {
                    self.active.push((group, parts[i..end].join(";")))
                }
                i = end;
            }
        }
    }
}
/// Close text colors before table padding/borders, then reopen them on the next line.
/// For red text split into "hel" and "lo", each line gets its own red start and reset.
pub fn propagate_sgr(lines: Vec<String>) -> Vec<String> {
    let mut state = SgrState::default();
    let mut out = vec![];
    for line in lines {
        let mut rendered = state.opening();
        rendered.push_str(&line);
        state.consume(&line);
        if !state.active.is_empty() {
            rendered.push_str("\x1b[0m")
        }
        out.push(rendered)
    }
    out
}
fn visibly_blank(s: &str) -> bool {
    crate::strip_ansi(s).trim().is_empty()
}
fn escapes_only(s: &str) -> String {
    let mut out = String::new();
    let mut pos = 0;
    while pos < s.len() {
        let rest = &s[pos..];
        let n = escape_len(rest);
        if n > 0 {
            out.push_str(&rest[..n]);
            pos += n
        } else {
            pos += rest.chars().next().unwrap().len_utf8()
        }
    }
    out
}
fn trim_visible_end(line: &str) -> (String, String) {
    let g = clusters(line);
    let end = g
        .iter()
        .rposition(|s| !visibly_blank(s))
        .map_or(0, |i| i + 1);
    (g[..end].concat(), escapes_only(&g[end..].concat()))
}
/// Put as much text as fits on each line, keeping visible characters whole.
///
/// ```
/// use pretty_table_rs::wrap::wrap;
/// assert_eq!(wrap("red green", 5, true).unwrap(), vec!["red", "green"]);
/// assert_eq!(wrap("中文", 2, true).unwrap(), vec!["中", "文"]);
/// ```
pub fn wrap(text: &str, width: usize, break_on_hyphens: bool) -> Result<Vec<String>> {
    let expanded = expand_tabs(text);
    let text: String = expanded
        .chars()
        .map(|c| {
            if matches!(c, '\t' | '\n' | '\r' | '\x0b' | '\x0c') {
                ' '
            } else {
                c
            }
        })
        .collect();
    let mut queue: std::collections::VecDeque<String> = chunks(&text, break_on_hyphens).into();
    let mut lines = vec![];
    // Spaces can be dropped at a wrap boundary, but a color command on those
    // spaces must still reach the next visible word.
    let mut pending = String::new();
    while !queue.is_empty() {
        if !lines.is_empty() {
            while queue.front().is_some_and(|s| visibly_blank(s)) {
                pending.push_str(&escapes_only(&queue.pop_front().unwrap()));
            }
        }
        let mut line = std::mem::take(&mut pending);
        let mut used = 0;
        while let Some(next) = queue.front() {
            let n = display_width(next);
            if used + n <= width {
                used += n;
                line.push_str(&queue.pop_front().unwrap());
            } else {
                break;
            }
        }
        if let Some(next) = queue.front() {
            if display_width(next) > width {
                let available = width.saturating_sub(used);
                let gs = clusters(next);
                let mut n = 0;
                let mut count = 0;
                for g in &gs {
                    let w = display_width(g);
                    if n + w > available {
                        break;
                    }
                    n += w;
                    count += 1
                }
                if count == 0 && used == 0 && visibly_blank(&line) {
                    let needed = gs.first().map_or(1, |g| display_width(g));
                    return Err(Error::CannotFit {
                        needed,
                        limit: width,
                    });
                }
                if count > 0 {
                    if break_on_hyphens {
                        if let Some(i) = gs[..count].iter().rposition(|g| g == "-") {
                            if i > 0 {
                                count = i + 1
                            }
                        }
                    }
                    let next = queue.pop_front().unwrap();
                    let gs = clusters(&next);
                    line.push_str(&gs[..count].concat());
                    if count < gs.len() {
                        queue.push_front(gs[count..].concat())
                    }
                }
            }
        }
        let (trimmed, escapes) = trim_visible_end(&line);
        pending.push_str(&escapes);
        if !visibly_blank(&trimmed) {
            lines.push(trimmed)
        }
    }
    if lines.is_empty() {
        lines.push(String::new())
    }
    Ok(propagate_sgr(lines))
}
