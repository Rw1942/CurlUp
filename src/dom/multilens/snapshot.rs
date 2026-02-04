//! DOM snapshot acquisition from the browser.
//!
//! Fetches the fully rendered HTML from the browser after JavaScript execution,
//! giving us the complete DOM as the user would see it.

use anyhow::{Context, Result};
use fantoccini::Client;

/// JavaScript to extract the full rendered HTML document.
const OUTER_HTML_JS: &str = "return document.documentElement.outerHTML;";

/// Fetch the fully rendered HTML from the browser.
///
/// This returns the complete DOM including any dynamically loaded content,
/// unlike a simple HTTP fetch which only gets the initial HTML.
pub async fn fetch_rendered_html(client: &Client) -> Result<String> {
    let result = client
        .execute(OUTER_HTML_JS, vec![])
        .await
        .context("Failed to execute outerHTML script")?;

    // The result is a JSON value, extract the string
    let html = result
        .as_str()
        .context("outerHTML result was not a string")?
        .to_string();

    Ok(html)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outer_html_js_is_valid() {
        // Just verify the JS string is properly formed
        assert!(OUTER_HTML_JS.contains("outerHTML"));
    }
}
