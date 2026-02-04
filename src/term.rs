use console::Term;
use std::io::{self, Write};

/// Layout constants for the floating UI
pub const HEADER_HEIGHT: usize = 4;    // Header rows (logo, URL, separator, blank)
pub const FOOTER_HEIGHT: usize = 4;    // Footer rows (separator, hints, prompt, blank)

/// Clear terminal screen and move cursor to top-left.
pub fn clear_screen() {
    print!("\x1b[2J\x1b[1;1H");
    let _ = io::stdout().flush();
}

/// Move cursor to a 1-based row/column.
pub fn move_cursor(row: usize, col: usize) {
    print!("\x1b[{};{}H", row, col);
    let _ = io::stdout().flush();
}

/// Clear the current line.
pub fn clear_line() {
    print!("\x1b[2K");
    let _ = io::stdout().flush();
}

/// Clear from cursor to end of line.
pub fn clear_to_eol() {
    print!("\x1b[K");
    let _ = io::stdout().flush();
}

/// Set scroll region (rows are 1-based, inclusive).
/// Content printed within this region will scroll; content outside stays fixed.
#[allow(dead_code)]
pub fn set_scroll_region(top: usize, bottom: usize) {
    print!("\x1b[{};{}r", top, bottom);
    let _ = io::stdout().flush();
}

/// Reset scroll region to full terminal.
pub fn reset_scroll_region() {
    print!("\x1b[r");
    let _ = io::stdout().flush();
}

/// Save cursor position (DEC private).
pub fn save_cursor() {
    print!("\x1b7");
    let _ = io::stdout().flush();
}

/// Restore cursor position (DEC private).
pub fn restore_cursor() {
    print!("\x1b8");
    let _ = io::stdout().flush();
}

/// Hide cursor for cleaner UI.
pub fn hide_cursor() {
    print!("\x1b[?25l");
    let _ = io::stdout().flush();
}

/// Show cursor.
pub fn show_cursor() {
    print!("\x1b[?25h");
    let _ = io::stdout().flush();
}

/// Get terminal width by querying the terminal directly.
pub fn terminal_width() -> usize {
    let (_, cols) = Term::stdout().size();
    (cols as usize).max(40)
}

/// Get terminal height by querying the terminal directly.
pub fn terminal_height() -> usize {
    let (rows, _) = Term::stdout().size();
    (rows as usize).max(10)
}

/// Compute usable content height for the scrollable area.
pub fn content_area_height() -> usize {
    let height = terminal_height();
    height.saturating_sub(HEADER_HEIGHT + FOOTER_HEIGHT).max(5)
}

/// Get the row number where content area starts (1-based).
pub fn content_start_row() -> usize {
    HEADER_HEIGHT + 1
}

/// Get the row number where content area ends (1-based).
pub fn content_end_row() -> usize {
    terminal_height().saturating_sub(FOOTER_HEIGHT)
}

/// Get the row number where footer starts (1-based).
pub fn footer_start_row() -> usize {
    terminal_height().saturating_sub(FOOTER_HEIGHT) + 1
}
