//! A theme chooses border glyph defaults and terminal colors.
//! Later style or glyph changes can keep the colors while choosing a different border.

/// ANSI reset used by color themes.
pub const RESET: &str = "\x1b[0m";
/// Colors and rule glyphs for a terminal table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    pub default_color: String,
    pub vertical_char: String,
    pub vertical_color: String,
    pub horizontal_char: String,
    pub horizontal_color: String,
    pub junction_char: String,
    pub junction_color: String,
}
impl Default for Theme {
    fn default() -> Self {
        Self::new("", "|", "", "-", "", "+", "")
    }
}
impl Theme {
    /// Accept a short color number or an already complete terminal color command.
    ///
    /// ```
    /// use pretty_table::Theme;
    /// assert_eq!(Theme::format_code("31"), "\x1b[31m");
    /// assert_eq!(Theme::format_code(""), "");
    /// ```
    pub fn format_code(s: &str) -> String {
        if s.trim().is_empty() {
            String::new()
        } else if s.starts_with("\x1b[") {
            s.into()
        } else {
            format!("\x1b[{s}m")
        }
    }
    /// Construct a theme from raw SGR codes and rule glyphs.
    pub fn new(
        default: &str,
        vertical: &str,
        vc: &str,
        horizontal: &str,
        hc: &str,
        junction: &str,
        jc: &str,
    ) -> Self {
        Self {
            default_color: Self::format_code(default),
            vertical_char: vertical.into(),
            vertical_color: Self::format_code(vc),
            horizontal_char: horizontal.into(),
            horizontal_color: Self::format_code(hc),
            junction_char: junction.into(),
            junction_color: Self::format_code(jc),
        }
    }
    // Color one border glyph, reset that color, then restore the theme's
    // default text color so the following cell does not inherit the border color.
    pub(crate) fn vertical(&self, glyph: &str) -> String {
        format!(
            "{}{}{RESET}{}",
            self.vertical_color, glyph, self.default_color
        )
    }
    pub(crate) fn horizontal(&self, glyph: &str) -> String {
        format!(
            "{}{}{RESET}{}",
            self.horizontal_color, glyph, self.default_color
        )
    }
    pub(crate) fn junction(&self, glyph: &str) -> String {
        format!(
            "{}{}{RESET}{}",
            self.junction_color, glyph, self.default_color
        )
    }
}
/// Named themes matching Python ColorTable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Themes {
    Default,
    DyslexiaFriendly,
    Earth,
    GlareReduction,
    HighContrast,
    Lavender,
    Ocean,
    OceanDeep,
    Pastel,
}
impl From<Themes> for Theme {
    fn from(t: Themes) -> Self {
        let (d, v, h, j) = match t {
            Themes::Default => ("", "", "", ""),
            Themes::DyslexiaFriendly => ("38;5;223", "38;5;22", "38;5;22", "38;5;58"),
            Themes::Earth => ("33", "38;5;94", "38;5;22", "38;5;130"),
            Themes::GlareReduction => ("38;5;252", "38;5;240", "38;5;240", "38;5;246"),
            Themes::HighContrast => ("97", "91", "94", "93"),
            Themes::Lavender => ("38;5;183", "35", "38;5;147", "38;5;219"),
            Themes::Ocean => ("96", "34", "34", "36"),
            Themes::OceanDeep => ("96", "34", "36", "94"),
            Themes::Pastel => ("38;5;223", "38;5;152", "38;5;187", "38;5;157"),
        };
        Self::new(d, "|", v, "-", h, "+", j)
    }
}
