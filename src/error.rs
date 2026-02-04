use thiserror::Error;

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum CurlUpError {
    #[error("failed to connect to WebDriver")]
    Connection,

    #[error("navigation failed: {0}")]
    Navigation(String),

    #[error("DOM extraction failed: {0}")]
    Dom(String),

    #[error("browser error: {0}")]
    Browser(String),
}
