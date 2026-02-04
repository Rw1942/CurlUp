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
use crate::term::{
    clear_screen, terminal_width, terminal_height, content_area_height,
    move_cursor, clear_line, clear_to_eol, reset_scroll_region,
    hide_cursor, show_cursor, content_start_row, content_end_row, footer_start_row,
    save_cursor, restore_cursor,
};

/// History entry for navigation
struct HistoryEntry {
    url: String,
    title: String,
}

impl HistoryEntry {
    /// Get a short display name for this entry
    fn display_name(&self) -> String {
        // Prefer title, fall back to domain from URL
        if !self.title.is_empty() && self.title.len() <= 20 {
            self.title.clone()
        } else if !self.title.is_empty() {
            format!("{}...", &self.title[..17])
        } else {
            // Extract domain from URL
            self.url
                .trim_start_matches("https://")
                .trim_start_matches("http://")
                .split('/')
                .next()
                .map(|s| if s.len() > 20 { format!("{}...", &s[..17]) } else { s.to_string() })
                .unwrap_or_else(|| "page".to_string())
        }
    }
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

/// View state for the current page display
struct ViewState {
    scroll_offset: usize,
    total_lines: usize,
}

impl ViewState {
    fn new() -> Self {
        Self {
            scroll_offset: 0,
            total_lines: 0,
        }
    }
    
    fn reset(&mut self) {
        self.scroll_offset = 0;
    }
    
    fn set_total_lines(&mut self, total: usize) {
        self.total_lines = total;
    }
    
    /// Scroll up by n lines, clamping to top
    fn scroll_up(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }
    
    /// Scroll down by n lines, clamping to bottom
    fn scroll_down(&mut self, n: usize) {
        let max_offset = self.max_scroll_offset();
        self.scroll_offset = (self.scroll_offset + n).min(max_offset);
    }
    
    /// Jump to top
    fn go_to_top(&mut self) {
        self.scroll_offset = 0;
    }
    
    /// Jump to bottom
    fn go_to_bottom(&mut self) {
        self.scroll_offset = self.max_scroll_offset();
    }
    
    /// Maximum scroll offset (so last content line is at bottom of viewport)
    fn max_scroll_offset(&self) -> usize {
        let viewport = content_area_height();
        self.total_lines.saturating_sub(viewport)
    }
    
    /// Check if we can scroll down more
    fn can_scroll_down(&self) -> bool {
        self.scroll_offset < self.max_scroll_offset()
    }
    
    /// Check if we can scroll up more
    fn can_scroll_up(&self) -> bool {
        self.scroll_offset > 0
    }
    
    /// Get scroll percentage for display
    fn scroll_percentage(&self) -> usize {
        if self.total_lines == 0 {
            return 100;
        }
        let max = self.max_scroll_offset();
        if max == 0 {
            return 100;
        }
        ((self.scroll_offset as f64 / max as f64) * 100.0).round() as usize
    }
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
    
    /// Get the back destination display name, if any
    fn back_destination(&self) -> Option<String> {
        self.back_history.last().map(|e| e.display_name())
    }
    
    /// Get the forward destination display name, if any
    fn forward_destination(&self) -> Option<String> {
        self.forward_history.last().map(|e| e.display_name())
    }
    
    /// Navigate to a new page (clears forward history)
    fn navigate_to(&mut self, from_url: &str, from_title: &str) {
        self.back_history.push(HistoryEntry {
            url: from_url.to_string(),
            title: from_title.to_string(),
        });
        self.forward_history.clear(); // Clear forward when navigating to new page
    }
    
    /// Go back, returns the URL to navigate to
    fn go_back(&mut self, current_url: &str, current_title: &str) -> Option<String> {
        if let Some(entry) = self.back_history.pop() {
            self.forward_history.push(HistoryEntry {
                url: current_url.to_string(),
                title: current_title.to_string(),
            });
            Some(entry.url)
        } else {
            None
        }
    }
    
    /// Go forward, returns the URL to navigate to
    fn go_forward(&mut self, current_url: &str, current_title: &str) -> Option<String> {
        if let Some(entry) = self.forward_history.pop() {
            self.back_history.push(HistoryEntry {
                url: current_url.to_string(),
                title: current_title.to_string(),
            });
            Some(entry.url)
        } else {
            None
        }
    }
    
    fn can_go_back(&self) -> bool {
        !self.back_history.is_empty()
    }
    
    fn can_go_forward(&self) -> bool {
        !self.forward_history.is_empty()
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
    let mut view_state = ViewState::new();
    let mut needs_redraw = true;
    let mut cached_lines: Vec<String> = Vec::new();
    
    loop {
        // Check if we need to fetch new content or can use cache
        let need_fetch = cached_page.as_ref().map(|c| c.url != current_url).unwrap_or(true);
        
        if need_fetch {
            // Show loading indicator
            show_loading_screen(&current_url);
            
            // Navigate to current URL with stealth mode
            browser::navigation::navigate_and_wait_with_stealth(client, &current_url, stealth).await?;
            browser::navigation::scroll_to_top_after_load(client).await?;
            
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
                html,
                standard_content,
                reader_content,
                is_readable,
            });
            
            // Reset scroll to top for new page
            view_state.reset();
            needs_redraw = true;
        }
        
        let cache = cached_page.as_ref().unwrap();
        
        // Determine if we should use condensed mode
        let use_condensed = condensed_active && cache.reader_content.is_some();
        
        // Build lines for display based on mode (only if needed)
        let (link_count, page_title) = if needs_redraw {
            let (lines, count, title) = if use_condensed {
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
            
            cached_lines = lines;
            view_state.set_total_lines(cached_lines.len());
            (count, title)
        } else {
            // Use cached values
            let count = if use_condensed {
                cache.reader_content.as_ref().map(|r| r.links.len().min(MAX_LINKS)).unwrap_or(0)
            } else {
                cache.standard_content.as_ref().map(|c| c.links.len().min(MAX_LINKS)).unwrap_or(0)
            };
            let title = if use_condensed {
                cache.reader_content.as_ref().map(|r| r.title.clone()).unwrap_or_default()
            } else {
                cache.standard_content.as_ref().map(get_page_title).unwrap_or_default()
            };
            (count, title)
        };

        // Draw the full UI with floating header/footer
        if needs_redraw {
            draw_full_ui(&cached_lines, &view_state, &current_url, &nav, use_condensed, link_count, cache.is_readable);
            needs_redraw = false;
        }
        
        // Get user input with immediate key handling
        match prompt_for_action_floating(link_count, &view_state)? {
            BrowseAction::FollowLink(num) => {
                let link = if use_condensed {
                    cache.reader_content.as_ref().and_then(|r| r.get_link(num))
                } else {
                    cache.standard_content.as_ref().and_then(|c| c.get_link(num))
                };
                
                if let Some(link) = link {
                    nav.navigate_to(&current_url, &page_title);
                    current_url = link.href.clone();
                    needs_redraw = true;
                } else {
                    show_status_message(&format!("Link #{} doesn't exist", num), false);
                }
            }
            BrowseAction::Back => {
                if let Some(url) = nav.go_back(&current_url, &page_title) {
                    current_url = url;
                    needs_redraw = true;
                } else {
                    show_status_message("You're at the first page", false);
                }
            }
            BrowseAction::Forward => {
                if let Some(url) = nav.go_forward(&current_url, &page_title) {
                    current_url = url;
                    needs_redraw = true;
                } else {
                    show_status_message("No forward history", false);
                }
            }
            BrowseAction::Refresh => {
                cached_page = None; // Force re-fetch
                needs_redraw = true;
            }
            BrowseAction::Quit => {
                cleanup_and_exit();
                return Ok(());
            }
            BrowseAction::Help => {
                show_help_overlay();
                needs_redraw = true;
            }
            BrowseAction::ShowUrl => {
                show_url_overlay(&current_url);
                needs_redraw = true;
            }
            BrowseAction::GoToUrl(url) => {
                nav.navigate_to(&current_url, &page_title);
                current_url = url;
                needs_redraw = true;
            }
            BrowseAction::Home => {
                cleanup_and_exit();
                return Ok(());
            }
            BrowseAction::ToggleCondensed => {
                if cache.reader_content.is_some() {
                    condensed_active = !condensed_active;
                    view_state.reset(); // Scroll to top on mode change
                    needs_redraw = true;
                    let msg = if condensed_active {
                        "Switched to condensed mode"
                    } else {
                        "Switched to standard mode"
                    };
                    show_status_message(msg, true);
                } else {
                    show_status_message("No article content detected", false);
                }
            }
            BrowseAction::ScrollUp(n) => {
                if view_state.can_scroll_up() {
                    view_state.scroll_up(n);
                    redraw_content_area(&cached_lines, &view_state);
                    update_footer(link_count, &nav, use_condensed, cache.is_readable, &view_state);
                }
            }
            BrowseAction::ScrollDown(n) => {
                if view_state.can_scroll_down() {
                    view_state.scroll_down(n);
                    redraw_content_area(&cached_lines, &view_state);
                    update_footer(link_count, &nav, use_condensed, cache.is_readable, &view_state);
                }
            }
            BrowseAction::PageUp => {
                let page_size = content_area_height().saturating_sub(2);
                if view_state.can_scroll_up() {
                    view_state.scroll_up(page_size);
                    redraw_content_area(&cached_lines, &view_state);
                    update_footer(link_count, &nav, use_condensed, cache.is_readable, &view_state);
                }
            }
            BrowseAction::PageDown => {
                let page_size = content_area_height().saturating_sub(2);
                if view_state.can_scroll_down() {
                    view_state.scroll_down(page_size);
                    redraw_content_area(&cached_lines, &view_state);
                    update_footer(link_count, &nav, use_condensed, cache.is_readable, &view_state);
                }
            }
            BrowseAction::GoToTop => {
                view_state.go_to_top();
                redraw_content_area(&cached_lines, &view_state);
                update_footer(link_count, &nav, use_condensed, cache.is_readable, &view_state);
            }
            BrowseAction::GoToBottom => {
                view_state.go_to_bottom();
                redraw_content_area(&cached_lines, &view_state);
                update_footer(link_count, &nav, use_condensed, cache.is_readable, &view_state);
            }
            BrowseAction::Invalid(input) => {
                show_status_message(&format!("Unknown: '{}' (h for help)", input), false);
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
    ToggleCondensed,
    ScrollUp(usize),
    ScrollDown(usize),
    PageUp,
    PageDown,
    GoToTop,
    GoToBottom,
    Invalid(String),
}

// ============================================================================
// UI Drawing Functions
// ============================================================================

/// Draw the complete UI with floating header and footer
fn draw_full_ui(
    lines: &[String],
    view_state: &ViewState,
    url: &str,
    nav: &NavState,
    condensed_mode: bool,
    link_count: usize,
    is_readable: bool,
) {
    let width = terminal_width();
    let height = terminal_height();
    
    // Clear screen and reset scroll region
    reset_scroll_region();
    clear_screen();
    hide_cursor();
    
    // Draw header (rows 1-HEADER_HEIGHT)
    draw_header(url, nav, condensed_mode, width);
    
    // Set up scroll region for content
    let content_top = content_start_row();
    let content_bottom = content_end_row();
    
    // Draw content area
    draw_content_area(lines, view_state, content_top, content_bottom, width);
    
    // Draw footer (fixed at bottom)
    draw_footer(link_count, nav, condensed_mode, is_readable, view_state, width, height);
    
    show_cursor();
}

/// Draw the header (stays fixed at top)
fn draw_header(url: &str, nav: &NavState, condensed_mode: bool, width: usize) {
    move_cursor(1, 1);
    
    // Separator line
    println!("{}", style("─".repeat(width)).dim());
    
    // Build navigation indicator showing back/forward counts
    let mut nav_parts = Vec::new();
    if nav.can_go_back() {
        nav_parts.push(format!("←{}", nav.back_history.len()));
    }
    if nav.can_go_forward() {
        nav_parts.push(format!("{}→", nav.forward_history.len()));
    }
    let nav_indicator = if !nav_parts.is_empty() {
        format!(" {} ", style(nav_parts.join(" ")).dim())
    } else {
        String::new()
    };
    
    let mode_indicator = if condensed_mode {
        format!(" {}", style("◆").magenta().bold())
    } else {
        String::new()
    };
    
    println!(
        "  {}{}{}  {}",
        style("CurlUp").cyan().bold(),
        mode_indicator,
        nav_indicator,
        style(truncate_url(url, width.saturating_sub(30))).dim()
    );
    
    // Bottom separator
    println!("{}", style("─".repeat(width)).dim());
    
    // Blank line before content
    println!();
}

/// Draw the content area with current scroll position
fn draw_content_area(lines: &[String], view_state: &ViewState, top_row: usize, bottom_row: usize, _width: usize) {
    let viewport_height = bottom_row - top_row + 1;
    let start_idx = view_state.scroll_offset;
    let end_idx = (start_idx + viewport_height).min(lines.len());
    
    // Move to content area start
    move_cursor(top_row, 1);
    
    // Draw visible lines
    for i in start_idx..end_idx {
        // Truncate line if too wide (preserving ANSI codes as much as possible)
        let line = &lines[i];
        println!("{}", line);
    }
    
    // Fill remaining rows with blank lines
    let lines_drawn = end_idx - start_idx;
    for _ in lines_drawn..viewport_height {
        println!();
    }
}

/// Redraw only the content area (for scroll operations)
fn redraw_content_area(lines: &[String], view_state: &ViewState) {
    let content_top = content_start_row();
    let content_bottom = content_end_row();
    let width = terminal_width();
    
    hide_cursor();
    draw_content_area(lines, view_state, content_top, content_bottom, width);
    show_cursor();
}

/// Draw the footer (stays fixed at bottom)
fn draw_footer(
    link_count: usize,
    nav: &NavState,
    condensed_mode: bool,
    is_readable: bool,
    view_state: &ViewState,
    width: usize,
    _height: usize,
) {
    let footer_row = footer_start_row();
    move_cursor(footer_row, 1);
    
    // Separator
    println!("{}", style("─".repeat(width)).dim());
    
    // Build scroll indicator
    let scroll_info = if view_state.total_lines > content_area_height() {
        let pct = view_state.scroll_percentage();
        let arrows = if view_state.can_scroll_up() && view_state.can_scroll_down() {
            "↑↓"
        } else if view_state.can_scroll_up() {
            "↑ "
        } else if view_state.can_scroll_down() {
            " ↓"
        } else {
            "  "
        };
        format!("{} {}%", style(arrows).cyan(), pct)
    } else {
        String::new()
    };
    
    // Build command hints
    let mut hints: Vec<String> = Vec::new();
    
    if link_count > 0 {
        hints.push(format!("[{}] link", style(format!("1-{}", link_count)).cyan()));
    }
    
    // Back with destination
    if let Some(dest) = nav.back_destination() {
        hints.push(format!("[{}] ← {}", style("b").cyan(), style(dest).dim()));
    }
    
    // Forward with destination
    if let Some(dest) = nav.forward_destination() {
        hints.push(format!("[{}] → {}", style("f").cyan(), style(dest).dim()));
    }
    
    // Scroll hints if content is scrollable
    if view_state.total_lines > content_area_height() {
        hints.push(format!("[{}]", style("j/k").dim()));
    }
    
    hints.push(format!("[{}]", style("h").dim()));
    hints.push(format!("[{}]", style("q").dim()));
    
    // Condensed mode toggle
    if is_readable {
        if condensed_mode {
            hints.push(format!("[{}]", style("C:on").magenta().bold()));
        } else {
            hints.push(format!("[{}]", style("C").dim()));
        }
    }
    
    // Print hints with scroll info on right
    let hints_str = hints.join("  ");
    let padding = width.saturating_sub(visible_width(&hints_str) + visible_width(&scroll_info) + 4);
    print!("  {}{}{}", hints_str, " ".repeat(padding), scroll_info);
    clear_to_eol();
    println!();
    
    // Prompt line
    print!("  {} ", style(">").cyan().bold());
    let _ = io::stdout().flush();
}

/// Update just the footer area (for scroll position updates)
fn update_footer(
    link_count: usize,
    nav: &NavState,
    condensed_mode: bool,
    is_readable: bool,
    view_state: &ViewState,
) {
    let width = terminal_width();
    let height = terminal_height();
    
    save_cursor();
    hide_cursor();
    draw_footer(link_count, nav, condensed_mode, is_readable, view_state, width, height);
    restore_cursor();
    show_cursor();
}

/// Calculate visible width of a string (ignoring ANSI codes)
fn visible_width(s: &str) -> usize {
    // Strip ANSI escape sequences for width calculation
    let mut width = 0;
    let mut in_escape = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c == 'm' {
                in_escape = false;
            }
        } else {
            width += 1;
        }
    }
    width
}

/// Show a loading screen while fetching content
fn show_loading_screen(url: &str) {
    let width = terminal_width();
    let height = terminal_height();
    
    reset_scroll_region();
    clear_screen();
    
    let mid_row = height / 2;
    move_cursor(mid_row - 1, 1);
    
    let loading_text = format!("  {} Loading...", style("⟳").cyan());
    println!("{}", loading_text);
    
    let url_text = format!("  {}", style(truncate_url(url, width - 4)).dim());
    println!("{}", url_text);
    
    let _ = io::stdout().flush();
}

/// Show a brief status message in the footer area
fn show_status_message(msg: &str, success: bool) {
    let footer_row = footer_start_row();
    
    save_cursor();
    hide_cursor();
    move_cursor(footer_row + 1, 1);
    clear_line();
    
    let styled_msg = if success {
        format!("  {} {}", style("✓").green(), style(msg).green())
    } else {
        format!("  {} {}", style("!").yellow(), style(msg).yellow())
    };
    print!("{}", styled_msg);
    clear_to_eol();
    
    let _ = io::stdout().flush();
    restore_cursor();
    show_cursor();
    
    // Brief pause so user can see the message
    std::thread::sleep(std::time::Duration::from_millis(600));
}

/// Show help as an overlay
fn show_help_overlay() {
    reset_scroll_region();
    clear_screen();
    
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
    println!("       {} or {}     scroll up/down one line", style("k").cyan(), style("j").cyan());
    println!("       {} or {}     page up/down", style("e").cyan(), style("d").cyan());
    println!("       {} {}        jump to top/bottom", style("g").cyan(), style("G").cyan());
    println!();
    
    println!("  {}  {}", style("MODES").white().bold(), "");
    println!("       {}             toggle reader mode", style("C").magenta());
    println!("                 Extracts article content for");
    println!("                 distraction-free reading");
    println!();
    
    println!("  {}  {}", style("OTHER").white().bold(), "");
    println!("       {}             show current URL", style("u").cyan());
    println!("       {}          return to start screen", style("home").cyan());
    println!("       {}             quit CurlUp", style("q").cyan());
    println!();
    
    println!("  {}", style("─".repeat(50)).dim());
    print!("  {} ", style("Press Enter to continue...").dim());
    let _ = io::stdout().flush();
    
    let mut buf = String::new();
    let _ = io::stdin().read_line(&mut buf);
}

/// Show current URL as an overlay
fn show_url_overlay(url: &str) {
    reset_scroll_region();
    clear_screen();
    
    println!();
    println!("  {}", style("Current URL").white().bold());
    println!();
    println!("  {}", style(url).cyan().underlined());
    println!();
    println!("  {}", style("(Copy this URL from your terminal)").dim());
    println!();
    
    print!("  {} ", style("Press Enter to continue...").dim());
    let _ = io::stdout().flush();
    
    let mut buf = String::new();
    let _ = io::stdin().read_line(&mut buf);
}

/// Clean up terminal state before exiting
fn cleanup_and_exit() {
    reset_scroll_region();
    show_cursor();
    println!("\n  {}\n", style("Goodbye!").dim());
}

/// Prompt for action with the floating UI
fn prompt_for_action_floating(
    link_count: usize,
    view_state: &ViewState,
) -> Result<BrowseAction> {
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();
    
    // Empty input = scroll down a bit (more intuitive than refresh)
    if input.is_empty() {
        if view_state.can_scroll_down() {
            return Ok(BrowseAction::ScrollDown(3));
        }
        return Ok(BrowseAction::Refresh);
    }
    
    // Single character commands for scrolling (most common)
    if input.len() == 1 {
        match input {
            // Scroll commands (vim-style)
            "j" => return Ok(BrowseAction::ScrollDown(1)),
            "k" => return Ok(BrowseAction::ScrollUp(1)),
            "g" => return Ok(BrowseAction::GoToTop),
            "G" => return Ok(BrowseAction::GoToBottom),
            "d" | " " => return Ok(BrowseAction::PageDown), // page down
            "e" => return Ok(BrowseAction::PageUp), // page up
            
            // Condensed mode toggle (case-sensitive C)
            "C" => return Ok(BrowseAction::ToggleCondensed),
            
            // Navigation
            "q" => return Ok(BrowseAction::Quit),
            "b" => return Ok(BrowseAction::Back),
            "f" => return Ok(BrowseAction::Forward),
            "r" => return Ok(BrowseAction::Refresh),
            "h" | "?" => return Ok(BrowseAction::Help),
            "u" | "U" => return Ok(BrowseAction::ShowUrl),
            
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
        "top" => return Ok(BrowseAction::GoToTop),
        "bottom" | "end" => return Ok(BrowseAction::GoToBottom),
        "condensed" | "reader" => return Ok(BrowseAction::ToggleCondensed),
        "gg" => return Ok(BrowseAction::GoToTop),
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
        } else if num == 0 {
            return Ok(BrowseAction::Invalid("Link numbers start at 1".to_string()));
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
