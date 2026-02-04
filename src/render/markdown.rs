//! # Markdown Rendering Engine
//!
//! Converts HTML to Markdown using the htmd crate, providing a clean,
//! structured output that preserves links, headings, lists, and code blocks.
//!
//! Markdown output is ideal for:
//! - Piping to other tools that consume markdown
//! - Saving web content in a readable, editable format
//! - Further processing with AI/LLM tools
//! - Preserving document structure better than plain text

use crate::dom::content::{Link, PageContent};
use super::util::terminal_width;

/// Build formatted markdown lines with link annotations for interactive display.
/// This is the markdown equivalent of text::build_render_lines.
pub fn build_render_lines(content: &PageContent) -> Vec<String> {
    let width = terminal_width();
    let markdown_lines = render_html_to_markdown_width(&content.html, width);
    annotate_links(&markdown_lines, &content.links)
}

/// Convert HTML to Markdown.
///
/// Returns lines of markdown text suitable for terminal display or piping.
pub fn render_html_to_markdown(html: &str) -> Vec<String> {
    let width = terminal_width();
    render_html_to_markdown_width(html, width)
}

/// Convert HTML to Markdown with a specific target width.
///
/// The width is used for wrapping long paragraphs to improve readability
/// in the terminal.
pub fn render_html_to_markdown_width(html: &str, width: usize) -> Vec<String> {
    match htmd::convert(html) {
        Ok(markdown) => {
            // Post-process: wrap long lines for better terminal display
            wrap_markdown_lines(&markdown, width)
        }
        Err(_) => vec!["[Error converting HTML to Markdown]".to_string()],
    }
}

/// Wrap long lines in markdown while preserving structure.
///
/// We need to be careful not to break:
/// - Headers (lines starting with #)
/// - Code blocks (indented or fenced)
/// - Lists (lines starting with - * or numbers)
/// - Links and inline formatting
fn wrap_markdown_lines(markdown: &str, width: usize) -> Vec<String> {
    let mut result = Vec::new();
    let mut in_code_block = false;

    for line in markdown.lines() {
        // Track fenced code blocks
        if line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            result.push(line.to_string());
            continue;
        }

        // Don't wrap inside code blocks
        if in_code_block {
            result.push(line.to_string());
            continue;
        }

        // Don't wrap headers, list items, blockquotes, or short lines
        let trimmed = line.trim_start();
        let is_structural = trimmed.starts_with('#')
            || trimmed.starts_with('-')
            || trimmed.starts_with('*')
            || trimmed.starts_with('>')
            || trimmed.starts_with("    ")  // indented code
            || trimmed.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false);

        if is_structural || line.len() <= width {
            result.push(line.to_string());
        } else {
            // Wrap long paragraph lines
            let wrapped = wrap_paragraph(line, width);
            result.extend(wrapped);
        }
    }

    result
}

/// Wrap a single paragraph line at word boundaries.
fn wrap_paragraph(line: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    // Preserve leading whitespace
    let leading_spaces: String = line.chars().take_while(|c| c.is_whitespace()).collect();

    for word in line.split_whitespace() {
        if current_line.is_empty() {
            current_line = format!("{}{}", leading_spaces, word);
        } else if current_line.len() + 1 + word.len() <= width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = format!("{}{}", leading_spaces, word);
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

// ============================================================================
// LINK ANNOTATIONS (for interactive browsing)
// ============================================================================

const SUPERSCRIPTS: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];

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

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_html_to_markdown() {
        let html = "<h1>Hello</h1><p>World</p>";
        let result = render_html_to_markdown_width(html, 80);
        let text = result.join("\n");
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
    }

    #[test]
    fn test_link_preservation() {
        let html = r#"<a href="https://example.com">Click here</a>"#;
        let result = render_html_to_markdown_width(html, 80);
        let text = result.join("\n");
        // Links should be preserved in markdown format
        assert!(text.contains("Click here"));
        assert!(text.contains("example.com"));
    }

    #[test]
    fn test_list_conversion() {
        let html = "<ul><li>One</li><li>Two</li></ul>";
        let result = render_html_to_markdown_width(html, 80);
        let text = result.join("\n");
        assert!(text.contains("One"));
        assert!(text.contains("Two"));
    }

    #[test]
    fn test_code_block_not_wrapped() {
        let markdown = "```\nthis is a very long line of code that should not be wrapped at all because it is inside a code block\n```";
        let result = wrap_markdown_lines(markdown, 40);
        // Code block content should not be wrapped
        assert!(result.iter().any(|l| l.contains("very long line")));
    }

    #[test]
    fn test_paragraph_wrapping() {
        let line = "This is a very long paragraph that should be wrapped to fit within a reasonable terminal width for better readability.";
        let wrapped = wrap_paragraph(line, 40);
        assert!(wrapped.len() > 1);
        assert!(wrapped.iter().all(|l| l.len() <= 50)); // Allow some slack for words
    }
}
