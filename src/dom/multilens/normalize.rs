//! Text normalization and cleanup utilities.
//!
//! Handles whitespace normalization, deduplication of repeated lines,
//! and removal of common boilerplate patterns.

/// Common boilerplate phrases to filter out.
const BOILERPLATE_PATTERNS: &[&str] = &[
    "all rights reserved",
    "copyright ©",
    "copyright (c)",
    "privacy policy",
    "terms of service",
    "terms and conditions",
    "cookie policy",
    "powered by",
    "subscribe to our newsletter",
    "sign up for our",
    "follow us on",
    "share this article",
    "share on facebook",
    "share on twitter",
    "advertisement",
    "sponsored content",
    "read more",
    "click here",
    "loading...",
    "please wait",
];

/// UI/icon text patterns commonly from Material Design or similar.
const UI_PATTERNS: &[&str] = &[
    "chevron_right",
    "chevron_left",
    "expand_more",
    "expand_less",
    "arrow_forward",
    "arrow_back",
    "menu",
    "close",
    "search",
    "more_vert",
    "more_horiz",
];

/// Clean and normalize extracted text into lines.
pub fn clean_text(text: &str) -> Vec<String> {
    // Split into lines
    let lines: Vec<&str> = text.lines().collect();

    // Process each line
    let mut result: Vec<String> = Vec::new();
    let mut prev_line = String::new();
    let mut blank_count = 0;

    for line in lines {
        let trimmed = normalize_whitespace(line);

        // Skip empty lines (but allow up to 2 consecutive)
        if trimmed.is_empty() {
            blank_count += 1;
            if blank_count <= 2 && !result.is_empty() {
                // Keep one blank line for paragraph separation
                if blank_count == 1 {
                    result.push(String::new());
                }
            }
            continue;
        }

        blank_count = 0;

        // Skip very short lines (likely UI elements)
        if trimmed.len() < 3 {
            continue;
        }

        // Skip boilerplate
        if is_boilerplate(&trimmed) {
            continue;
        }

        // Skip UI patterns
        if is_ui_pattern(&trimmed) {
            continue;
        }

        // Skip exact duplicates of previous line
        if trimmed == prev_line {
            continue;
        }

        // Skip lines that are just symbols/punctuation
        if is_symbol_only(&trimmed) {
            continue;
        }

        prev_line = trimmed.clone();
        result.push(trimmed);
    }

    // Remove trailing empty lines
    while result.last().map(|s| s.is_empty()).unwrap_or(false) {
        result.pop();
    }

    result
}

/// Normalize whitespace within a line.
fn normalize_whitespace(text: &str) -> String {
    // Replace multiple spaces with single space
    let mut result = String::with_capacity(text.len());
    let mut prev_space = false;

    for c in text.chars() {
        if c.is_whitespace() {
            if !prev_space {
                result.push(' ');
                prev_space = true;
            }
        } else {
            result.push(c);
            prev_space = false;
        }
    }

    result.trim().to_string()
}

/// Check if a line is boilerplate content.
fn is_boilerplate(line: &str) -> bool {
    let lower = line.to_lowercase();

    for pattern in BOILERPLATE_PATTERNS {
        if lower.contains(pattern) {
            return true;
        }
    }

    false
}

/// Check if a line is a UI/icon pattern.
fn is_ui_pattern(line: &str) -> bool {
    let lower = line.to_lowercase();

    for pattern in UI_PATTERNS {
        if lower == *pattern {
            return true;
        }
    }

    // Also check for common icon-only patterns
    if line.len() <= 2 {
        let c = line.chars().next().unwrap_or(' ');
        // Common icon characters
        if "×✕✖✗✘☰☱≡▼▲◀▶←→↑↓⋮⋯".contains(c) {
            return true;
        }
    }

    false
}

/// Check if a line is only symbols/punctuation.
fn is_symbol_only(line: &str) -> bool {
    if line.is_empty() {
        return true;
    }

    // Allow lines that have at least some alphanumeric content
    !line.chars().any(|c| c.is_alphanumeric())
}

/// Join lines back into a single string with proper spacing.
pub fn join_lines(lines: &[String]) -> String {
    lines.join("\n")
}

/// Additional cleanup for final output.
pub fn final_cleanup(text: &str) -> String {
    let mut result = text.to_string();

    // Remove excessive newlines (max 2 consecutive)
    while result.contains("\n\n\n") {
        result = result.replace("\n\n\n", "\n\n");
    }

    // Trim overall
    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_whitespace() {
        assert_eq!(normalize_whitespace("  hello   world  "), "hello world");
        assert_eq!(normalize_whitespace("no\textra\tspaces"), "no extra spaces");
    }

    #[test]
    fn test_skip_boilerplate() {
        assert!(is_boilerplate("© 2024 All Rights Reserved"));
        assert!(is_boilerplate("Read our Privacy Policy"));
        assert!(!is_boilerplate("This is regular content"));
    }

    #[test]
    fn test_skip_ui_patterns() {
        assert!(is_ui_pattern("chevron_right"));
        assert!(is_ui_pattern("expand_more"));
        assert!(!is_ui_pattern("actual content"));
    }

    #[test]
    fn test_clean_text() {
        let input = r#"
            Main content here
            
            Another paragraph
            
            
            
            © 2024 All Rights Reserved
            chevron_right
            More actual content
        "#;

        let lines = clean_text(input);
        assert!(lines.iter().any(|l| l.contains("Main content")));
        assert!(lines.iter().any(|l| l.contains("More actual")));
        assert!(!lines.iter().any(|l| l.contains("All Rights")));
        assert!(!lines.iter().any(|l| l.contains("chevron")));
    }

    #[test]
    fn test_symbol_only() {
        assert!(is_symbol_only("---"));
        assert!(is_symbol_only("***"));
        assert!(!is_symbol_only("a---b"));
        assert!(!is_symbol_only("123"));
    }
}
