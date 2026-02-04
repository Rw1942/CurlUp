use std::io::{self, Write};

/// Clear terminal screen and move cursor to top-left.
pub fn clear_screen() {
    print!("\x1b[2J\x1b[1;1H");
    let _ = io::stdout().flush();
}

/// Move cursor to a 1-based row/column.
pub fn move_cursor(row: usize, col: usize) {
    print!("\x1b[{};{}H", row, col);
}

/// Clear the current line.
pub fn clear_line() {
    print!("\x1b[2K");
}

/// Get terminal width with a conservative cap.
pub fn terminal_width() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(80)
        .min(100)
}

/// Get terminal height with a conservative cap.
pub fn terminal_height() -> usize {
    std::env::var("LINES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(24)
        .max(10)
        .min(80)
}

/// Compute usable content height for scrolling output.
pub fn content_area_height(terminal_height: usize) -> usize {
    let header_height = 5;
    let bottom_bar_height = 2;
    terminal_height.saturating_sub(header_height + bottom_bar_height).max(3)
}
