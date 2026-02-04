//! # Terminal Text Rendering Engine
//!
//! This module formats extracted web content for terminal display using
//! the html2text crate with rich annotations for ANSI styling.
//!
//! Colors are sourced from the centralized `brand` module for consistency.

use html2text::render::RichAnnotation;

use crate::brand::{
    RESET, BOLD, DIM, ITALIC, UNDERLINE,
    TEAL, CORAL, SLATE, BG_CHARCOAL,
};

// ============================================================================
// PUBLIC API
// ============================================================================

/// Render HTML to rich terminal text at a specific width.
/// Uses ANSI escape codes for bold, italic, code, etc.
pub fn render_html_to_width(html: &str, width: usize) -> Vec<String> {
    // Use rich mode with our custom colour mapper
    match html2text::config::rich()
        .coloured(html.as_bytes(), width, colour_map)
    {
        Ok(text) => {
            // Post-process: convert ASCII table borders to Unicode
            let with_unicode_tables = convert_table_borders(&text);
            // Post-process: enhance blockquotes
            let with_blockquotes = enhance_blockquotes(&with_unicode_tables);
            with_blockquotes.lines().map(|s| s.to_string()).collect()
        }
        Err(_) => vec!["[Error rendering HTML]".to_string()],
    }
}

// ============================================================================
// COLOUR MAPPING
// ============================================================================

/// Map RichAnnotations to ANSI escape sequences.
/// Uses a modern color palette for better readability and visual appeal.
fn colour_map(annotations: &[RichAnnotation], text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }

    // Collect active styles
    let mut styles = Vec::new();
    
    for ann in annotations {
        match ann {
            RichAnnotation::Strong => {
                styles.push(BOLD);
            }
            RichAnnotation::Emphasis => {
                styles.push(ITALIC);
                styles.push(CORAL); // Warm coral for emphasis - eye-catching but not harsh
            }
            RichAnnotation::Code => {
                styles.push(BG_CHARCOAL); // Subtle dark background
                styles.push(TEAL);         // Teal text for code - modern and readable
            }
            RichAnnotation::Preformat(_) => {
                styles.push(SLATE); // Muted slate for preformatted blocks
            }
            RichAnnotation::Strikeout => {
                styles.push(DIM);
            }
            RichAnnotation::Link(_) => {
                // Links are handled by our own marker system
                // Subtle underline to indicate interactivity
                styles.push(UNDERLINE);
            }
            _ => {}
        }
    }

    if styles.is_empty() {
        text.to_string()
    } else {
        format!("{}{}{}", styles.join(""), text, RESET)
    }
}

// ============================================================================
// TABLE BORDER CONVERSION
// ============================================================================

/// Convert ASCII table borders to Unicode box-drawing characters.
fn convert_table_borders(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let lines: Vec<&str> = text.lines().collect();
    
    for (i, line) in lines.iter().enumerate() {
        let prev_line = if i > 0 { Some(lines[i - 1]) } else { None };
        let next_line = lines.get(i + 1).copied();
        
        let converted = convert_line_borders(line, prev_line, next_line);
        result.push_str(&converted);
        result.push('\n');
    }
    
    // Remove trailing newline
    if result.ends_with('\n') {
        result.pop();
    }
    
    result
}

/// Convert a single line's table borders to Unicode.
fn convert_line_borders(line: &str, prev: Option<&str>, next: Option<&str>) -> String {
    let mut result = String::with_capacity(line.len() * 2);
    let chars: Vec<char> = line.chars().collect();
    
    for (i, &c) in chars.iter().enumerate() {
        let converted = match c {
            '+' => {
                // Determine which Unicode corner/junction to use
                let above = prev.and_then(|p| p.chars().nth(i));
                let below = next.and_then(|n| n.chars().nth(i));
                let left = if i > 0 { chars.get(i - 1).copied() } else { None };
                let right = chars.get(i + 1).copied();
                
                get_box_char(above, below, left, right)
            }
            '-' => '─',
            '|' => '│',
            _ => c,
        };
        result.push(converted);
    }
    
    result
}

/// Determine the appropriate Unicode box-drawing character based on neighbors.
fn get_box_char(above: Option<char>, below: Option<char>, left: Option<char>, right: Option<char>) -> char {
    let has_above = matches!(above, Some('|') | Some('+') | Some('│'));
    let has_below = matches!(below, Some('|') | Some('+') | Some('│'));
    let has_left = matches!(left, Some('-') | Some('+') | Some('─'));
    let has_right = matches!(right, Some('-') | Some('+') | Some('─'));
    
    match (has_above, has_below, has_left, has_right) {
        // Corners
        (false, true, false, true) => '┌',  // top-left
        (false, true, true, false) => '┐',  // top-right
        (true, false, false, true) => '└',  // bottom-left
        (true, false, true, false) => '┘',  // bottom-right
        
        // T-junctions
        (false, true, true, true) => '┬',   // top T
        (true, false, true, true) => '┴',   // bottom T
        (true, true, false, true) => '├',   // left T
        (true, true, true, false) => '┤',   // right T
        
        // Cross
        (true, true, true, true) => '┼',
        
        // Edges (shouldn't normally happen with +)
        (true, true, false, false) => '│',
        (false, false, true, true) => '─',
        
        // Default fallback
        _ => '┼',
    }
}

// ============================================================================
// BLOCKQUOTE ENHANCEMENT
// ============================================================================

/// Enhance blockquotes with a colored left border.
fn enhance_blockquotes(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    
    for line in text.lines() {
        // Detect blockquote lines (html2text uses > prefix)
        if line.trim_start().starts_with('>') {
            // Replace the > with a styled vertical bar
            let content = line.trim_start().strip_prefix('>').unwrap_or(line);
            result.push_str(&format!("{}│{} {}", TEAL, RESET, content.trim_start()));
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }
    
    // Remove trailing newline
    if result.ends_with('\n') {
        result.pop();
    }
    
    result
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
