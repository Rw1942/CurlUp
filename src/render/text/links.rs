use crate::dom::content::PageContent;

use super::format_for_terminal_with_url;
use super::util::separator;

pub(super) fn build_render_with_links_lines(content: &PageContent, width: usize) -> Vec<String> {
    let formatted = format_for_terminal_with_url(Some(content.url.as_str()), &content.lines, width);
    let mut lines = Vec::new();

    for line in &formatted {
        lines.push(line.to_string());
    }

    // Show links section if there are any links
    if !content.links.is_empty() {
        lines.push(String::new());
        lines.push(separator(width, '━'));

        let link_count = content.links.len().min(20);
        lines.push(format!(
            "  \x1b[1;36m🔗 {} Links\x1b[0m \x1b[90m(enter a number to follow)\x1b[0m",
            link_count
        ));
        lines.push(String::new());

        // Show links in a clean two-column style if width allows
        let max_links = 20.min(content.links.len());

        for (i, link) in content.links.iter().take(max_links).enumerate() {
            let num = i + 1;
            let available_text_width = width.saturating_sub(8);
            let truncated_text = truncate_text(&link.text, available_text_width.min(60));

            // Color code by link type
            let (color_start, color_end) = get_link_colors(&link.href);

            lines.push(format!(
                "  {}\x1b[1m{:>2}\x1b[0m {}{}{}",
                color_start,
                num,
                color_end,
                truncated_text,
                "\x1b[0m"
            ));
        }

        if content.links.len() > max_links {
            lines.push(String::new());
            lines.push(format!(
                "  \x1b[90m+ {} more links on this page\x1b[0m",
                content.links.len() - max_links
            ));
        }

        lines.push(String::new());
        lines.push(separator(width, '━'));
    } else {
        lines.push(String::new());
        lines.push(separator(width, '─'));
        lines.push("  \x1b[90mNo clickable links found on this page\x1b[0m".to_string());
        lines.push(separator(width, '─'));
    }

    lines
}

/// Get ANSI color codes based on link URL type
fn get_link_colors(url: &str) -> (&'static str, &'static str) {
    if url.contains("github.com") {
        ("\x1b[35m", "\x1b[0m ") // Purple for GitHub
    } else if url.contains("news.ycombinator.com") {
        ("\x1b[33m", "\x1b[0m ") // Orange/yellow for HN
    } else if url.contains("wikipedia.org") {
        ("\x1b[37m", "\x1b[0m ") // White for Wikipedia
    } else {
        ("\x1b[36m", "\x1b[0m ") // Cyan default
    }
}

/// Truncate text to a maximum length with ellipsis
fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len.saturating_sub(3)])
    }
}
