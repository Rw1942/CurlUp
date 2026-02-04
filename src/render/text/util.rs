//! Terminal utility functions.

use console::Term;

/// Get terminal width by querying the terminal directly.
pub fn terminal_width() -> usize {
    let (_, cols) = Term::stdout().size();
    (cols as usize).max(40)
}
