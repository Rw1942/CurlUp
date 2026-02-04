use console::Term;
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

/// Compute usable content height for scrolling output.
pub fn content_area_height(terminal_height: usize) -> usize {
    let header_height = 5;
    let bottom_bar_height = 2;
    terminal_height.saturating_sub(header_height + bottom_bar_height).max(3)
}
