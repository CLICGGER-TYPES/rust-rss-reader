use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),
    #[error("feed parse error: {0}")]
    FeedParse(String),
    #[error("fetch failed: {0}")]
    Fetch(String),
    #[error("OPML error: {0}")]
    Opml(String),
    #[error("readability error: {0}")]
    Readability(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid input: {0}")]
    Invalid(String),
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// 方便 Tauri 命令的 `?` 操作符（rss_core::Error -> String）。
impl From<Error> for String {
    fn from(e: Error) -> Self {
        e.to_string()
    }
}
