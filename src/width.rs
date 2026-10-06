//! Measure the space text occupies on screen, not its byte length.
//! A Chinese character can occupy two columns; an ANSI color code occupies none.

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;
/// Length of a terminal escape at the beginning of a string, in bytes.
pub(crate) fn escape_len(s: &str) -> usize {
    let b = s.as_bytes();
    if b.first() != Some(&27) {
        return 0;
    }
    if b.len() < 2 {
        return 1;
    }
    // ESC [ ... m can set a color; ESC ] ... can carry a terminal hyperlink.
    // Skip each complete command so its bytes never count as visible text.
    match b[1] {
        b'[' => b
            .iter()
            .enumerate()
            .skip(2)
            .find(|(_, c)| (0x40..=0x7e).contains(*c))
            .map_or(b.len(), |(i, _)| i + 1),
        b']' | b'P' | b'_' | b'^' | b'X' => {
            let mut i = 2;
            while i < b.len() {
                if b[1] == b']' && b[i] == 7 {
                    return i + 1;
                }
                if b[i] == 27 && b.get(i + 1) == Some(&b'\\') {
                    return i + 2;
                }
                i += 1
            }
            b.len()
        }
        0x20..=0x2f => b
            .iter()
            .enumerate()
            .skip(2)
            .find(|(_, c)| (0x30..=0x7e).contains(*c))
            .map_or(b.len(), |(i, _)| i + 1),
        0x30..=0x7e => 2,
        // With an unsupported escape such as ESC followed by 中, skip only ESC.
        // Skipping two bytes would stop in the middle of that Unicode character.
        _ => 1,
    }
}
/// Drop terminal escapes and control characters before measuring Unicode display width.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut pos = 0;
    while pos < s.len() {
        let rest = &s[pos..];
        let esc = escape_len(rest);
        if esc > 0 {
            pos += esc;
            continue;
        }
        let c = rest.chars().next().unwrap();
        pos += c.len_utf8();
        if !c.is_control() {
            out.push(c)
        }
    }
    out
}
/// Count visible columns on one line, ignoring terminal codes and control characters.
///
/// ```
/// use pretty_table::display_width;
/// assert_eq!(display_width("中"), 2);
/// assert_eq!(display_width("e\u{301}"), 1);
/// assert_eq!(display_width("\x1b[31mred\x1b[0m"), 3);
/// ```
pub fn display_width(s: &str) -> usize {
    UnicodeWidthStr::width(strip_ansi(s).as_str())
}
/// Pad by terminal display width, using CPython's center parity rule.
pub fn justify(s: &str, width: usize, align: crate::Align) -> String {
    let pad = width.saturating_sub(display_width(s));
    // If centering leaves an odd extra space, use Python's choice of side.
    // For example, centering "ab" in width 3 produces " ab".
    let left = match align {
        crate::Align::Left => 0,
        crate::Align::Right => pad,
        crate::Align::Center => pad / 2 + (pad & width & 1),
    };
    format!("{}{s}{}", " ".repeat(left), " ".repeat(pad - left))
}
pub(crate) fn clusters(s: &str) -> Vec<String> {
    // Find graphemes in visible text, then map boundaries back to the original
    // bytes so even escape sequences inside combining/ZWJ clusters are kept.
    let mut plain = String::new();
    let mut positions = Vec::new();
    let (mut pos, mut attached_start) = (0, 0);
    while pos < s.len() {
        let rest = &s[pos..];
        let n = escape_len(rest);
        if n > 0 {
            pos += n;
            continue;
        }
        let c = rest.chars().next().unwrap();
        positions.push((plain.len(), attached_start));
        plain.push(c);
        pos += c.len_utf8();
        attached_start = pos;
    }
    // A grapheme is one visible unit, such as e + an accent or a joined emoji.
    // Find those units in plain text, then recover their original colored bytes.
    let starts: Vec<usize> = plain
        .grapheme_indices(true)
        .map(|(i, _)| positions[positions.binary_search_by_key(&i, |p| p.0).unwrap()].1)
        .collect();
    if starts.is_empty() {
        return if s.is_empty() { vec![] } else { vec![s.into()] };
    }
    starts
        .iter()
        .enumerate()
        .map(|(i, start)| s[*start..starts.get(i + 1).copied().unwrap_or(s.len())].to_string())
        .collect()
}
/// Keep the longest prefix that fits, without cutting a visible character apart.
///
/// ```
/// use pretty_table::truncate;
/// assert_eq!(truncate("中文", 2), "中");
/// assert_eq!(truncate("abcdef", 3), "abc");
/// ```
pub fn truncate(s: &str, width: usize) -> String {
    let mut used = 0;
    let mut out = String::new();
    for g in clusters(s) {
        let w = display_width(&g);
        if used + w > width {
            break;
        }
        used += w;
        out.push_str(&g)
    }
    out
}
// A cell containing 中 needs at least two columns, even when wrapping is enabled.
pub(crate) fn widest_cluster(s: &str) -> usize {
    clusters(s)
        .iter()
        .map(|g| display_width(g))
        .max()
        .unwrap_or(0)
}
/// Expand tabs exactly as Python `str.expandtabs()`: count Unicode scalars,
/// including escape-sequence characters, and reset the position on CR/LF.
pub fn expand_tabs(s: &str) -> String {
    let mut out = String::new();
    let mut col = 0;
    for c in s.chars() {
        if c == '\t' {
            // Move to the next group of eight character positions.
            // "ab\tx" becomes "ab      x". This follows Python's character count,
            // which deliberately differs from measuring terminal display columns.
            let n = 8 - col % 8;
            out.push_str(&" ".repeat(n));
            col += n
        } else {
            out.push(c);
            if matches!(c, '\r' | '\n') {
                col = 0
            } else {
                col += 1
            }
        }
    }
    out
}
