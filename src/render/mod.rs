pub mod markdown;
pub mod text;

use crate::dom::content::{Link, PageContent};
use crate::dom::link_filter::MAX_LINKS;
use crate::term::terminal_width;

// ============================================================================
// UNIFIED RENDERING API
// ============================================================================

/// Build formatted lines with link annotations for interactive display.
/// Uses html2text for proper HTML rendering (handles entities correctly).
pub fn build_render_lines(content: &PageContent) -> Vec<String> {
    let width = terminal_width();
    let text_lines = text::render_html_to_width(&content.html, width);
    annotate_links(&text_lines, &content.links)
}

// ============================================================================
// LINK ANNOTATIONS
// ============================================================================

/// Creates a visually distinct link marker.
/// Uses consistent bracketed format [1] [2] ... [20] with cyan background.
fn footnote_marker(num: usize) -> String {
    if num == 0 || num > MAX_LINKS {
        return String::new();
    }
    // Cyan background (46), black text (30), bold (1)
    format!("\x1b[46;30;1m[{}]\x1b[0m", num)
}

/// Annotate rendered text with link markers.
/// Numbers are 1-based and match the link indices exactly.
fn annotate_links(lines: &[String], links: &[Link]) -> Vec<String> {
    let mut result = lines.to_vec();

    for (idx, link) in links.iter().take(MAX_LINKS).enumerate() {
        let num = idx + 1;
        let marker = footnote_marker(num);

        // Try to find link text in rendered content
        if let Some((line_idx, _start, end)) = find_link_text(&result, &link.text) {
            let line = &result[line_idx];
            let before = &line[..end];
            let after = &line[end..];
            result[line_idx] = format!("{}{}{}", before, marker, after);
        }
    }

    result
}

/// Find link text in rendered lines using word-boundary aware matching.
/// Returns (line_index, start_pos, end_pos) if found.
fn find_link_text(lines: &[String], needle: &str) -> Option<(usize, usize, usize)> {
    let needle_lower = needle.to_lowercase();
    let needle_words: Vec<&str> = needle_lower.split_whitespace().collect();

    if needle_words.is_empty() {
        return None;
    }

    for (line_idx, line) in lines.iter().enumerate() {
        let line_lower = line.to_lowercase();

        // Try exact match first
        if let Some(pos) = line_lower.find(&needle_lower) {
            let end = pos + needle.len();
            // Check word boundaries
            let valid_start = pos == 0
                || line
                    .chars()
                    .nth(pos.saturating_sub(1))
                    .map_or(true, |c| !c.is_alphanumeric());
            let valid_end = end >= line.len()
                || line.chars().nth(end).map_or(true, |c| !c.is_alphanumeric());

            if valid_start && valid_end && end <= line.len() {
                return Some((line_idx, pos, end));
            }
        }

        // Try matching first few significant words for longer links
        if needle_words.len() >= 3 {
            let prefix: String = needle_words[..3.min(needle_words.len())].join(" ");
            if let Some(pos) = line_lower.find(&prefix) {
                let end = pos + prefix.len();
                if end <= line.len() {
                    return Some((line_idx, pos, end));
                }
            }
        }
    }

    None
}
