//! Interactive browsing mode for navigating web pages via links.
//!
//! This module provides a polished terminal browsing experience:
//! - Display page content with numbered links
//! - Enter a number to follow that link
//! - Native terminal scrollback for long content
//! - Visual feedback and helpful prompts
//! - Reader mode for article-focused reading

use anyhow::Result;
use console::style;
use std::io::{self, Write};

use crate::browser;
use crate::dom::content::PageContent;
use crate::dom::extract::extract_page_content;
use crate::dom::link_filter::MAX_LINKS;
use crate::dom::multilens::extract_multilens;
use crate::dom::multilens::snapshot::fetch_rendered_html;
use crate::dom::reader::{extract_reader_content, is_probably_readable, ReaderContent};
use crate::render::build_render_lines;
use crate::term::{clear_screen, terminal_width};

/// History entry for back navigation
struct HistoryEntry {
    url: String,
    title: String,
}

/// Cached page state for toggling between modes without re-fetching
#[allow(dead_code)]
struct CachedPage {
    url: String,
    html: String,
    standard_content: Option<PageContent>,
    reader_content: Option<ReaderContent>,
    is_readable: bool,
}

/// Run the interactive browsing session with all options
pub async fn run_interactive_with_options(
    client: &fantoccini::Client,
    initial_url: &str,
    stealth: bool,
    multilens: bool,
    focus: bool,
    reader_mode: bool,
) -> Result<()> {
    let mut history: Vec<HistoryEntry> = Vec::new();
    let mut current_url = initial_url.to_string();
    let mut first_page = true;
    let mut reader_active = reader_mode;
    let mut cached_page: Option<CachedPage> = None;
    
    loop {
        // Check if we need to fetch new content or can use cache
        let need_fetch = cached_page.as_ref().map(|c| c.url != current_url).unwrap_or(true);
        
        if need_fetch {
            // Show loading indicator
            if !first_page {
                print_loading_inline();
            }
            
            // Navigate to current URL with stealth mode
            browser::navigation::navigate_and_wait_with_stealth(client, &current_url, stealth).await?;
            browser::navigation::scroll_to_top_after_load(client).await?;
            
            // Fetch rendered HTML for both modes
            let html = fetch_rendered_html(client).await?;
            
            // Check if page is suitable for reader mode
            let is_readable = is_probably_readable(&html);
            
            // Extract standard content
            let standard_content = if multilens {
                Some(extract_multilens(client, &current_url, focus).await?)
            } else {
                Some(extract_page_content(client, &current_url, focus).await?)
            };
            
            // Extract reader content if readable
            let reader_content = if is_readable {
                extract_reader_content(&html, &current_url)
            } else {
                None
            };
            
            // Cache the page
            cached_page = Some(CachedPage {
                url: current_url.clone(),
                html,
                standard_content,
                reader_content,
                is_readable,
            });
        }
        
        let cache = cached_page.as_ref().unwrap();
        
        // Determine if we should use reader mode
        let use_reader = reader_active && cache.reader_content.is_some();
        
        // Build lines for display based on mode
        let (lines, link_count, page_title) = if use_reader {
            let reader = cache.reader_content.as_ref().unwrap();
            let lines = build_reader_render_lines(reader);
            let count = reader.links.len().min(MAX_LINKS);
            let title = reader.title.clone();
            (lines, count, title)
        } else {
            let content = cache.standard_content.as_ref().unwrap();
            let lines = build_render_lines(content);
            let count = content.links.len().min(MAX_LINKS);
            let title = get_page_title(content);
            (lines, count, title)
        };

        // Clear screen and show header
        clear_screen();
        print_header_with_mode(&current_url, history.len(), use_reader);

        // Display all content - let user scroll with native terminal scrollback
        for line in &lines {
            println!("{}", line);
        }
        
        // Show reader mode hint if page is readable but not in reader mode
        let show_reader_hint = cache.is_readable && !use_reader && first_page;
        
        // Get user input
        match prompt_for_action_with_reader(link_count, history.len(), first_page, show_reader_hint, use_reader)? {
            BrowseAction::FollowLink(num) => {
                let link = if use_reader {
                    cache.reader_content.as_ref().and_then(|r| r.get_link(num))
                } else {
                    cache.standard_content.as_ref().and_then(|c| c.get_link(num))
                };
                
                if let Some(link) = link {
                    // Save current URL to history before navigating
                    history.push(HistoryEntry {
                        url: current_url.clone(),
                        title: page_title.clone(),
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
                cached_page = None; // Force re-fetch
            }
            BrowseAction::Quit => {
                print_goodbye();
                return Ok(());
            }
            BrowseAction::Help => {
                print_help_with_reader();
                wait_for_enter();
            }
            BrowseAction::ShowUrl => {
                print_current_url(&current_url);
                wait_for_enter();
            }
            BrowseAction::GoToUrl(url) => {
                history.push(HistoryEntry {
                    url: current_url.clone(),
                    title: page_title.clone(),
                });
                current_url = url;
            }
            BrowseAction::Home => {
                // Go back to start screen
                return Ok(());
            }
            BrowseAction::ToggleReader => {
                if cache.reader_content.is_some() {
                    reader_active = !reader_active;
                    if reader_active {
                        println!("\n  {} {}", 
                            style("Reader mode").magenta().bold(),
                            style("— showing article content only").dim()
                        );
                    } else {
                        println!("\n  {} {}", 
                            style("Standard mode").cyan().bold(),
                            style("— showing full page").dim()
                        );
                    }
                    std::thread::sleep(std::time::Duration::from_millis(400));
                } else {
                    print_warning("No article content detected - reader mode not available");
                }
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
    ToggleReader,
    Invalid(String),
}

/// Prompt the user for an action (with reader mode support)
fn prompt_for_action_with_reader(
    link_count: usize,
    history_depth: usize,
    show_tip: bool,
    show_reader_hint: bool,
    reader_active: bool,
) -> Result<BrowseAction> {
    println!();
    println!("{}", style("─".repeat(terminal_width())).dim());
    
    // Show reader mode hint if applicable
    if show_reader_hint {
        println!(
            "  {}",
            style("Article detected - press R for reader mode").dim().italic()
        );
    }
    
    // Build a clean, informative prompt line
    let mut prompt_parts: Vec<String> = Vec::new();
    
    // Link navigation hint (most important action)
    if link_count > 0 {
        prompt_parts.push(format!(
            "{}",
            style(format!("1-{}", link_count)).cyan().bold()
        ));
    }
    
    // Back navigation (only if history exists)
    if history_depth > 0 {
        prompt_parts.push(format!("{}", style("b").dim()));
    }
    
    // Standard commands (lowercase)
    prompt_parts.push(format!("{}", style("r").dim()));
    prompt_parts.push(format!("{}", style("h").dim()));
    prompt_parts.push(format!("{}", style("q").dim()));
    
    // Reader mode toggle (uppercase R, separated for clarity)
    if reader_active {
        prompt_parts.push(format!("{}", style("R:on").magenta().bold()));
    } else {
        prompt_parts.push(format!("{}", style("R").dim()));
    }
    
    // First-time tip
    let tip = if show_tip && link_count > 0 {
        format!("  {}", style("type a number to follow a link").dim().italic())
    } else {
        String::new()
    };
    
    let prompt = format!("  {} >{}", prompt_parts.join(" "), tip);
    print!("{} ", prompt);
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();
    
    if input.is_empty() {
        return Ok(BrowseAction::Refresh);
    }
    
    // Check for reader mode toggle first (case-sensitive R)
    if input == "R" || input == "reader" {
        return Ok(BrowseAction::ToggleReader);
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
                if num > 0 && num <= link_count {
                    Ok(BrowseAction::FollowLink(num))
                } else if num == 0 {
                    Ok(BrowseAction::Invalid("Link numbers start at 1".to_string()))
                } else {
                    Ok(BrowseAction::Invalid(format!(
                        "Link #{} not found (only {} links on this page)",
                        num,
                        link_count
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

/// Print page header with URL, navigation context, and reader mode indicator
fn print_header_with_mode(url: &str, history_depth: usize, reader_mode: bool) {
    let width = terminal_width();
    
    let back_indicator = if history_depth > 0 {
        format!(" {} ", style(format!("[{}]", history_depth)).dim())
    } else {
        String::new()
    };
    
    let reader_indicator = if reader_mode {
        format!(" {}", style("[Reader]").magenta().bold())
    } else {
        String::new()
    };
    
    println!();
    println!("{}", style("─".repeat(width)).dim());
    println!(
        "  {}{}{}{}",
        style("CurlUp").cyan().bold(),
        reader_indicator,
        back_indicator,
        style(truncate_url(url, width.saturating_sub(30))).dim()
    );
    println!("{}", style("─".repeat(width)).dim());
    println!();
}

/// Build render lines for reader mode content with article metadata
fn build_reader_render_lines(reader: &ReaderContent) -> Vec<String> {
    use crate::render::text::render_html_to_width;
    
    let width = terminal_width();
    let mut lines = Vec::new();
    
    // Article title
    lines.push(format!(
        "{}{}{}",
        "\x1b[1m\x1b[36m", // Bold cyan
        reader.title,
        "\x1b[0m"
    ));
    
    // Underline for title
    let title_len = reader.title.chars().count().min(width);
    lines.push(format!(
        "{}{}{}",
        "\x1b[36m",
        "═".repeat(title_len),
        "\x1b[0m"
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
            "\x1b[2m", // Dim
            meta_parts.join(" · "),
            "\x1b[0m"
        ));
        lines.push(String::new());
    }
    
    // Separator
    lines.push(format!("{}{}{}", "\x1b[2m", "─".repeat(width.min(60)), "\x1b[0m"));
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

/// Inject link markers into reader HTML
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

/// Colorize link markers in rendered text
fn colorize_reader_link_markers(line: &str) -> String {
    let mut result = line.to_string();
    for num in 1..=MAX_LINKS {
        let placeholder = format!("«{}»", num);
        let colored = format!("\x1b[46;30;1m[{}]\x1b[0m", num);
        result = result.replace(&placeholder, &colored);
    }
    result
}

/// Print help information with reader mode
fn print_help_with_reader() {
    clear_screen();
    
    println!();
    println!("  {}", style("CurlUp Navigation").cyan().bold());
    println!("  {}", style("─".repeat(50)).dim());
    println!();
    
    // Links section - most important
    println!("  {}  Links appear as {} in the text", 
        style("LINKS").white().bold(),
        style("colored numbers").cyan().bold()
    );
    println!("       Type the number and press Enter to follow");
    println!("       Example: {} opens the first link", style("1").cyan().bold());
    println!();
    
    // Reader mode
    println!("  {}", style("READER MODE").white().bold());
    println!("       {}      toggle reader mode (article view)", style("R").magenta());
    println!("       Extracts just the article content for");
    println!("       distraction-free reading. Works on news,");
    println!("       blogs, and article pages.");
    println!();
    
    // Navigation
    println!("  {}", style("NAVIGATE").white().bold());
    println!("       {}      go back to previous page", style("b").cyan());
    println!("       {}      refresh current page", style("r").cyan());
    println!("       {}   return to site picker", style("home").cyan());
    println!();
    
    // Direct URL
    println!("  {}", style("GO TO URL").white().bold());
    println!("       Type any URL directly (e.g., {})", style("google.com").cyan());
    println!("       {}      show current page URL (for copying)", style("u").cyan());
    println!();
    
    // Scrolling
    println!("  {}", style("SCROLL").white().bold());
    println!("       Use your terminal's native scrollback:");
    println!("       Mouse wheel, trackpad, or Shift+PageUp/Down");
    println!();
    
    // Exit
    println!("  {}", style("EXIT").white().bold());
    println!("       {}      quit CurlUp", style("q").cyan());
    println!();
    
    println!("  {}", style("─".repeat(50)).dim());
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
