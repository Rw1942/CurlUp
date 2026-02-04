//! Inline link annotation for terminal display.

use crate::dom::content::{Link, PageContent};
use super::format_for_terminal;

const SUPERSCRIPTS: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];

/// Build formatted lines with link annotations for display.
/// Returns vector of strings ready for printing.
pub fn build_render_with_links_lines(content: &PageContent, width: usize) -> Vec<String> {
    // Render HTML to terminal text using html2text
    let formatted = format_for_terminal(&content.html, width);
    annotate_links(&formatted, &content.links)
}

fn footnote_marker(num: usize) -> String {
    if num == 0 || num > 20 {
        return String::new();
    }
    if num <= 9 {
        format!("\x1b[2m{}\x1b[0m", SUPERSCRIPTS[num])
    } else {
        format!("\x1b[2m[{}]\x1b[0m", num)
    }
}

fn annotate_links(lines: &[String], links: &[Link]) -> Vec<String> {
    let mut result = lines.to_vec();
    
    for (idx, link) in links.iter().take(20).enumerate() {
        // Skip short links (navigation-like)
        if link.text.len() < 8 {
            continue;
        }
        
        let marker = footnote_marker(idx + 1);
        let needle = link.text.to_lowercase();
        
        for line in &mut result {
            if let Some(pos) = line.to_lowercase().find(&needle) {
                let end = pos + link.text.len();
                if end <= line.len() {
                    let before = &line[..end];
                    let after = &line[end..];
                    *line = format!("{}{}{}", before, marker, after);
                    break;
                }
            }
        }
    }
    
    result
}
