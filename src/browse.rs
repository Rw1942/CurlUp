//! Interactive browsing mode for navigating web pages via links.
//!
//! This module provides a polished terminal browsing experience:
//! - Display page content with numbered links
//! - Enter a number to follow that link
//! - Native terminal scrollback for long content
//! - Visual feedback and helpful prompts

use anyhow::Result;
use console::style;
use std::io::{self, Write};

use crate::browser;
use crate::dom::content::PageContent;
use crate::dom::extract::extract_page_content;
use crate::dom::multilens::extract_multilens;
use crate::render::text::build_render_lines;
use crate::term::{clear_screen, terminal_width};

/// History entry for back navigation
struct HistoryEntry {
    url: String,
    title: String,
}

/// Run the interactive browsing session with all options
pub async fn run_interactive_with_options(
    client: &fantoccini::Client,
    initial_url: &str,
    stealth: bool,
    multilens: bool,
) -> Result<()> {
    let mut history: Vec<HistoryEntry> = Vec::new();
    let mut current_url = initial_url.to_string();
    let mut first_page = true;
    
    loop {
        // Show loading indicator
        if !first_page {
            print_loading_inline();
        }
        
        // Navigate to current URL with stealth mode
        browser::navigation::navigate_and_wait_with_stealth(client, &current_url, stealth).await?;
        browser::navigation::scroll_to_top_after_load(client).await?;
        
        // Extract content with links (use multilens if enabled)
        let content = if multilens {
            extract_multilens(client, &current_url).await?
        } else {
            extract_page_content(client, &current_url).await?
        };
        
        // Build lines for display
        let lines = build_render_lines(&content);
        
        // Clear screen and show header
        clear_screen();
        print_header(&current_url, history.len());
        
        // Display all content - let user scroll with native terminal scrollback
        for line in &lines {
            println!("{}", line);
        }
        
        // Get user input
        match prompt_for_action(&content, history.len(), first_page)? {
            BrowseAction::FollowLink(num) => {
                if let Some(link) = content.get_link(num) {
                    // Save current URL to history before navigating
                    history.push(HistoryEntry {
                        url: current_url.clone(),
                        title: get_page_title(&content),
                    });
                    current_url = link.href.clone();
                    println!(
                        "\n  > Following link to {}",
                        style(truncate_text(&link.text, 40)).white()
                    );
                } else {
                    print_error(&format!("Link #{} doesn't exist", num));
                }
            }
            BrowseAction::Back => {
                if let Some(entry) = history.pop() {
                    println!(
                        "\n  < Going back to {}",
                        style(truncate_text(&entry.title, 40)).white()
                    );
                    current_url = entry.url;
                } else {
                    print_warning("You're at the first page - nowhere to go back");
                }
            }
            BrowseAction::Refresh => {
                println!("\n  Refreshing...");
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
    
    // Build summary line
    let summary = links_summary_line(content, show_tip);
    
    println!();
    println!("{}", style("─".repeat(terminal_width())).dim());
    println!("  {}", summary);
    
    let prompt = format!(
        "  > {}{}{} ",
        style("q:quit h:help r:refresh").dim(),
        back_hint,
        link_hint,
    );

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
        format!(" {} ", style(format!("[{}]", history_depth)).dim())
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
  │   SCROLLING                                         │
  │   ─────────                                         │
  │   Use your terminal's native scrollback:            │
  │   - Scroll wheel / trackpad                         │
  │   - Shift+PageUp / Shift+PageDown                   │
  │                                                     │
  │   NAVIGATION                                        │
  │   ───────────                                       │
  │   [1-20]     Follow a numbered link                 │
  │   b, back    Go back to previous page               │
  │   r          Refresh current page                   │
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
    println!("\n  {}", style(msg).red());
    std::thread::sleep(std::time::Duration::from_millis(1500));
}

/// Print warning message
fn print_warning(msg: &str) {
    println!("\n  {}", style(msg).yellow());
    std::thread::sleep(std::time::Duration::from_millis(1500));
}

/// Print loading indicator inline
fn print_loading_inline() {
    print!("\n  Loading...");
    let _ = io::stdout().flush();
}

/// Print goodbye message
fn print_goodbye() {
    println!("\n  {}\n", style("Goodbye!").dim());
}

/// Wait for user to press Enter
fn wait_for_enter() {
    print!("\n  {} ", style("Press Enter to continue...").dim());
    let _ = io::stdout().flush();
    let mut buf = String::new();
    let _ = io::stdin().read_line(&mut buf);
}

/// Get page title from content (extracts domain from URL)
fn get_page_title(content: &PageContent) -> String {
    // Extract domain from URL as the page title
    content
        .url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .map(|s| truncate_text(s, 30))
        .unwrap_or_else(|| "Unknown".to_string())
}

fn links_summary_line(content: &PageContent, show_tip: bool) -> String {
    let link_count = content.links.len();
    let link_summary = if link_count == 0 {
        "No links".to_string()
    } else {
        let shown = link_count.min(20);
        if link_count > shown {
            format!("{} of {} links inline (1-{})", shown, link_count, shown)
        } else {
            format!("{} links inline (1-{})", link_count, link_count)
        }
    };

    if show_tip {
        format!("{} | Tip: type a number to follow", link_summary)
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
