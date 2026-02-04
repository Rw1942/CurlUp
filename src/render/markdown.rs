//! Markdown output for piping to other tools.
//!
//! Converts HTML to Markdown using htmd for use with --markdown flag.

use crate::term::terminal_width;

/// Convert HTML to Markdown lines.
pub fn render_html_to_markdown(html: &str) -> Vec<String> {
    let width = terminal_width();
    match htmd::convert(html) {
        Ok(markdown) => wrap_and_clean(&markdown, width),
        Err(_) => vec!["[Error converting HTML to Markdown]".to_string()],
    }
}

/// Wrap long lines and clean up markdown for output.
fn wrap_and_clean(markdown: &str, width: usize) -> Vec<String> {
    let mut result = Vec::new();
    let mut in_code_block = false;
    let mut prev_blank = false;

    for line in markdown.lines() {
        // Track fenced code blocks
        if line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            result.push(line.to_string());
            prev_blank = false;
            continue;
        }

        // Don't wrap inside code blocks
        if in_code_block {
            result.push(line.to_string());
            prev_blank = false;
            continue;
        }

        // Collapse multiple blank lines
        let is_blank = line.trim().is_empty();
        if is_blank {
            if !prev_blank {
                result.push(String::new());
            }
            prev_blank = true;
            continue;
        }
        prev_blank = false;

        // Don't wrap structural lines (headers, lists, etc.)
        let trimmed = line.trim_start();
        let is_structural = trimmed.starts_with('#')
            || trimmed.starts_with('-')
            || trimmed.starts_with('*')
            || trimmed.starts_with('>')
            || trimmed.starts_with("    ")
            || trimmed.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false);

        if is_structural || line.len() <= width {
            result.push(line.to_string());
        } else {
            // Wrap long paragraph lines
            result.extend(wrap_line(line, width));
        }
    }

    result
}

/// Wrap a single line at word boundaries.
fn wrap_line(line: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();

    for word in line.split_whitespace() {
        if current.is_empty() {
            current = format!("{}{}", indent, word);
        } else if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = format!("{}{}", indent, word);
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}
