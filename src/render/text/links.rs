//! Inline link annotation for terminal display.
//!
//! Links are marked with subtle footnote-style superscripts after the link text:
//! "Check out this article¹ about Rust programming."
//!
//! This keeps content readable while showing which text is clickable.

use crate::dom::content::{Link, PageContent};
use super::format_for_terminal_with_url;

/// Unicode superscript digits for compact link markers.
const SUPERSCRIPTS: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];

/// Build rendered lines with inline link annotations.
pub(super) fn build_render_with_links_lines(content: &PageContent, width: usize) -> Vec<String> {
    let formatted = format_for_terminal_with_url(Some(content.url.as_str()), &content.lines, width);
    annotate_links(&formatted, &content.links)
}

/// Create a dim footnote marker for a link number.
///
/// Uses Unicode superscripts (¹²³) for 1-9, bracketed numbers for 10-20.
fn footnote_marker(num: usize) -> String {
    if num == 0 || num > 20 {
        return String::new();
    }
    
    // Dim ANSI escape: \x1b[2m = dim, \x1b[0m = reset
    if num <= 9 {
        format!("\x1b[2m{}\x1b[0m", SUPERSCRIPTS[num])
    } else {
        format!("\x1b[2m[{}]\x1b[0m", num)
    }
}

/// Annotate link text with footnote-style superscript numbers.
///
/// For each link, finds its text in the content and appends a dim superscript.
/// Only the first occurrence of each link text is annotated.
fn annotate_links(lines: &[String], links: &[Link]) -> Vec<String> {
    let mut result = lines.to_vec();
    
    for (idx, link) in links.iter().take(20).enumerate() {
        // Skip short text (prone to false matches)
        if link.text.len() < 4 {
            continue;
        }
        
        let marker = footnote_marker(idx + 1);
        let needle = link.text.to_lowercase();
        
        // Find and annotate first occurrence
        for line in &mut result {
            if let Some(pos) = line.to_lowercase().find(&needle) {
                let end = pos + link.text.len();
                let before = &line[..end];
                let after = &line[end..];
                *line = format!("{}{}{}", before, marker, after);
                break; // Only annotate first occurrence
            }
        }
    }
    
    result
}
