//! Interactive browsing mode for navigating web pages via links.
//!
//! This module provides a polished terminal browsing experience:
//! - Display page content with numbered links
//! - Enter a number to follow that link
//! - Keyboard shortcuts for navigation
//! - Visual feedback and helpful prompts

use anyhow::Result;
use console::style;
use std::io::{self, Write};

use crate::browser;
use crate::dom::content::PageContent;
use crate::dom::extract::extract_page_content;
use crate::render::text::render_with_links_limited;
use crate::term::{clear_line, clear_screen, content_area_height, move_cursor, terminal_height, terminal_width};

/// History entry for back navigation
struct HistoryEntry {
    url: String,
    title: String,
}

/// Run the interactive browsing session
pub async fn run_interactive(
    client: &fantoccini::Client,
    initial_url: &str,
    raw_mode: bool,
) -> Result<()> {
    let mut history: Vec<HistoryEntry> = Vec::new();
    let mut current_url = initial_url.to_string();
    let mut first_page = true;
    
    loop {
        // Show loading indicator
        if !first_page {
            print_loading_inline();
        }
        
        // Navigate to current URL
        browser::navigation::navigate_and_wait(client, &current_url).await?;
        browser::navigation::scroll_to_top_after_load(client).await?;
        
        // Extract content with links
        let content = extract_page_content(client, &current_url).await?;
        
        // Clear screen and render
        clear_screen();
        print_header(&current_url, history.len());
        
        if raw_mode {
            crate::render::text::render(&content.lines);
            println!();
            print_raw_mode_footer(content.links.len());
            return Ok(());
        }
        
        let height = terminal_height();
        let content_height = content_area_height(height);
        let _truncated = render_with_links_limited(&content, content_height);
        
        // Get user input
        match prompt_for_action(&content, history.len(), height, first_page)? {
            BrowseAction::FollowLink(num) => {
                if let Some(link) = content.get_link(num) {
                    // Save current URL to history before navigating
                    history.push(HistoryEntry {
                        url: current_url.clone(),
                        title: get_page_title(&content),
                    });
                    current_url = link.href.clone();
                    println!(
                        "\n  {} Following link to {}",
                        style("→").green(),
                        style(truncate_text(&link.text, 40)).white()
                    );
                } else {
                    print_error(&format!("Link #{} doesn't exist", num));
                }
            }
            BrowseAction::Back => {
                if let Some(entry) = history.pop() {
                    println!(
                        "\n  {} Going back to {}",
                        style("←").yellow(),
                        style(truncate_text(&entry.title, 40)).white()
                    );
                    current_url = entry.url;
                } else {
                    print_warning("You're at the first page - nowhere to go back");
                }
            }
            BrowseAction::Refresh => {
                println!("\n  {} Refreshing...", style("⟳").cyan());
            }
            BrowseAction::Quit => {
                print_goodbye();
                return Ok(());
            }
            BrowseAction::Help => {
                print_help();
                wait_for_enter();
            }
            BrowseAction::ShowUrl => {
                print_current_url(&current_url);
                wait_for_enter();
            }
            BrowseAction::GoToUrl(url) => {
                history.push(HistoryEntry {
                    url: current_url.clone(),
                    title: get_page_title(&content),
                });
                current_url = url;
            }
            BrowseAction::Home => {
                // Go back to start screen
                return Ok(());
            }
            BrowseAction::Invalid(input) => {
                print_error(&format!("Unknown command: '{}' - press 'h' for help", input));
            }
        }

        first_page = false;
    }
}

/// Actions the user can take while browsing
enum BrowseAction {
    FollowLink(usize),
    Back,
    Refresh,
    Quit,
    Help,
    ShowUrl,
    GoToUrl(String),
    Home,
    Invalid(String),
}

/// Prompt the user for an action
fn prompt_for_action(
    content: &PageContent,
    history_depth: usize,
    terminal_height: usize,
    show_tip: bool,
) -> Result<BrowseAction> {
    // Build prompt with context
    let back_hint = if history_depth > 0 {
        format!(" {}:{}", style("b").dim(), style("back").dim())
    } else {
        String::new()
    };
    
    let link_hint = if !content.links.is_empty() {
        format!(
            " {}",
            style(format!("1-{}:link", content.links.len().min(20))).dim()
        )
    } else {
        String::new()
    };
    
    let summary = links_summary_line(content, show_tip);
    let prompt = format!(
        "  {} {}{}{} ",
        style("→").green().bold(),
        style("q:quit h:help").dim(),
        back_hint,
        link_hint,
    );

    let width = terminal_width();
    let summary_row = terminal_height.saturating_sub(1).max(1);
    let prompt_row = terminal_height.max(1);

    move_cursor(summary_row, 1);
    clear_line();
    print!("  {}", truncate_text(&summary, width.saturating_sub(2)));

    move_cursor(prompt_row, 1);
    clear_line();
    print!("{}", prompt);
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();
    
    if input.is_empty() {
        return Ok(BrowseAction::Refresh);
    }
    
    let lower = input.to_lowercase();
    
    match lower.as_str() {
        "q" | "quit" | "exit" => Ok(BrowseAction::Quit),
        "b" | "back" => Ok(BrowseAction::Back),
        "r" | "refresh" | "reload" => Ok(BrowseAction::Refresh),
        "h" | "help" | "?" => Ok(BrowseAction::Help),
        "u" | "url" => Ok(BrowseAction::ShowUrl),
        "home" | "start" => Ok(BrowseAction::Home),
        _ => {
            // Check if it's a URL
            if input.contains('.') && !input.contains(' ') {
                let url = normalize_url(input);
                return Ok(BrowseAction::GoToUrl(url));
            }
            
            // Try to parse as a number
            if let Ok(num) = input.parse::<usize>() {
                if num > 0 && num <= content.links.len().min(20) {
                    Ok(BrowseAction::FollowLink(num))
                } else if num == 0 {
                    Ok(BrowseAction::Invalid("Link numbers start at 1".to_string()))
                } else {
                    Ok(BrowseAction::Invalid(format!(
                        "Link #{} not found (only {} links on this page)",
                        num,
                        content.links.len().min(20)
                    )))
                }
            } else {
                Ok(BrowseAction::Invalid(input.to_string()))
            }
        }
    }
}

/// Normalize URL (add https:// if needed)
fn normalize_url(url: &str) -> String {
    let url = url.trim();
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else if url.starts_with("localhost") {
        format!("http://{}", url)
    } else {
        format!("https://{}", url)
    }
}

/// Print page header with URL and navigation context
fn print_header(url: &str, history_depth: usize) {
    let width = terminal_width();
    
    let back_indicator = if history_depth > 0 {
        format!(" {} ", style(format!("← {}", history_depth)).dim())
    } else {
        String::new()
    };
    
    println!();
    println!("{}", style("─".repeat(width)).dim());
    println!(
        "  {}{}{}",
        style("CurlUp").cyan().bold(),
        back_indicator,
        style(truncate_url(url, width.saturating_sub(20))).dim()
    );
    println!("{}", style("─".repeat(width)).dim());
    println!();
}

/// Print tips for first-time users
/// Print help information
fn print_help() {
    clear_screen();
    
    let help = r#"
  ╭─────────────────────────────────────────────────────╮
  │                                                     │
  │            CurlUp Navigation Guide                  │
  │                                                     │
  ├─────────────────────────────────────────────────────┤
  │                                                     │
  │   NAVIGATION                                        │
  │   ───────────                                       │
  │   [1-20]     Follow a numbered link                 │
  │   b, back    Go back to previous page               │
  │   r          Refresh current page                   │
  │   [Enter]    Refresh current page                   │
  │                                                     │
  │   QUICK ACTIONS                                     │
  │   ─────────────                                     │
  │   [url]      Go directly to any URL                 │
  │              (e.g., "google.com" or full URL)       │
  │   u          Show current page URL                  │
  │   home       Return to site picker                  │
  │                                                     │
  │   OTHER                                             │
  │   ─────                                             │
  │   h, ?       Show this help                         │
  │   q          Quit CurlUp                            │
  │                                                     │
  ╰─────────────────────────────────────────────────────╯
"#;
    
    println!("{}", style(help).cyan());
}

/// Print raw mode footer
fn print_raw_mode_footer(link_count: usize) {
    println!("{}", style("─".repeat(60)).dim());
    println!(
        "  {} {} links found. {}",
        style("ℹ").cyan(),
        link_count,
        style("Raw mode - run without -r for interactive browsing").dim()
    );
}

/// Print current URL
fn print_current_url(url: &str) {
    println!();
    println!("  {}", style("Current URL:").white().bold());
    println!("  {}", style(url).cyan().underlined());
    println!();
    println!("  {}", style("(You can copy this URL)").dim());
}

/// Print error message
fn print_error(msg: &str) {
    println!("\n  {} {}", style("✗").red(), style(msg).red());
    std::thread::sleep(std::time::Duration::from_millis(1500));
}

/// Print warning message
fn print_warning(msg: &str) {
    println!("\n  {} {}", style("!").yellow(), style(msg).yellow());
    std::thread::sleep(std::time::Duration::from_millis(1500));
}

/// Print loading indicator inline
fn print_loading_inline() {
    print!("\n  {} Loading...", style("⟳").cyan());
    let _ = io::stdout().flush();
}

/// Print goodbye message
fn print_goodbye() {
    println!("\n  {} {}\n", style("👋").dim(), style("Goodbye!").dim());
}

/// Wait for user to press Enter
fn wait_for_enter() {
    print!("\n  {} ", style("Press Enter to continue...").dim());
    let _ = io::stdout().flush();
    let mut buf = String::new();
    let _ = io::stdin().read_line(&mut buf);
}

/// Get page title from content
fn get_page_title(content: &PageContent) -> String {
    content
        .lines
        .first()
        .map(|s| truncate_text(s, 30))
        .unwrap_or_else(|| "Unknown".to_string())
}

fn links_summary_line(content: &PageContent, show_tip: bool) -> String {
    let link_count = content.links.len();
    let link_summary = if link_count == 0 {
        "Links: none".to_string()
    } else {
        let shown = link_count.min(20);
        if link_count > shown {
            format!("Links: {} shown (1-{}), {} more", shown, shown, link_count - shown)
        } else {
            format!("Links: {} (1-{})", shown, shown)
        }
    };

    if show_tip {
        format!("{} | Tip: enter a link number or URL", link_summary)
    } else {
        link_summary
    }
}

/// Truncate URL for display
fn truncate_url(url: &str, max_len: usize) -> String {
    if url.len() <= max_len {
        url.to_string()
    } else {
        format!("{}...", &url[..max_len.saturating_sub(3)])
    }
}

/// Truncate text for display
fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len.saturating_sub(3)])
    }
}
