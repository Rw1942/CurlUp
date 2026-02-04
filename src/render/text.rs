//! # Terminal Text Rendering Engine
//!
//! This module formats extracted web content for terminal display.
//! Content extraction (removing ads, navigation, boilerplate) is handled
//! by the Readability algorithm in the extraction phase.
//!
//! This module focuses on:
//! - **Light noise filtering**: Removing any residual UI elements
//! - **Terminal formatting**: Line wrapping and visual presentation
//! - **Preformatted preservation**: Keeping ASCII art and tables intact
//! - **Link rendering**: Displaying clickable links for navigation

mod links;
mod util;

use crate::dom::content::PageContent;
use self::links::build_render_with_links_lines;
use self::util::{clean_blank_lines, is_preformatted, terminal_width, wrap_text};

// ============================================================================
// PUBLIC API
// ============================================================================

/// Render text in raw mode (minimal formatting, preserves all content).
///
/// Use this for scripting, piping to other tools, or when users want
/// unprocessed output. Still applies basic line wrapping for terminal width.
pub fn render(lines: &[String]) {
    let width = terminal_width();
    for line in lines {
        if !line.is_empty() {
            for wrapped in wrap_text(line, width, 0) {
                println!("{}", wrapped);
            }
            println!();
        }
    }
}

/// Render text in formatted mode with light cleanup.
///
/// Since Readability handles main content extraction, this just:
/// - Filters any residual noise (icon ligatures, etc.)
/// - Wraps text for terminal width
/// - Preserves preformatted content (ASCII art)
pub fn render_condensed(lines: &[String]) {
    let width = terminal_width();
    let formatted = format_for_terminal(lines, width);
    for line in formatted {
        println!("{}", line);
    }
}

/// Render page content with links, limited to a maximum number of lines.
///
/// Returns true if output was truncated.
pub fn render_with_links_limited(content: &PageContent, max_lines: usize) -> bool {
    if max_lines == 0 {
        return true;
    }

    let width = terminal_width();
    let lines = build_render_with_links_lines(content, width);

    if lines.len() <= max_lines {
        for line in lines {
            println!("{}", line);
        }
        return false;
    }

    let visible_lines = max_lines.saturating_sub(1).max(1);
    for line in lines.iter().take(visible_lines) {
        println!("{}", line);
    }
    println!("  ... more content below");
    true
}

// ============================================================================
// NOISE FILTERING
// ============================================================================

/// Material Design icon names that appear as text in web pages.
///
/// Google's Material Design uses ligatures that render as icons in browsers
/// but appear as text when extracted. These should be removed.
const MATERIAL_ICONS: &[&str] = &[
    "chevron_right",
    "chevron_left",
    "expand_more",
    "expand_less",
    "arrow_forward",
    "arrow_back",
    "close",
    "menu",
    "search",
    "share",
    "bookmark",
    "bookmark_border",
    "notifications",
    "settings",
    "account_circle",
    "more_vert",
    "more_horiz",
];

/// Phrases that should be filtered out (residual UI elements).
const NOISE_PHRASES: &[&str] = &[
    "ADVERTISEMENT",
    "SKIP ADVERTISEMENT",
    "Skip to content",
    "SKIP TO CONTENT",
    "SKIP TO MAIN CONTENT",
    "Cookie Policy",
    "Privacy Policy",
    "Terms of Service",
];

/// Filter and clean lines before formatting.
///
/// This performs light cleanup since Readability already handled
/// the heavy lifting of content extraction.
fn filter_noise(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .map(|s| {
            let mut cleaned = s.trim().to_string();
            // Remove icon names from text
            for icon in MATERIAL_ICONS {
                cleaned = cleaned.replace(icon, "");
            }
            // Normalize whitespace
            cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
        })
        .filter(|s| {
            if s.is_empty() {
                return false;
            }
            // Exact match noise
            if NOISE_PHRASES.iter().any(|&n| *s == n) {
                return false;
            }
            // Empty-ish lines with just symbols
            if s.chars().all(|c| !c.is_alphanumeric()) {
                return false;
            }
            true
        })
        .collect()
}

// ============================================================================
// FORMATTING
// ============================================================================

/// Format lines for terminal display.
pub fn format_for_terminal(lines: &[String], width: usize) -> Vec<String> {
    let filtered = filter_noise(lines);
    let mut output = Vec::new();

    for line in &filtered {
        // Preserve preformatted content (ASCII art, tables)
        if is_preformatted(line) {
            output.push(line.clone());
            continue;
        }

        // Wrap regular text
        let wrapped = wrap_text(line, width, 0);
        for w in wrapped {
            output.push(w);
        }
    }

    clean_blank_lines(output)
}

/// Format with URL context (for link rendering).
/// 
/// This is called by the links module.
pub(crate) fn format_for_terminal_with_url(
    _url: Option<&str>,
    lines: &[String],
    width: usize,
) -> Vec<String> {
    // URL context is no longer needed since we're not doing site-specific parsing
    format_for_terminal(lines, width)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_noise_removes_icons() {
        let lines = vec![
            "chevron_right Next Page".to_string(),
            "Actual content here".to_string(),
        ];
        let filtered = filter_noise(&lines);
        assert_eq!(filtered, vec!["Next Page", "Actual content here"]);
    }

    #[test]
    fn test_filter_noise_removes_empty() {
        let lines = vec![
            "Content".to_string(),
            "   ".to_string(),
            "More content".to_string(),
        ];
        let filtered = filter_noise(&lines);
        assert_eq!(filtered, vec!["Content", "More content"]);
    }

    #[test]
    fn test_filter_noise_removes_symbols_only() {
        let lines = vec![
            "Content".to_string(),
            "---".to_string(),
            "More".to_string(),
        ];
        let filtered = filter_noise(&lines);
        assert_eq!(filtered, vec!["Content", "More"]);
    }
}
