// placeholder

#[derive(Debug, thiserror::Error)]
pub enum BrowserError {
    #[error("browser error: {0}")]
    Other(String),
}
