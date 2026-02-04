//! Interactive browsing mode for navigating web pages via links.
//!
//! This module provides a polished terminal browsing experience:
//! - Display page content with numbered links
//! - Enter a number to follow that link
//! - Floating command bar that stays visible while scrolling
//! - Page-based navigation with j/k, arrows, and Page Up/Down
//! - Visual feedback and helpful prompts
//! - Condensed mode for article-focused reading (auto-enabled when available)

use anyhow::Result;
use console::style;
use std::io::{self, Write};

use crate::brand::{self, RESET};
use crate::browser;
use crate::dom::content::PageContent;
use crate::dom::extract::extract_page_content;
use crate::dom::link_filter::MAX_LINKS;
use crate::dom::multilens::extract_multilens;
use crate::dom::multilens::snapshot::fetch_rendered_html;
use crate::dom::reader::{extract_reader_content, is_probably_readable, ReaderContent};
use crate::render::build_render_lines;
use crate::term::terminal_width;

/// History entry for navigation
struct HistoryEntry {
    url: String,
}

/// Cached page state for toggling between modes without re-fetching
struct CachedPage {
    url: String,
    standard_content: Option<PageContent>,
    reader_content: Option<ReaderContent>,
}

/// Navigation state for back/forward
struct NavState {
    back_history: Vec<HistoryEntry>,
    forward_history: Vec<HistoryEntry>,
}

impl NavState {
    fn new() -> Self {
        Self {
            back_history: Vec::new(),
            forward_history: Vec::new(),
        }
    }
    
    fn navigate_to(&mut self, from_url: &str) {
        self.back_history.push(HistoryEntry {
            url: from_url.to_string(),
        });
        self.forward_history.clear();
    }
    
    fn go_back(&mut self, current_url: &str) -> Option<String> {
        if let Some(entry) = self.back_history.pop() {
            self.forward_history.push(HistoryEntry {
                url: current_url.to_string(),
            });
            Some(entry.url)
        } else {
            None
        }
    }
    
    fn go_forward(&mut self, current_url: &str) -> Option<String> {
        if let Some(entry) = self.forward_history.pop() {
            self.back_history.push(HistoryEntry {
                url: current_url.to_string(),
            });
            Some(entry.url)
        } else {
            None
        }
    }
}

/// Run the interactive browsing session with all options
pub async fn run_interactive_with_options(
    client: &fantoccini::Client,
    initial_url: &str,
    stealth: bool,
    multilens: bool,
    focus: bool,
    condensed_enabled: bool,
) -> Result<()> {
    let mut nav = NavState::new();
    let mut current_url = initial_url.to_string();
    // Condensed mode is auto-enabled when available (unless disabled via CLI)
    let mut condensed_active = condensed_enabled;
    let mut cached_page: Option<CachedPage> = None;
    
    loop {
        // Check if we need to fetch new content or can use cache
        let need_fetch = cached_page.as_ref().map(|c| c.url != current_url).unwrap_or(true);
        
        if need_fetch {
            // Show loading indicator
            println!("\n  {} Loading {}...\n", style("⟳").cyan(), truncate_url(&current_url, 50));
            
            // Navigate to current URL with stealth mode
            if let Err(e) = browser::navigation::navigate_and_wait_with_stealth(client, &current_url, stealth).await {
                // Show user-friendly error and let them try again
                println!("\n  {} {}\n", style("✗").red().bold(), style("Failed to load site").red());
                for line in e.to_string().lines() {
                    println!("  {}", line);
                }
                println!();
                
                // Offer options
                print!("  {} Enter new URL or 'q' to quit: ", style(">").cyan().bold());
                io::stdout().flush()?;
                
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                let input = input.trim();
                
                if input.eq_ignore_ascii_case("q") || input.eq_ignore_ascii_case("quit") {
                    return Ok(());
                } else if input.is_empty() {
                    // Go back if we have history, otherwise return to picker
                    if let Some(url) = nav.go_back(&current_url) {
                        current_url = url;
                    } else {
                        return Ok(());
                    }
                } else {
                    current_url = normalize_url(input);
                }
                continue;
            }
            
            if let Err(e) = browser::navigation::scroll_to_top_after_load(client).await {
                // Non-fatal: just log and continue
                eprintln!("  {} Scroll failed: {}", style("!").yellow(), e);
            }
            
            // Fetch rendered HTML for both modes
            let html = fetch_rendered_html(client).await?;
            
            // Check if page is suitable for condensed mode
            let is_readable = is_probably_readable(&html);
            
            // Extract standard content
            let standard_content = if multilens {
                Some(extract_multilens(client, &current_url, focus).await?)
            } else {
                Some(extract_page_content(client, &current_url, focus).await?)
            };
            
            // Extract condensed content if readable
            let reader_content = if is_readable {
                extract_reader_content(&html, &current_url)
            } else {
                None
            };
            
            // Cache the page
            cached_page = Some(CachedPage {
                url: current_url.clone(),
                standard_content,
                reader_content,
            });
        }
        
        let cache = cached_page.as_ref().unwrap();
        
        // Determine if we should use condensed mode
        let use_condensed = condensed_active && cache.reader_content.is_some();
        
        // Build and display content
        let link_count = if use_condensed {
            let reader = cache.reader_content.as_ref().unwrap();
            let lines = build_reader_render_lines(reader);
            let count = reader.links.len().min(MAX_LINKS);
            display_page(&lines, &current_url, count);
            count
        } else {
            let content = cache.standard_content.as_ref().unwrap();
            let lines = build_render_lines(content);
            let count = content.links.len().min(MAX_LINKS);
            display_page(&lines, &current_url, count);
            count
        };
        
        // Get user input
        match prompt_for_action(link_count)? {
            BrowseAction::FollowLink(num) => {
                let link = if use_condensed {
                    cache.reader_content.as_ref().and_then(|r| r.get_link(num))
                } else {
                    cache.standard_content.as_ref().and_then(|c| c.get_link(num))
                };
                
                if let Some(link) = link {
                    nav.navigate_to(&current_url);
                    current_url = link.href.clone();
                } else {
                    println!("  {} Link #{} doesn't exist", style("!").yellow(), num);
                    continue;
                }
            }
            BrowseAction::Back => {
                if let Some(url) = nav.go_back(&current_url) {
                    current_url = url;
                } else {
                    println!("  {} You're at the first page", style("!").yellow());
                    continue;
                }
            }
            BrowseAction::Forward => {
                if let Some(url) = nav.go_forward(&current_url) {
                    current_url = url;
                } else {
                    println!("  {} No forward history", style("!").yellow());
                    continue;
                }
            }
            BrowseAction::Refresh => {
                cached_page = None; // Force re-fetch
            }
            BrowseAction::Quit => {
                println!("\n  {}\n", style("Goodbye!").dim());
                return Ok(());
            }
            BrowseAction::Help => {
                print_help();
                continue;
            }
            BrowseAction::ShowUrl => {
                println!("\n  {} {}\n", style("URL:").white().bold(), style(&current_url).cyan().underlined());
                continue;
            }
            BrowseAction::GoToUrl(url) => {
                nav.navigate_to(&current_url);
                current_url = url;
            }
            BrowseAction::Home => {
                // Return to site picker
                return Ok(());
            }
            BrowseAction::Top => {
                // Re-display page (effectively scrolls to top)
                // Will redisplay on next loop iteration
            }
            BrowseAction::ToggleCondensed => {
                if cache.reader_content.is_some() {
                    condensed_active = !condensed_active;
                    // Will redisplay on next loop iteration
                } else {
                    println!("  {} No article content detected", style("!").yellow());
                    continue;
                }
            }
            BrowseAction::Invalid(input) => {
                println!("  {} Unknown: '{}' (type 'help')", style("!").yellow(), input);
                continue;
            }
        }
    }
}

/// Actions the user can take while browsing
enum BrowseAction {
    FollowLink(usize),
    Back,
    Forward,
    Refresh,
    Quit,
    Help,
    ShowUrl,
    GoToUrl(String),
    Home,
    Top,  // Re-display page (scroll to top)
    ToggleCondensed,
    Invalid(String),
}

// ============================================================================
// UI Functions - Simple print-all approach with native terminal scrollback
// ============================================================================

/// Display a page with all content
fn display_page(lines: &[String], url: &str, link_count: usize) {
    println!("\n--- {} ---\n", url);
    for line in lines {
        println!("{}", line);
    }
    println!("\n--- {} links | b=back q=quit ---\n", link_count);
}

/// Print help information
fn print_help() {
    println!();
    println!("  {}", style("CurlUp Help").cyan().bold());
    println!("  {}", style("─".repeat(50)).dim());
    println!();
    println!("  {}  {}", style("NAVIGATION").white().bold(), "");
    println!("       {}          follow link number", style("1-99").cyan());
    println!("       {}             go back (shows destination)", style("b").cyan());
    println!("       {}             go forward (shows destination)", style("f").cyan());
    println!("       {}             refresh page", style("r").cyan());
    println!("       {}          go to URL directly", style("url").cyan());
    println!();
    println!("  {}  {}", style("SCROLLING").white().bold(), "");
    println!("       {}             scroll to top of page", style("t").cyan());
    println!("       Also use terminal scrollback: mouse wheel,");
    println!("       trackpad, or Shift+PageUp/Down");
    println!();
    println!("  {}  {}", style("MODES").white().bold(), "");
    println!("       {}             toggle reader mode", style("C").magenta());
    println!("                 Extracts article content for");
    println!("                 distraction-free reading");
    println!();
    println!("  {}  {}", style("OTHER").white().bold(), "");
    println!("       {}             show current URL", style("u").cyan());
    println!("       {} or {}      return to site picker", style("h").cyan(), style("0").cyan());
    println!("       {}             quit CurlUp", style("q").cyan());
    println!();
    println!("  {}", style("─".repeat(50)).dim());
}

/// Prompt for user action
fn prompt_for_action(link_count: usize) -> Result<BrowseAction> {
    // Show prompt
    print!("\n  {} ", style(">").cyan().bold());
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();
    
    // Empty input = do nothing (user can scroll with terminal)
    if input.is_empty() {
        return Ok(BrowseAction::Refresh);
    }
    
    // Single character commands
    if input.len() == 1 {
        match input {
            // Condensed mode toggle (case-sensitive C)
            "C" => return Ok(BrowseAction::ToggleCondensed),
            
            // Navigation
            "q" => return Ok(BrowseAction::Quit),
            "b" => return Ok(BrowseAction::Back),
            "f" => return Ok(BrowseAction::Forward),
            "r" => return Ok(BrowseAction::Refresh),
            "h" | "0" => return Ok(BrowseAction::Home),  // h or 0 = home (site picker)
            "t" => return Ok(BrowseAction::Top),         // t = scroll to top
            "u" | "U" => return Ok(BrowseAction::ShowUrl),
            "?" => return Ok(BrowseAction::Help),
            
            _ => {}
        }
    }
    
    // Multi-character commands
    let lower = input.to_lowercase();
    
    match lower.as_str() {
        "quit" | "exit" => return Ok(BrowseAction::Quit),
        "back" => return Ok(BrowseAction::Back),
        "forward" => return Ok(BrowseAction::Forward),
        "refresh" | "reload" => return Ok(BrowseAction::Refresh),
        "help" => return Ok(BrowseAction::Help),
        "url" => return Ok(BrowseAction::ShowUrl),
        "home" | "start" => return Ok(BrowseAction::Home),
        "top" => return Ok(BrowseAction::Top),
        "condensed" | "reader" => return Ok(BrowseAction::ToggleCondensed),
        _ => {}
    }
    
    // Check if it's a URL
    if input.contains('.') && !input.contains(' ') {
        let url = normalize_url(input);
        return Ok(BrowseAction::GoToUrl(url));
    }
    
    // Try to parse as a number (link selection)
    if let Ok(num) = input.parse::<usize>() {
        if num > 0 && num <= link_count {
            return Ok(BrowseAction::FollowLink(num));
        } else {
            return Ok(BrowseAction::Invalid(format!(
                "Link #{} not found ({} links)",
                num, link_count
            )));
        }
    }
    
    Ok(BrowseAction::Invalid(input.to_string()))
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

/// Build render lines for condensed mode content with article metadata
fn build_reader_render_lines(reader: &ReaderContent) -> Vec<String> {
    use crate::render::text::render_html_to_width;
    
    let width = terminal_width();
    let mut lines = Vec::new();
    
    // Article title - using brand colors
    lines.push(format!(
        "{}{}{}{}",
        brand::BOLD,
        brand::TEAL,
        reader.title,
        RESET
    ));
    
    // Underline for title
    let title_len = reader.title.chars().count().min(width);
    lines.push(format!(
        "{}{}{}",
        brand::TEAL,
        "═".repeat(title_len),
        RESET
    ));
    lines.push(String::new());
    
    // Metadata line: byline, site, read time
    let mut meta_parts = Vec::new();
    
    if let Some(ref byline) = reader.byline {
        meta_parts.push(format!("By {}", byline));
    }
    
    if let Some(ref site) = reader.site_name {
        meta_parts.push(site.clone());
    }
    
    meta_parts.push(format!("{} min read", reader.read_time_minutes));
    
    if let Some(ref date) = reader.published_time {
        // Try to format date nicely, or use as-is
        let date_display = if date.len() > 10 { &date[..10] } else { date };
        meta_parts.push(date_display.to_string());
    }
    
    if !meta_parts.is_empty() {
        lines.push(format!(
            "{}{}{}",
            brand::DIM,
            meta_parts.join(" · "),
            RESET
        ));
        lines.push(String::new());
    }
    
    // Separator
    lines.push(format!("{}{}{}", brand::DIM, "─".repeat(width.min(60)), RESET));
    lines.push(String::new());
    
    // Inject link markers into the HTML content
    let html_with_markers = inject_reader_link_markers(&reader.html, &reader.links);
    
    // Render the article content
    let content_lines = render_html_to_width(&html_with_markers, width);
    
    // Colorize link markers
    for line in content_lines {
        let colored = colorize_reader_link_markers(&line);
        lines.push(colored);
    }
    
    lines
}

/// Inject link markers into condensed mode HTML
fn inject_reader_link_markers(html: &str, links: &[crate::dom::content::Link]) -> String {
    let mut result = html.to_string();
    
    for (idx, link) in links.iter().take(MAX_LINKS).enumerate() {
        let num = idx + 1;
        let marker = format!("«{}»", num);
        
        // Find and mark links by their href
        let patterns = [
            format!(r#"href="{}""#, link.href),
            format!(r#"href='{}'"#, link.href),
        ];
        
        for pattern in &patterns {
            if let Some(href_pos) = result.find(pattern) {
                if let Some(close_offset) = result[href_pos..].find("</a>") {
                    let close_pos = href_pos + close_offset;
                    let mut new_result = String::with_capacity(result.len() + marker.len() + 1);
                    new_result.push_str(&result[..close_pos]);
                    new_result.push(' ');
                    new_result.push_str(&marker);
                    new_result.push_str(&result[close_pos..]);
                    result = new_result;
                    break;
                }
            }
        }
    }
    
    result
}

/// Colorize link markers in condensed mode rendered text
fn colorize_reader_link_markers(line: &str) -> String {
    let mut result = line.to_string();
    for num in 1..=MAX_LINKS {
        let placeholder = format!("«{}»", num);
        let colored = brand::link_marker(num);
        result = result.replace(&placeholder, &colored);
    }
    result
}

/// Truncate URL for display
fn truncate_url(url: &str, max_len: usize) -> String {
    if url.len() <= max_len {
        url.to_string()
    } else {
        format!("{}...", &url[..max_len.saturating_sub(3)])
    }
}
