use console::Term;
use std::io::{self, Write};

/// Clear terminal screen and move cursor to top-left.
pub fn clear_screen() {
    print!("\x1b[2J\x1b[1;1H");
    let _ = io::stdout().flush();
}

/// Get terminal width by querying the terminal directly.
pub fn terminal_width() -> usize {
    let (_, cols) = Term::stdout().size();
    (cols as usize).max(40)
}
