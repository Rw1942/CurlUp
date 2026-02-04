//! # CurlUp Brand & Color System
//!
//! Centralized color palette and branding constants for consistent terminal styling.
//! All UI components should reference these constants for a unified visual identity.
//!
//! ## Color Philosophy
//! - **Primary (Teal)**: Main brand color, used for headers, branding, interactive elements
//! - **Accent (Coral)**: Warm highlight color for emphasis, warnings, important callouts
//! - **Secondary (Lavender)**: Soft accent for condensed mode, metadata, secondary headers
//! - **Neutral (Slate/Charcoal)**: Muted tones for backgrounds, borders, dim text
//!
//! ## ANSI 256-Color Mode
//! Using `\x1b[38;5;Nm` for foreground and `\x1b[48;5;Nm` for background
//! provides richer colors than basic 16-color ANSI while maintaining compatibility.

#![allow(dead_code)] // Constants available for future use

// ============================================================================
// TEXT STYLES
// ============================================================================

pub const RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";
pub const ITALIC: &str = "\x1b[3m";
pub const UNDERLINE: &str = "\x1b[4m";

// ============================================================================
// PRIMARY PALETTE - Core brand colors
// ============================================================================

/// Primary brand color - vibrant teal
/// Use for: main headers, branding, primary actions, links
pub const TEAL: &str = "\x1b[38;5;79m";

/// Accent highlight - warm coral
/// Use for: emphasis, warnings, important callouts, italic text
pub const CORAL: &str = "\x1b[38;5;209m";

/// Secondary accent - soft lavender
/// Use for: condensed mode, metadata, secondary headers (h3+)
pub const LAVENDER: &str = "\x1b[38;5;141m";

/// Tertiary accent - muted sky blue
/// Use for: h2 headers, secondary navigation
pub const SKY: &str = "\x1b[38;5;110m";

// ============================================================================
// NEUTRAL PALETTE - Backgrounds and muted text
// ============================================================================

/// Muted slate gray for secondary text
/// Use for: preformatted text, timestamps, less important info
pub const SLATE: &str = "\x1b[38;5;245m";

/// Light gray for very dim/subtle text
/// Use for: hints, separators, disabled states
pub const MIST: &str = "\x1b[38;5;240m";

/// Bright white for high contrast text
/// Use for: important content on dark backgrounds, h4+ headers
pub const WHITE: &str = "\x1b[38;5;255m";

// ============================================================================
// SEMANTIC COLORS - Status and feedback
// ============================================================================

/// Success/confirmation - soft green
pub const SUCCESS: &str = "\x1b[38;5;114m";

/// Warning/caution - amber
pub const WARNING: &str = "\x1b[38;5;214m";

/// Error/danger - soft red
pub const ERROR: &str = "\x1b[38;5;203m";

// ============================================================================
// BACKGROUND COLORS
// ============================================================================

/// Dark charcoal background for code blocks
pub const BG_CHARCOAL: &str = "\x1b[48;5;236m";

/// Teal background for link markers (with dark text)
pub const BG_TEAL: &str = "\x1b[48;5;37m";

/// Lavender background for condensed mode indicators
pub const BG_LAVENDER: &str = "\x1b[48;5;97m";

// ============================================================================
// COMPOSITE STYLES - Pre-combined for common use cases
// ============================================================================

/// Link marker style: teal background + dark text + bold
pub const LINK_MARKER: &str = "\x1b[48;5;37;38;5;232;1m";

/// Code inline style: charcoal background + teal text
pub const CODE_INLINE: &str = "\x1b[48;5;236;38;5;79m";

/// Condensed mode badge: lavender background + white text + bold
pub const CONDENSED_BADGE: &str = "\x1b[48;5;97;38;5;255;1m";

/// Header h1: bold + teal
pub const H1_STYLE: &str = "\x1b[1;38;5;79m";

/// Header h2: bold + sky blue
pub const H2_STYLE: &str = "\x1b[1;38;5;110m";

/// Header h3: bold + lavender
pub const H3_STYLE: &str = "\x1b[1;38;5;141m";

/// Header h4: bold + white
pub const H4_STYLE: &str = "\x1b[1;38;5;255m";

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Wrap text with a color and reset
#[inline]
pub fn colorize(text: &str, color: &str) -> String {
    format!("{}{}{}", color, text, RESET)
}

/// Wrap text with a style (color + formatting) and reset
#[inline]
pub fn stylize(text: &str, style: &str) -> String {
    format!("{}{}{}", style, text, RESET)
}

/// Create a colored link marker [N]
pub fn link_marker(num: usize) -> String {
    format!("{}[{}]{}", LINK_MARKER, num, RESET)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_colorize() {
        let result = colorize("hello", TEAL);
        assert!(result.starts_with(TEAL));
        assert!(result.ends_with(RESET));
        assert!(result.contains("hello"));
    }

    #[test]
    fn test_link_marker() {
        let marker = link_marker(1);
        assert!(marker.contains("[1]"));
        assert!(marker.ends_with(RESET));
    }
}
