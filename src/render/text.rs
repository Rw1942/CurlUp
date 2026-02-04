//! # Terminal Text Rendering Engine
//!
//! This module formats extracted web content for terminal display using
//! the html2text crate for clean HTML-to-text conversion.
//!
//! This module focuses on:
//! - **HTML rendering**: Converting HTML to readable terminal text via html2text
//! - **Link rendering**: Displaying clickable links with numbered annotations

mod links;
mod util;

use crate::dom::content::PageContent;
pub use self::links::build_render_with_links_lines;
use self::util::terminal_width;

/// Build formatted lines with link annotations without printing.
/// Use this when you need to handle display yourself (e.g., scrolling).
pub fn build_render_lines(content: &PageContent) -> Vec<String> {
    let width = terminal_width();
    build_render_with_links_lines(content, width)
}

// ============================================================================
// PUBLIC API
// ============================================================================

/// Render HTML to plain text for terminal display.
///
/// Uses html2text for clean text conversion with proper wrapping,
/// list formatting, table rendering, and preformatted block handling.
pub fn render_html(html: &str) -> Vec<String> {
    let width = terminal_width();
    render_html_to_width(html, width)
}

/// Render HTML to plain text at a specific width.
pub fn render_html_to_width(html: &str, width: usize) -> Vec<String> {
    match html2text::from_read(html.as_bytes(), width) {
        Ok(text) => text.lines().map(|s| s.to_string()).collect(),
        Err(_) => vec!["[Error rendering HTML]".to_string()],
    }
}


/// Format lines for terminal display using html2text.
/// Called by the links module for content with link annotations.
pub fn format_for_terminal(html: &str, width: usize) -> Vec<String> {
    render_html_to_width(html, width)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_html_basic() {
        let html = "<p>Hello world</p>";
        let result = render_html_to_width(html, 80);
        assert!(!result.is_empty());
        assert!(result.iter().any(|line| line.contains("Hello world")));
    }

    #[test]
    fn test_render_html_list() {
        let html = "<ul><li>Item one</li><li>Item two</li></ul>";
        let result = render_html_to_width(html, 80);
        let text = result.join("\n");
        assert!(text.contains("Item one"));
        assert!(text.contains("Item two"));
    }

    #[test]
    fn test_render_html_heading() {
        let html = "<h1>Main Title</h1><p>Some content</p>";
        let result = render_html_to_width(html, 80);
        let text = result.join("\n");
        assert!(text.contains("Main Title"));
        assert!(text.contains("Some content"));
    }
}
