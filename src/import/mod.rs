//! Input helpers build an ordinary Table; normal rendering works the same afterwards.
//! CSV and HTML readers are optional features; JSON and MediaWiki readers are always available.

#[cfg(feature = "csv")]
mod csv;
#[cfg(feature = "html")]
mod html;
mod json;
mod mediawiki;
#[cfg(feature = "csv")]
pub use csv::from_csv;
#[cfg(feature = "html")]
pub use html::{from_html, from_html_one};
pub use json::from_json;
pub use mediawiki::from_mediawiki;
