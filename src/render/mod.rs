pub mod text;

use crate::brand::{
    RESET, BOLD,
    TEAL, SKY, LAVENDER, WHITE,
    link_marker,
};
use crate::dom::content::{Link, PageContent};
use crate::dom::link_filter::MAX_LINKS;
use crate::term::terminal_width;

// ============================================================================
// UNIFIED RENDERING API
// ============================================================================

/// Build formatted lines with link annotations for interactive display.
/// Injects link and header markers into HTML before rendering so they survive html2text conversion.
pub fn build_render_lines(content: &PageContent) -> Vec<String> {
    let width = terminal_width();
    
    // Inject header markers first
    let html_with_headers = inject_header_markers(&content.html);
    
    // Inject link markers
    let html_with_markers = inject_link_markers(&html_with_headers, &content.links);
    
    // Render HTML to text (markers survive as plain text)
    let text_lines = text::render_html_to_width(&html_with_markers, width);
    
    // Colorize link markers and style headers
    let with_links = colorize_link_markers(&text_lines);
    style_headers(&with_links, width)
}

// ============================================================================
// HEADER MARKER INJECTION
// ============================================================================

/// Inject header markers into HTML that survive html2text conversion.
/// Format: ‹H1› before content, ‹/H1› after for h1, etc.
fn inject_header_markers(html: &str) -> String {
    let mut result = html.to_string();
    
    // Process headers h1-h6
    for level in 1..=6 {
        let open_tag = format!("<h{}", level);
        let close_tag = format!("</h{}>", level);
        let start_marker = format!("‹H{}›", level);
        let end_marker = format!("‹/H{}›", level);
        
        // Find all instances of this header level
        let mut search_pos = 0;
        while let Some(tag_start) = result[search_pos..].find(&open_tag) {
            let absolute_start = search_pos + tag_start;
            
            // Find the end of the opening tag (the >)
            if let Some(tag_end_offset) = result[absolute_start..].find('>') {
                let content_start = absolute_start + tag_end_offset + 1;
                
                // Find the closing tag
                if let Some(close_offset) = result[content_start..].find(&close_tag) {
                    let close_pos = content_start + close_offset;
                    
                    // Insert end marker before closing tag
                    result.insert_str(close_pos, &end_marker);
                    // Insert start marker after opening tag
                    result.insert_str(content_start, &start_marker);
                    
                    // Move past this header (account for inserted markers)
                    search_pos = close_pos + end_marker.len() + start_marker.len() + close_tag.len();
                } else {
                    search_pos = content_start;
                }
            } else {
                search_pos = absolute_start + 1;
            }
        }
    }
    
    result
}

/// Style headers by finding markers and applying ANSI codes.
fn style_headers(lines: &[String], width: usize) -> Vec<String> {
    let mut result = Vec::with_capacity(lines.len());
    
    for line in lines {
        let styled = style_header_line(line, width);
        result.push(styled);
    }
    
    result
}

/// Style a single line, processing any header markers.
fn style_header_line(line: &str, width: usize) -> String {
    let mut result = line.to_string();
    
    // Process each header level
    for level in 1..=6 {
        let start_marker = format!("‹H{}›", level);
        let end_marker = format!("‹/H{}›", level);
        
        // Replace markers with styled content
        while let (Some(start_pos), Some(end_pos)) = (result.find(&start_marker), result.find(&end_marker)) {
            if start_pos < end_pos {
                let before = &result[..start_pos];
                let content = &result[start_pos + start_marker.len()..end_pos];
                let after = &result[end_pos + end_marker.len()..];
                
                // Get header styling based on level
                let (color, style, add_underline) = header_style(level);
                
                // Build styled header
                let mut styled_header = format!("{}{}{}{}", style, color, content, RESET);
                
                // Add underline below for h1/h2
                if add_underline {
                    let underline_char = if level == 1 { '═' } else { '─' };
                    let content_len = content.chars().count();
                    let underline: String = std::iter::repeat(underline_char).take(content_len.min(width)).collect();
                    styled_header = format!("{}\n{}{}{}", styled_header, color, underline, RESET);
                }
                
                result = format!("{}{}{}", before, styled_header, after);
            } else {
                break;
            }
        }
    }
    
    result
}

/// Get styling for a header level.
/// Returns (color, style, should_add_underline)
/// Uses the centralized brand palette for consistent styling.
fn header_style(level: usize) -> (&'static str, &'static str, bool) {
    match level {
        1 => (TEAL, BOLD, true),            // h1: Bold teal with double underline
        2 => (SKY, BOLD, true),             // h2: Bold sky blue with single underline
        3 => (LAVENDER, BOLD, false),       // h3: Bold lavender
        4 => (WHITE, BOLD, false),          // h4: Bold white
        5 => (WHITE, "", false),            // h5: White
        _ => (WHITE, "", false),            // h6: White
    }
}

// ============================================================================
// LINK MARKER INJECTION
// ============================================================================

/// Placeholder format that survives html2text conversion.
/// Format: «N» where N is the link number (1-20).
fn placeholder_marker(num: usize) -> String {
    format!("«{}»", num)
}

/// Creates a visually distinct colored link marker.
/// Uses consistent bracketed format [1] [2] ... [20] with teal background.
/// Delegates to the centralized brand module for consistent styling.
fn colored_marker(num: usize) -> String {
    if num == 0 || num > MAX_LINKS {
        return String::new();
    }
    link_marker(num)
}

/// Inject placeholder markers into HTML after each link's text.
/// This ensures markers survive html2text conversion.
fn inject_link_markers(html: &str, links: &[Link]) -> String {
    let mut result = html.to_string();
    
    for (idx, link) in links.iter().take(MAX_LINKS).enumerate() {
        let num = idx + 1;
        let marker = placeholder_marker(num);
        
        // Find and mark links by their href
        // We inject the marker just before </a> for links with matching href
        if let Some(marked) = inject_marker_for_link(&result, &link.href, &marker) {
            result = marked;
        }
    }
    
    result
}

/// Inject a marker into ALL <a> tags with matching href.
/// Returns Some(modified_html) if at least one match found, None if not found.
fn inject_marker_for_link(html: &str, href: &str, marker: &str) -> Option<String> {
    // Find <a> tags with this href and inject marker before </a>
    // We need to handle various href formats (with/without quotes, encoded, etc.)
    
    // Normalize the href for matching
    let href_normalized = href.trim();
    
    // Search for anchor tags - try common patterns
    let patterns = [
        format!(r#"href="{}""#, href_normalized),
        format!(r#"href='{}'"#, href_normalized),
        format!(r#"href={}"#, href_normalized),
    ];
    
    let mut result = html.to_string();
    let mut found_any = false;
    
    for pattern in &patterns {
        let mut search_pos = 0;
        
        while let Some(href_pos) = result[search_pos..].find(&*pattern) {
            let absolute_href_pos = search_pos + href_pos;
            
            // Find the closing </a> after this href
            if let Some(close_offset) = result[absolute_href_pos..].find("</a>") {
                let close_pos = absolute_href_pos + close_offset;
                
                // Insert marker just before </a>
                let insertion = format!(" {}", marker);
                result.insert_str(close_pos, &insertion);
                found_any = true;
                
                // Move past this tag (account for inserted marker)
                search_pos = close_pos + insertion.len() + 4; // 4 = "</a>".len()
            } else {
                search_pos = absolute_href_pos + 1;
            }
        }
    }
    
    if found_any {
        Some(result)
    } else {
        None
    }
}

/// Replace placeholder markers with colored ANSI markers in rendered text.
fn colorize_link_markers(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .map(|line| {
            let mut result = line.clone();
            for num in 1..=MAX_LINKS {
                let placeholder = placeholder_marker(num);
                let colored = colored_marker(num);
                result = result.replace(&placeholder, &colored);
            }
            result
        })
        .collect()
}
