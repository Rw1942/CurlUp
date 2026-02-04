use std::env;

/// Get the current terminal width.
///
/// Reads from the `COLUMNS` environment variable, defaulting to 80 columns
/// if not set. This is the standard way to detect terminal width in Unix.
///
/// # Returns
/// Terminal width in characters (minimum 40, default 80)
pub(super) fn terminal_width() -> usize {
    env::var("COLUMNS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(80)
        .max(40) // Minimum usable width
}

/// Check if text appears to be ASCII art or preformatted content.
///
/// Preformatted content should not be wrapped as it would break the visual
/// structure. We detect this by looking for:
///
/// - Box drawing characters (│ ┌ ┐ └ ┘ ├ ┤ ─ etc.)
/// - ASCII art patterns (.-. \_( etc.)
/// - High ratio of special characters (>40%)
///
/// # Arguments
/// * `text` - The text line to check
///
/// # Returns
/// `true` if the text should be preserved without wrapping
pub(super) fn is_preformatted(text: &str) -> bool {
    // Box drawing characters (Unicode)
    let box_chars = ['│', '┌', '┐', '└', '┘', '├', '┤', '┬', '┴', '┼', '─', '═', '║'];
    let has_box_chars = text.chars().any(|c| box_chars.contains(&c));

    // Common ASCII art patterns
    let ascii_patterns = [
        ".-.", ".--.", "(_", ".--(", "/ \\", ".-'", "`-'", "---", "===", "___)",
    ];
    let has_ascii_pattern = ascii_patterns.iter().any(|p| text.contains(p));

    // High ratio of special characters suggests preformatted content
    let special_count = text
        .chars()
        .filter(|c| !c.is_alphanumeric() && !c.is_whitespace())
        .count();
    let high_special_ratio = text.len() > 10 && special_count as f32 / text.len() as f32 > 0.4;

    has_box_chars || has_ascii_pattern || high_special_ratio
}

/// Wrap text to fit within the specified width, preserving word boundaries.
///
/// Handles continuation lines with proper indentation. Does not wrap
/// preformatted content (ASCII art, tables, etc.).
///
/// # Arguments
/// * `text` - The text to wrap
/// * `width` - Maximum line width in characters
/// * `indent` - Number of spaces to indent continuation lines
///
/// # Returns
/// Vector of wrapped lines
pub(super) fn wrap_text(text: &str, width: usize, indent: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }

    // Don't wrap preformatted/ASCII art content
    if is_preformatted(text) {
        return vec![text.to_string()];
    }

    let indent_str = " ".repeat(indent);
    let effective_width = width.saturating_sub(indent);

    // Minimum width for wrapping to make sense
    if effective_width < 20 {
        return vec![format!("{}{}", indent_str, text)];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() <= effective_width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            // Line is full, start a new one
            lines.push(format!(
                "{}{}",
                if lines.is_empty() { "" } else { &indent_str },
                current_line
            ));
            current_line = word.to_string();
        }
    }

    // Don't forget the last line
    if !current_line.is_empty() {
        lines.push(format!(
            "{}{}",
            if lines.is_empty() { "" } else { &indent_str },
            current_line
        ));
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

/// Create a horizontal separator line.
///
/// # Arguments
/// * `width` - Desired width (capped at 80)
/// * `ch` - Character to repeat
pub(super) fn separator(width: usize, ch: char) -> String {
    ch.to_string().repeat(width.min(80))
}

/// Remove excessive consecutive blank lines.
///
/// Limits to max 2 consecutive blank lines and trims leading/trailing blanks.
pub(super) fn clean_blank_lines(lines: Vec<String>) -> Vec<String> {
    let mut result = Vec::new();
    let mut blank_count = 0;

    for line in lines {
        if line.trim().is_empty() {
            blank_count += 1;
            if blank_count <= 2 {
                result.push(String::new());
            }
        } else {
            blank_count = 0;
            result.push(line);
        }
    }

    // Trim leading blanks
    while result.first().map(|s| s.is_empty()).unwrap_or(false) {
        result.remove(0);
    }
    // Trim trailing blanks
    while result.last().map(|s| s.is_empty()).unwrap_or(false) {
        result.pop();
    }

    result
}
