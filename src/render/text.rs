//! # Terminal Text Rendering Engine
//!
//! This module is the core of CurlUp's value proposition: transforming raw web page text
//! into readable, structured terminal output. It handles:
//!
//! - **Content Detection**: Identifying different types of content (news, weather, lists, etc.)
//! - **Noise Filtering**: Removing UI chrome, icons, and irrelevant text
//! - **Terminal Formatting**: Proper line wrapping, indentation, and visual hierarchy
//! - **Preformatted Preservation**: Keeping ASCII art and tables intact
//!
//! ## Architecture
//!
//! The rendering pipeline follows these stages:
//!
//! ```text
//! Raw Lines → Filter → Parse → Render
//!     │          │        │       │
//!     │          │        │       └─► Terminal output with formatting
//!     │          │        └─► ContentBlock enum (structured data)
//!     │          └─► Remove noise, icons, UI elements
//!     └─► Vec<String> from DOM extraction
//! ```
//!
//! ## Extending with Site-Specific Parsers
//!
//! The [`ContentParser`] trait defines how content is parsed. To add a new site:
//!
//! 1. Implement [`ContentParser`] for your site
//! 2. Add URL detection logic to determine which parser to use
//! 3. Register the parser in the parsing pipeline
//!
//! ## Content Block Types
//!
//! See [`ContentBlock`] for all recognized content patterns:
//! - News items with source, headline, timestamp
//! - Numbered lists (Hacker News style)
//! - Weather data with forecasts
//! - Stock tickers with price changes
//! - Section headers for visual organization
//! - Navigation menus
//! - Preformatted/ASCII art content

use std::env;

use crate::dom::links::PageContent;

// ============================================================================
// PUBLIC API
// ============================================================================

/// Render text in raw mode (minimal formatting, preserves all content).
///
/// Use this for scripting, piping to other tools, or when users want
/// unprocessed output. Still applies basic line wrapping for terminal width.
pub fn render(lines: &[String]) {
    let width = terminal_width();
    for line in lines {
        if !line.is_empty() {
            for wrapped in wrap_text(line, width, 0) {
                println!("{}", wrapped);
            }
            println!();
        }
    }
}

/// Render text in formatted mode (smart parsing, visual hierarchy).
///
/// This is the default mode that makes CurlUp useful. It:
/// - Detects and formats different content types
/// - Removes noise and UI elements
/// - Creates visual sections with separators
/// - Wraps text appropriately for the terminal
pub fn render_condensed(lines: &[String]) {
    let width = terminal_width();
    let formatted = format_for_terminal_with_url(None, lines, width);
    for line in formatted {
        println!("{}", line);
    }
}

/// Render page content with links, limited to a maximum number of lines.
///
/// Returns true if output was truncated.
pub fn render_with_links_limited(content: &PageContent, max_lines: usize) -> bool {
    if max_lines == 0 {
        return true;
    }

    let width = terminal_width();
    let lines = build_render_with_links_lines(content, width);

    if lines.len() <= max_lines {
        for line in lines {
            println!("{}", line);
        }
        return false;
    }

    let visible_lines = max_lines.saturating_sub(1).max(1);
    for line in lines.iter().take(visible_lines) {
        println!("{}", line);
    }
    println!("  ... more content below");
    true
}

fn build_render_with_links_lines(content: &PageContent, width: usize) -> Vec<String> {
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
        ("\x1b[35m", "\x1b[0m ")  // Purple for GitHub
    } else if url.contains("news.ycombinator.com") {
        ("\x1b[33m", "\x1b[0m ")  // Orange/yellow for HN
    } else if url.contains("wikipedia.org") {
        ("\x1b[37m", "\x1b[0m ")  // White for Wikipedia
    } else {
        ("\x1b[36m", "\x1b[0m ")  // Cyan default
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

// ============================================================================
// CONTENT TYPES
// ============================================================================

/// Structured content blocks recognized during parsing.
///
/// Each variant represents a distinct type of content with its own
/// rendering rules. This enum is the intermediate representation
/// between raw text and formatted output.
///
/// # Adding New Content Types
///
/// To add a new content type:
/// 1. Add a variant to this enum
/// 2. Add parsing logic in [`parse_content`] or a site-specific parser
/// 3. Add rendering logic in [`render_parsed_content`]
#[derive(Debug, Clone)]
pub enum ContentBlock {
    /// Weather information (Google Weather, wttr.in, etc.)
    ///
    /// # Fields
    /// - `location`: City/region name
    /// - `temp`: Current temperature with unit
    /// - `forecast`: Vec of (day, high, low) tuples
    Weather {
        location: String,
        temp: String,
        forecast: Vec<(String, String, String)>,
    },

    /// Section header for visual organization.
    ///
    /// Rendered with emphasis (e.g., "━━ TOP STORIES ━━")
    SectionHeader(String),

    /// News article or story from a news source.
    ///
    /// # Fields
    /// - `source`: Publication name (e.g., "CNN", "BBC")
    /// - `headline`: Article title
    /// - `timestamp`: Optional relative time (e.g., "2 hours ago")
    NewsItem {
        source: String,
        headline: String,
        timestamp: Option<String>,
    },

    /// Financial ticker with price and change.
    ///
    /// # Fields
    /// - `symbol`: Index/stock name (e.g., "Dow Jones", "S&P 500")
    /// - `value`: Current price/value
    /// - `change`: Percentage change (with + or -)
    StockTicker {
        symbol: String,
        value: String,
        change: String,
    },

    /// Numbered list item (e.g., Hacker News posts).
    ///
    /// # Fields
    /// - `number`: The list number as string
    /// - `text`: The item content
    /// - `metadata`: Optional info like "239 points · 4 hours ago · 90 comments"
    NumberedItem {
        number: String,
        text: String,
        metadata: Option<String>,
    },

    /// Plain text content (fallback for unrecognized patterns).
    PlainText(String),

    /// Navigation menu items.
    ///
    /// Rendered compactly with separators (e.g., "Home | About | Contact")
    Navigation(Vec<String>),

    /// Preformatted content that should not be wrapped.
    ///
    /// Used for ASCII art, tables, code blocks, etc.
    Preformatted(String),

    /// Article with full metadata (tech blogs, news sites).
    ///
    /// # Fields
    /// - `headline`: Article title
    /// - `author`: Author name (optional)
    /// - `timestamp`: Publication time (optional)  
    /// - `read_time`: Estimated read time like "5 MIN READ" (optional)
    /// - `description`: Article summary/subtitle (optional)
    Article {
        headline: String,
        author: Option<String>,
        timestamp: Option<String>,
        read_time: Option<String>,
        description: Option<String>,
    },

    /// Sports score or game info.
    ///
    /// # Fields
    /// - `teams`: Team names/matchup
    /// - `score`: Current or final score
    /// - `status`: Game status (live, final, scheduled time)
    SportsScore {
        teams: String,
        score: Option<String>,
        status: Option<String>,
    },
}

// ============================================================================
// PARSER TRAIT (for future site-specific parsers)
// ============================================================================

/// Trait for site-specific content parsers.
///
/// Implement this trait to add specialized parsing for specific websites.
/// The generic parser handles most sites, but some (like Hacker News, Reddit)
/// benefit from custom parsing logic.
///
/// # Example Implementation
///
/// ```ignore
/// struct HackerNewsParser;
///
/// impl ContentParser for HackerNewsParser {
///     fn can_parse(&self, url: &str) -> bool {
///         url.contains("news.ycombinator.com")
///     }
///
///     fn parse(&self, lines: &[&str]) -> Vec<ContentBlock> {
///         // HN-specific parsing logic
///     }
/// }
/// ```
#[allow(dead_code)]
pub trait ContentParser {
    /// Returns true if this parser should handle the given URL.
    fn can_parse(&self, url: &str) -> bool;

    /// Parse raw lines into structured content blocks.
    fn parse(&self, lines: &[&str]) -> Vec<ContentBlock>;

    /// Optional: Return patterns this parser recognizes (for debugging/docs).
    fn patterns(&self) -> Vec<&'static str> {
        vec![]
    }
}

// ============================================================================
// SITE-SPECIFIC PARSERS
// ============================================================================

struct HackerNewsParser;
struct GoogleNewsParser;
struct GitHubTrendingParser;
struct RedditProgrammingParser;
struct DevCommunityParser;
struct NprParser;
struct WikipediaParser;
struct WttrParser;
struct ImdbTopParser;
struct ReutersParser;

impl ContentParser for HackerNewsParser {
    fn can_parse(&self, url: &str) -> bool {
        url.contains("news.ycombinator.com")
    }

    fn parse(&self, lines: &[&str]) -> Vec<ContentBlock> {
        parse_hacker_news(lines)
    }
}

impl ContentParser for GoogleNewsParser {
    fn can_parse(&self, url: &str) -> bool {
        url.contains("news.google.com")
    }

    fn parse(&self, lines: &[&str]) -> Vec<ContentBlock> {
        parse_with_site_noise(
            lines,
            &[
                "top stories", "for you", "local", "world", "business", "technology",
                "entertainment", "sports", "science", "health", "menu", "search", "sign in",
            ],
        )
    }
}

impl ContentParser for GitHubTrendingParser {
    fn can_parse(&self, url: &str) -> bool {
        url.contains("github.com/trending")
    }

    fn parse(&self, lines: &[&str]) -> Vec<ContentBlock> {
        parse_with_site_noise(
            lines,
            &[
                "trending", "developers", "repositories", "spoken language", "language",
                "stars", "forks", "built by", "today", "this week", "this month",
            ],
        )
    }
}

impl ContentParser for RedditProgrammingParser {
    fn can_parse(&self, url: &str) -> bool {
        url.contains("reddit.com/r/programming")
    }

    fn parse(&self, lines: &[&str]) -> Vec<ContentBlock> {
        parse_with_site_noise(
            lines,
            &[
                "log in", "sign up", "open in app", "join", "create post",
                "sort by", "best", "hot", "new", "top", "rising", "r/programming",
            ],
        )
    }
}

impl ContentParser for DevCommunityParser {
    fn can_parse(&self, url: &str) -> bool {
        url.contains("dev.to")
    }

    fn parse(&self, lines: &[&str]) -> Vec<ContentBlock> {
        parse_with_site_noise(
            lines,
            &[
                "dev community", "create account", "sign in", "search",
                "reading list", "home",
            ],
        )
    }
}

impl ContentParser for NprParser {
    fn can_parse(&self, url: &str) -> bool {
        url.contains("npr.org")
    }

    fn parse(&self, lines: &[&str]) -> Vec<ContentBlock> {
        parse_with_site_noise(lines, &["listen", "donate", "sign in", "sections", "menu"])
    }
}

impl ContentParser for WikipediaParser {
    fn can_parse(&self, url: &str) -> bool {
        url.contains("wikipedia.org")
    }

    fn parse(&self, lines: &[&str]) -> Vec<ContentBlock> {
        parse_with_site_noise(
            lines,
            &[
                "jump to navigation", "jump to search", "contents", "from wikipedia",
                "references", "external links", "edit", "tools",
            ],
        )
    }
}

impl ContentParser for WttrParser {
    fn can_parse(&self, url: &str) -> bool {
        url.contains("wttr.in")
    }

    fn parse(&self, lines: &[&str]) -> Vec<ContentBlock> {
        parse_content_filtered(lines)
    }
}

impl ContentParser for ImdbTopParser {
    fn can_parse(&self, url: &str) -> bool {
        url.contains("imdb.com/chart/top")
    }

    fn parse(&self, lines: &[&str]) -> Vec<ContentBlock> {
        parse_with_site_noise(
            lines,
            &[
                "imdb charts", "top 250 movies", "sort by", "rating", "year", "back to top",
            ],
        )
    }
}

impl ContentParser for ReutersParser {
    fn can_parse(&self, url: &str) -> bool {
        url.contains("reuters.com")
    }

    fn parse(&self, lines: &[&str]) -> Vec<ContentBlock> {
        parse_with_site_noise(
            lines,
            &[
                "reuters", "latest", "world", "business", "markets", "sign in", "search",
            ],
        )
    }
}

fn select_site_parser(url: &str) -> Option<Box<dyn ContentParser>> {
    let parsers: Vec<Box<dyn ContentParser>> = vec![
        Box::new(HackerNewsParser),
        Box::new(GoogleNewsParser),
        Box::new(GitHubTrendingParser),
        Box::new(RedditProgrammingParser),
        Box::new(DevCommunityParser),
        Box::new(NprParser),
        Box::new(WikipediaParser),
        Box::new(WttrParser),
        Box::new(ImdbTopParser),
        Box::new(ReutersParser),
    ];

    for parser in parsers {
        if parser.can_parse(url) {
            return Some(parser);
        }
    }

    None
}

fn parse_with_site_noise(lines: &[&str], noise: &[&str]) -> Vec<ContentBlock> {
    let filtered: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|line| !contains_any_ci(line, noise))
        .collect();
    parse_content_filtered(&filtered)
}

fn contains_any_ci(s: &str, patterns: &[&str]) -> bool {
    let lower = s.to_lowercase();
    patterns.iter().any(|p| lower.contains(&p.to_lowercase()))
}

fn parse_hacker_news(lines: &[&str]) -> Vec<ContentBlock> {
    let mut blocks = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let current = lines[i];

        if is_pipe_nav(current) || contains_any_ci(current, &["hacker news", "guidelines", "faq"]) {
            i += 1;
            continue;
        }

        if let Some((item, consumed)) = try_parse_hn_item(&lines[i..]) {
            blocks.push(item);
            i += consumed;
            continue;
        }

        if let Some((item, consumed)) = try_parse_numbered_item(&lines[i..]) {
            blocks.push(item);
            i += consumed;
            continue;
        }

        if is_section_header(current) {
            blocks.push(ContentBlock::SectionHeader(current.to_string()));
            i += 1;
            continue;
        }

        if is_nav_item(current) {
            i += 1;
            continue;
        }

        blocks.push(ContentBlock::PlainText(current.to_string()));
        i += 1;
    }

    blocks
}

fn try_parse_hn_item(lines: &[&str]) -> Option<(ContentBlock, usize)> {
    if lines.is_empty() {
        return None;
    }

    let (num, rest) = parse_number_prefix(lines[0])?;

    if num.parse::<u32>().unwrap_or(0) < 1 {
        return None;
    }

    let mut consumed = 1;
    let mut title = rest.trim().to_string();

    if title.is_empty() {
        if lines.len() < 2 {
            return None;
        }
        let next = lines[1];
        if is_metadata_line(next) {
            return None;
        }
        title = next.to_string();
        consumed += 1;
    }

    if lines.len() > consumed {
        let domain_line = lines[consumed];
        if is_domain_line(domain_line) {
            title.push(' ');
            title.push_str(domain_line);
            consumed += 1;
        }
    }

    let (metadata, meta_consumed) = collect_metadata(lines, consumed, 2);
    if meta_consumed > 0 {
        consumed += meta_consumed;
    }

    Some((
        ContentBlock::NumberedItem {
            number: num,
            text: title,
            metadata,
        },
        consumed,
    ))
}

fn parse_number_prefix(line: &str) -> Option<(String, String)> {
    let mut chars = line.chars().peekable();
    let mut num = String::new();

    while chars.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        num.push(chars.next().unwrap());
    }

    if num.is_empty() {
        return None;
    }

    if chars.next() != Some('.') {
        return None;
    }

    let rest: String = chars.collect();
    Some((num, rest))
}

// ============================================================================
// TERMINAL UTILITIES
// ============================================================================

/// Get the current terminal width.
///
/// Reads from the `COLUMNS` environment variable, defaulting to 80 columns
/// if not set. This is the standard way to detect terminal width in Unix.
///
/// # Returns
/// Terminal width in characters (minimum 40, default 80)
fn terminal_width() -> usize {
    env::var("COLUMNS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(80)
        .max(40) // Minimum usable width
}

/// Check if text appears to be ASCII art or preformatted content.
///
/// Preformatted content should not be wrapped as it would break the visual
/// structure. We detect this by looking for:
///
/// - Box drawing characters (│ ┌ ┐ └ ┘ ├ ┤ ─ etc.)
/// - ASCII art patterns (.-. \_( etc.)
/// - High ratio of special characters (>40%)
///
/// # Arguments
/// * `text` - The text line to check
///
/// # Returns
/// `true` if the text should be preserved without wrapping
fn is_preformatted(text: &str) -> bool {
    // Box drawing characters (Unicode)
    let box_chars = ['│', '┌', '┐', '└', '┘', '├', '┤', '┬', '┴', '┼', '─', '═', '║'];
    let has_box_chars = text.chars().any(|c| box_chars.contains(&c));

    // Common ASCII art patterns
    let ascii_patterns = [".-.", ".--.", "(_", ".--(", "/ \\", ".-'", "`-'", "---", "===", "___)"];
    let has_ascii_pattern = ascii_patterns.iter().any(|p| text.contains(p));

    // High ratio of special characters suggests preformatted content
    let special_count = text
        .chars()
        .filter(|c| !c.is_alphanumeric() && !c.is_whitespace())
        .count();
    let high_special_ratio = text.len() > 10 && special_count as f32 / text.len() as f32 > 0.4;

    has_box_chars || has_ascii_pattern || high_special_ratio
}

/// Wrap text to fit within the specified width, preserving word boundaries.
///
/// Handles continuation lines with proper indentation. Does not wrap
/// preformatted content (ASCII art, tables, etc.).
///
/// # Arguments
/// * `text` - The text to wrap
/// * `width` - Maximum line width in characters
/// * `indent` - Number of spaces to indent continuation lines
///
/// # Returns
/// Vector of wrapped lines
fn wrap_text(text: &str, width: usize, indent: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }

    // Don't wrap preformatted/ASCII art content
    if is_preformatted(text) {
        return vec![text.to_string()];
    }

    let indent_str = " ".repeat(indent);
    let effective_width = width.saturating_sub(indent);

    // Minimum width for wrapping to make sense
    if effective_width < 20 {
        return vec![format!("{}{}", indent_str, text)];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() <= effective_width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            // Line is full, start a new one
            lines.push(format!(
                "{}{}",
                if lines.is_empty() { "" } else { &indent_str },
                current_line
            ));
            current_line = word.to_string();
        }
    }

    // Don't forget the last line
    if !current_line.is_empty() {
        lines.push(format!(
            "{}{}",
            if lines.is_empty() { "" } else { &indent_str },
            current_line
        ));
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

/// Create a horizontal separator line.
///
/// # Arguments
/// * `width` - Desired width (capped at 80)
/// * `ch` - Character to repeat
fn separator(width: usize, ch: char) -> String {
    ch.to_string().repeat(width.min(80))
}

// ============================================================================
// NOISE FILTERING
// ============================================================================

/// Phrases that should be completely filtered out.
///
/// These are common UI elements, prompts, and irrelevant text that appear
/// on many websites but don't contribute to the actual content.
const NOISE_PHRASES: &[&str] = &[
    // Call-to-action prompts
    "More",
    "See more",
    "See more headlines & perspectives",
    "See more updates ›",
    "Sign in",
    "Sign in for personalized stories in your briefing & news feed",
    "Sign in to get news based on your interests",
    "Sign in to get stories based on your interests",
    "More news",
    "Subscribe",
    "Watch",
    // Navigation/UI
    "Customize",
    "Your topics",
    "Manage local news",
    "Disclaimer",
    "Opinion",
    "ADVERTISEMENT",
    "SKIP ADVERTISEMENT",
    "Skip to content",
    "SKIP TO CONTENT",
    "SKIP TO MAIN CONTENT",
    "SKIP TO SITE INDEX",
    // Live/video indicators that appear alone
    "LIVE",
    "Live",
    "AD",
    // Grid/layout
    "GRID",
    "GRID SETTINGS",
    "FEATURED",
    "THEME",
    "SIGN IN",
    "FORUM",
    "SECTIONS",
    // Cloudflare
    "Performing security verification",
    "Performance and Security by Cloudflare",
    "Privacy",
];

/// Material Design icon names that appear as text in web pages.
///
/// Google's Material Design uses ligatures that render as icons in browsers
/// but appear as text when extracted. These should be removed.
const MATERIAL_ICONS: &[&str] = &[
    "chevron_right",
    "chevron_left",
    "expand_more",
    "expand_less",
    "arrow_forward",
    "arrow_back",
    "close",
    "menu",
    "search",
    "share",
    "bookmark",
    "bookmark_border",
    "notifications",
    "settings",
    "account_circle",
    "more_vert",
    "more_horiz",
];

/// Filter and clean raw lines before parsing.
///
/// This function:
/// 1. Removes Material icon ligature text
/// 2. Filters out noise phrases
/// 3. Removes Cloudflare/security text
/// 4. Normalizes whitespace
///
/// # Arguments
/// * `lines` - Raw lines from DOM extraction
///
/// # Returns
/// Filtered and cleaned lines ready for parsing
fn filter_noise(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .map(|s| {
            let mut cleaned = s.trim().to_string();
            // Remove icon names from text
            for icon in MATERIAL_ICONS {
                cleaned = cleaned.replace(icon, "");
            }
            // Normalize whitespace
            cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
        })
        .filter(|s| {
            if s.is_empty() {
                return false;
            }
            // Exact match noise
            if NOISE_PHRASES.iter().any(|&n| *s == n) {
                return false;
            }
            // Cloudflare/security patterns
            if s.starts_with("Ray ID:") || s.contains("security service") {
                return false;
            }
            // Image credits that aren't useful
            if s.ends_with("/Getty Images") || s.ends_with("/Reuters") || s.ends_with("/AP") {
                return false;
            }
            // Standalone short bracketed items like "[1]", "[2]" 
            if s.starts_with('[') && s.ends_with(']') && s.len() <= 5 {
                return false;
            }
            // "Built by" lines (GitHub)
            if s.starts_with("Built by") {
                return false;
            }
            // Pipe-separated navigation bars (HN, news sites, etc.)
            if is_pipe_nav(s) {
                return false;
            }
            // Generic auth/menu prompts
            if matches_any_ci(s, &["sign in", "sign up", "log in", "menu", "search"]) {
                return false;
            }
            // Empty-ish lines with just symbols
            if s.chars().all(|c| !c.is_alphanumeric()) {
                return false;
            }
            true
        })
        .collect()
}

/// Group related lines into more stable units before parsing.
fn group_lines(lines: &[String]) -> Vec<String> {
    let mut grouped: Vec<String> = Vec::new();

    for line in lines {
        let trimmed = line.trim();

        if is_domain_line(trimmed) {
            if let Some(last) = grouped.last_mut() {
                if !is_metadata_line(last) && !is_section_header(last) {
                    last.push(' ');
                    last.push_str(trimmed);
                    continue;
                }
            }
        }

        grouped.push(trimmed.to_string());
    }

    grouped
}

/// Check for simple domain lines like "(example.com)".
fn is_domain_line(s: &str) -> bool {
    s.starts_with('(')
        && s.ends_with(')')
        && s.contains('.')
        && !s.contains(' ')
        && s.len() <= 80
}

/// Check if a line looks like metadata (points/comments/ago/read time).
fn is_metadata_line(s: &str) -> bool {
    is_timestamp(s)
        || is_read_time(s)
        || s.contains("points")
        || s.contains("comments")
        || s.contains("comment")
        || s.contains("ago")
        || s.contains("mins")
        || s.contains("hours")
        || s.contains("posted")
        || s.contains("by ")
}

/// Detect pipe-separated navigation bars.
fn is_pipe_nav(s: &str) -> bool {
    if !s.contains('|') {
        return false;
    }
    let parts: Vec<&str> = s.split('|').map(|p| p.trim()).filter(|p| !p.is_empty()).collect();
    if parts.len() < 3 {
        return false;
    }
    parts
        .iter()
        .all(|p| p.len() <= 18 && p.split_whitespace().count() <= 3)
}

/// Case-insensitive exact match helper.
fn matches_any_ci(s: &str, patterns: &[&str]) -> bool {
    let lower = s.to_lowercase();
    patterns.iter().any(|p| lower == p.to_lowercase())
}

// ============================================================================
// CONTENT DETECTION PATTERNS
// ============================================================================

/// Known news sources for reliable detection.
///
/// When we see these exact strings, we can confidently identify them as
/// news sources rather than other content types.
const KNOWN_NEWS_SOURCES: &[&str] = &[
    // Major US news
    "ABC News",
    "NBC News",
    "CBS News",
    "Fox News",
    "CNN",
    "MSNBC",
    "NPR",
    "PBS",
    // Major newspapers
    "The New York Times",
    "Washington Post",
    "Wall Street Journal",
    "WSJ",
    "USA Today",
    "Los Angeles Times",
    // Wire services
    "Reuters",
    "AP News",
    "Associated Press",
    "AFP",
    // UK/International
    "BBC",
    "The Guardian",
    "The Telegraph",
    "Sky News",
    // Business
    "Bloomberg",
    "CNBC",
    "Forbes",
    "Financial Times",
    // Politics
    "Politico",
    "The Hill",
    "Axios",
    // Tech
    "TechCrunch",
    "The Verge",
    "Ars Technica",
    "Wired",
    // Regional (add more as needed)
    "9News",
    "Colorado Hometown Weekly",
    "Denver Post",
];

/// Section headers that indicate content organization.
///
/// These create visual sections in the output.
const SECTION_HEADERS: &[&str] = &[
    "Top stories",
    "U.S.",
    "World",
    "Business",
    "Technology",
    "Entertainment",
    "Sports",
    "Science",
    "Health",
    "Local news",
    "Beyond the front page",
    "Picks for you",
    "Headlines",
    "Latest News",
    "Breaking News",
];

/// Stock market index/ticker names.
const STOCK_SYMBOLS: &[&str] = &[
    "Dow Jones",
    "S&P 500",
    "Nasdaq",
    "Russell",
    "NYSE",
    "FTSE",
    "DAX",
    "Nikkei",
];

/// Day names for weather forecast detection.
const DAY_NAMES: &[&str] = &[
    "Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sunday", "Monday", "Tuesday", "Wednesday",
    "Thursday", "Friday", "Saturday", "Today", "Tomorrow",
];

// ============================================================================
// CONTENT DETECTION FUNCTIONS
// ============================================================================

/// Check if a string looks like a temperature value.
///
/// Matches patterns like: "72°", "72°F", "22°C", "72 °F"
fn is_temperature(s: &str) -> bool {
    s.ends_with('°')
        || s.ends_with("°F")
        || s.ends_with("°C")
        || (s.len() <= 6
            && s.chars().filter(|c| c.is_ascii_digit()).count() >= 1
            && s.contains('°'))
}

/// Check if a string is a day name (for weather forecasts).
fn is_day_name(s: &str) -> bool {
    DAY_NAMES.iter().any(|&d| s == d)
}

/// Check if a string is a section header.
fn is_section_header(s: &str) -> bool {
    SECTION_HEADERS.iter().any(|&h| s == h)
}

/// Check if a string looks like a navigation item.
///
/// Navigation items are typically short (1-2 words) without periods.
fn is_nav_item(s: &str) -> bool {
    s.len() < 20 && s.split_whitespace().count() <= 2 && !s.contains('.')
}

/// Check if a string looks like a news source or category name.
///
/// This is more permissive than exact matching to catch sources we don't
/// have in our known list.
fn is_source_or_category(s: &str) -> bool {
    let word_count = s.split_whitespace().count();

    // Must be short (1-4 words, under 40 chars)
    if word_count > 4 || s.len() >= 40 {
        return false;
    }

    // Not a timestamp
    if is_timestamp(s) {
        return false;
    }

    // Not a numbered list item (1. 2. 3.)
    if s.chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
        && s.contains('.')
    {
        return false;
    }

    // Not a Wikipedia-style bracketed item like [Rust] or [edit]
    if s.starts_with('[') && s.ends_with(']') {
        return false;
    }

    // Known news sources get priority
    if KNOWN_NEWS_SOURCES.iter().any(|&src| s == src) {
        return true;
    }

    // Generic check: short, no periods (except in names like "U.S.")
    !s.contains('.') || s == "U.S." || s == "N.Y." || s.ends_with(" News")
}

/// Check if a string looks like a headline (longer text).
fn is_headline(s: &str) -> bool {
    s.len() > 30 || s.split_whitespace().count() > 5
}

/// Check if a string looks like a relative timestamp.
///
/// Matches patterns like:
/// - "2 hours ago", "Yesterday", "3 days ago"
/// - "1h", "2h", "45m", "1m ago", "2d"
/// - "5:00 PM", "11:25 AM"
/// - "2/2/2026", "Feb. 3, 2026"
fn is_timestamp(s: &str) -> bool {
    let lower = s.to_lowercase();
    let trimmed = s.trim();
    
    // Standard "X time ago" patterns
    if lower.contains("ago")
        && (lower.contains("hour")
            || lower.contains("minute")
            || lower.contains("day")
            || lower.contains("second")
            || lower.contains("week")
            || lower.contains("month"))
    {
        return true;
    }
    
    // Short forms: "1h", "2m", "3d", "1m ago", "2h ago"
    if trimmed.len() <= 10 {
        // Pattern: digits followed by h/m/d/s optionally followed by "ago"
        let chars: Vec<char> = trimmed.chars().collect();
        if chars.len() >= 2 {
            let has_digit = chars.iter().any(|c| c.is_ascii_digit());
            let has_time_unit = lower.ends_with('h') 
                || lower.ends_with('m') 
                || lower.ends_with('d')
                || lower.ends_with('s')
                || lower.ends_with("h ago")
                || lower.ends_with("m ago")
                || lower.ends_with("d ago");
            if has_digit && has_time_unit {
                return true;
            }
        }
    }
    
    // "Yesterday", "Today"
    if lower == "yesterday" || lower == "today" {
        return true;
    }
    
    // Time patterns: "5:00 PM", "11:25 AM MST"
    if trimmed.contains(':') && (lower.contains("am") || lower.contains("pm")) {
        return true;
    }
    
    // Date patterns with slashes or specific months
    if trimmed.contains('/') && trimmed.chars().filter(|c| c.is_ascii_digit()).count() >= 4 {
        return true;
    }
    
    false
}

/// Check if a string is a known stock symbol.
fn is_stock_symbol(s: &str) -> bool {
    STOCK_SYMBOLS.iter().any(|&sym| s == sym)
}

/// Check if a string looks like an author byline.
///
/// Matches patterns like: "ANDREW CUNNINGHAM", "John Smith", "BY JANE DOE"
fn is_author_line(s: &str) -> bool {
    let trimmed = s.trim();
    
    // "By Author Name" pattern
    if trimmed.starts_with("By ") || trimmed.starts_with("BY ") {
        return true;
    }
    
    // All caps name (common in some publications)
    if trimmed.len() > 3 && trimmed.len() < 40 {
        let words: Vec<&str> = trimmed.split_whitespace().collect();
        if words.len() >= 2 && words.len() <= 4 {
            // Check if it looks like a name (capitalized words, no special chars)
            let looks_like_name = words.iter().all(|w| {
                w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                    && w.chars().all(|c| c.is_alphabetic() || c == '.' || c == '-')
            });
            if looks_like_name {
                return true;
            }
        }
    }
    
    false
}

/// Check if a string looks like a read time indicator.
///
/// Matches patterns like: "5 MIN READ", "3 minute read", "2 min"
fn is_read_time(s: &str) -> bool {
    let lower = s.to_lowercase();
    (lower.contains("min") && lower.contains("read"))
        || (lower.ends_with(" min read"))
        || (lower.ends_with(" minute read"))
}

/// Check if a string looks like a sports score line.
///
/// Matches patterns like: "Lakers 105 - Celtics 98", "NYK 3 | BOS 2"
fn is_sports_score(s: &str) -> bool {
    let trimmed = s.trim();
    
    // Check for score-like pattern: has numbers and team-like words
    let has_numbers = trimmed.chars().any(|c| c.is_ascii_digit());
    let has_separator = trimmed.contains(" - ") || trimmed.contains(" | ") || trimmed.contains(" vs ");
    
    // Team abbreviations or names with scores
    if has_numbers && (has_separator || trimmed.contains("Spread:") || trimmed.contains("Total:")) {
        return true;
    }
    
    false
}

/// Extract author from a combined "AUTHOR NAME – timestamp" line.
fn extract_author_timestamp(s: &str) -> Option<(String, String)> {
    // Pattern: "ANDREW CUNNINGHAM – 2/2/2026" or "John Smith – 5:00 PM"
    for sep in &[" – ", " - ", " | "] {
        if let Some(pos) = s.find(sep) {
            let author = s[..pos].trim();
            let timestamp = s[pos + sep.len()..].trim();
            if is_author_line(author) && (is_timestamp(timestamp) || timestamp.parse::<u32>().is_ok()) {
                return Some((author.to_string(), timestamp.to_string()));
            }
        }
    }
    None
}

/// Check if a string looks like a percentage.
fn is_percentage(s: &str) -> bool {
    s.ends_with('%') || (s.starts_with('+') || s.starts_with('-')) && s.contains('%')
}

/// Check if a string looks like a number (with commas/decimals).
fn is_number(s: &str) -> bool {
    let cleaned = s.replace(',', "").replace('.', "");
    !cleaned.is_empty() && cleaned.chars().all(|c| c.is_ascii_digit())
}

// ============================================================================
// CONTENT PARSING
// ============================================================================

/// Main entry point for formatting text for terminal output.
/// Formatting entry point that allows URL-aware parsing.
fn format_for_terminal_with_url(url: Option<&str>, lines: &[String], width: usize) -> Vec<String> {
    let parsed = parse_content_with_context(url, lines);
    render_parsed_content(&parsed, width)
}

/// Parse raw lines into structured content blocks.
///
/// This is the main parsing function that:
/// 1. Filters noise from the input
/// 2. Iterates through lines looking for patterns
/// 3. Creates appropriate ContentBlock variants
///
/// The parsing is greedy - each pattern tries to consume as many lines
/// as it needs, then returns control to the main loop.
fn parse_content_with_context(url: Option<&str>, lines: &[String]) -> Vec<ContentBlock> {
    let filtered = filter_noise(lines);
    let grouped = group_lines(&filtered);
    let grouped_refs: Vec<&str> = grouped.iter().map(|s| s.as_str()).collect();

    if let Some(url) = url {
        if let Some(parser) = select_site_parser(url) {
            return parser.parse(&grouped_refs);
        }
    }

    parse_content_filtered(&grouped_refs)
}

fn parse_content_filtered(lines: &[&str]) -> Vec<ContentBlock> {
    let filtered = lines;

    let mut blocks = Vec::new();
    let mut i = 0;

    while i < filtered.len() {
        let current = filtered[i];

        // Try parsers in order of specificity (most specific first)

        // 1. Preformatted content (ASCII art, tables)
        if is_preformatted(current) {
            blocks.push(ContentBlock::Preformatted(current.to_string()));
            i += 1;
            continue;
        }

        // 2. Weather block (consumes multiple lines)
        if let Some((weather, consumed)) = try_parse_weather(&filtered[i..]) {
            blocks.push(weather);
            i += consumed;
            continue;
        }

        // 3. Section headers
        if is_section_header(current) {
            blocks.push(ContentBlock::SectionHeader(current.to_string()));
            i += 1;
            continue;
        }

        // 4. Numbered list items (HN style)
        if let Some((num_item, consumed)) = try_parse_numbered_item(&filtered[i..]) {
            blocks.push(num_item);
            i += consumed;
            continue;
        }

        // 5. Navigation menus (sequence of short items)
        if is_nav_item(current) {
            let mut nav_items = vec![current.to_string()];
            let mut j = i + 1;
            while j < filtered.len() && is_nav_item(filtered[j]) && nav_items.len() < 10 {
                nav_items.push(filtered[j].to_string());
                j += 1;
            }
            if nav_items.len() >= 3 {
                blocks.push(ContentBlock::Navigation(nav_items));
                i = j;
                continue;
            }
        }

        // 6. Article with metadata (headline + author/time combo)
        if let Some((article, consumed)) = try_parse_article(&filtered[i..]) {
            blocks.push(article);
            i += consumed;
            continue;
        }

        // 7. News items (source + headline + timestamp)
        if is_source_or_category(current) && i + 1 < filtered.len() {
            let next = filtered[i + 1];
            if is_headline(next) {
                let mut timestamp = None;
                let mut consumed = 2;

                if i + 2 < filtered.len() && is_timestamp(filtered[i + 2]) {
                    timestamp = Some(filtered[i + 2].to_string());
                    consumed = 3;
                }

                // Skip author bylines
                while i + consumed < filtered.len() && filtered[i + consumed].starts_with("By ") {
                    consumed += 1;
                }

                blocks.push(ContentBlock::NewsItem {
                    source: current.to_string(),
                    headline: next.to_string(),
                    timestamp,
                });
                i += consumed;
                continue;
            }
        }

        // 8. Stock tickers
        if is_stock_symbol(current) && i + 2 < filtered.len() {
            let pct = filtered[i + 1];
            let value = filtered[i + 2];
            if is_percentage(pct) && is_number(value) {
                blocks.push(ContentBlock::StockTicker {
                    symbol: current.to_string(),
                    value: value.to_string(),
                    change: pct.to_string(),
                });
                i += 3;
                continue;
            }
        }

        // 9. Sports scores
        if is_sports_score(current) {
            blocks.push(ContentBlock::SportsScore {
                teams: current.to_string(),
                score: None,
                status: None,
            });
            i += 1;
            continue;
        }

        // 10. Fallback: plain text
        blocks.push(ContentBlock::PlainText(current.to_string()));
        i += 1;
    }

    blocks
}

/// Try to parse a weather block from the current position.
///
/// Weather data typically includes:
/// - Location name
/// - Current temperature
/// - "Google Weather" marker (optional)
/// - Forecast with day names and temperatures
///
/// # Returns
/// Some((ContentBlock::Weather, lines_consumed)) if successful, None otherwise
fn try_parse_weather(lines: &[&str]) -> Option<(ContentBlock, usize)> {
    if lines.len() < 5 {
        return None;
    }

    // Look for weather indicators
    let has_weather_marker = lines.iter().take(10).any(|s| *s == "Google Weather");
    let has_temp = lines.iter().take(10).any(|s| is_temperature(s));

    if !has_weather_marker && !has_temp {
        return None;
    }

    let mut location = String::new();
    let mut temp = String::new();
    let mut forecast: Vec<(String, String, String)> = Vec::new();
    let mut consumed = 0;

    let mut current_day = String::new();
    let mut high_temp = String::new();

    for (idx, &line) in lines.iter().enumerate().take(20) {
        if line == "Google Weather" {
            consumed = idx + 1;
            continue;
        }

        // Location (city name)
        if location.is_empty() && !is_temperature(line) && !is_day_name(line) && line.len() > 2 {
            if !line.chars().all(|c| c.is_ascii_digit() || c == '°') {
                location = line.to_string();
                consumed = idx + 1;
                continue;
            }
        }

        // Current temperature
        if temp.is_empty() && is_temperature(line) {
            temp = line.to_string();
            consumed = idx + 1;
            continue;
        }

        // Day name (start of forecast entry)
        if is_day_name(line) {
            if !current_day.is_empty() && !high_temp.is_empty() {
                forecast.push((current_day.clone(), high_temp.clone(), String::new()));
            }
            current_day = line.to_string();
            high_temp.clear();
            consumed = idx + 1;
            continue;
        }

        // Temperatures for forecast
        if !current_day.is_empty() && is_temperature(line) {
            if high_temp.is_empty() {
                high_temp = line.to_string();
            } else {
                // This is low temp - complete the forecast entry
                forecast.push((current_day.clone(), high_temp.clone(), line.to_string()));
                current_day.clear();
                high_temp.clear();
            }
            consumed = idx + 1;
            continue;
        }

        // Stop at section headers or news content
        if is_section_header(line) || line == "Top stories" || line == "Local news" {
            break;
        }
    }

    // Add any remaining day
    if !current_day.is_empty() && !high_temp.is_empty() {
        forecast.push((current_day, high_temp, String::new()));
    }

    if !temp.is_empty() || !forecast.is_empty() {
        Some((
            ContentBlock::Weather {
                location,
                temp,
                forecast,
            },
            consumed,
        ))
    } else {
        None
    }
}

/// Try to parse an article with metadata (tech blogs, news sites).
///
/// Detects patterns like:
/// - Headline followed by "AUTHOR NAME – timestamp"
/// - Headline followed by description followed by author
fn try_parse_article(lines: &[&str]) -> Option<(ContentBlock, usize)> {
    if lines.is_empty() {
        return None;
    }

    let current = lines[0];
    
    // Must look like a headline
    if !is_headline(current) {
        return None;
    }
    
    // Skip if it looks like a source (those are handled by NewsItem)
    if is_source_or_category(current) {
        return None;
    }
    
    let mut author = None;
    let mut timestamp = None;
    let mut read_time = None;
    let mut description = None;
    let mut consumed = 1;
    
    // Look at following lines for metadata
    for j in 1..lines.len().min(5) {
        let line = lines[j];
        
        // Check for combined "AUTHOR – timestamp" pattern
        if let Some((auth, ts)) = extract_author_timestamp(line) {
            author = Some(auth);
            timestamp = Some(ts);
            consumed = j + 1;
            continue;
        }
        
        // Check for standalone author
        if author.is_none() && is_author_line(line) {
            author = Some(line.to_string());
            consumed = j + 1;
            continue;
        }
        
        // Check for standalone timestamp
        if timestamp.is_none() && is_timestamp(line) {
            timestamp = Some(line.to_string());
            consumed = j + 1;
            continue;
        }
        
        // Check for read time
        if read_time.is_none() && is_read_time(line) {
            read_time = Some(line.to_string());
            consumed = j + 1;
            continue;
        }
        
        // Check for description (longer text that's not metadata)
        if description.is_none() 
            && line.len() > 40 
            && !is_author_line(line) 
            && !is_timestamp(line)
            && !is_read_time(line)
            && !is_section_header(line) 
        {
            description = Some(line.to_string());
            consumed = j + 1;
            continue;
        }
        
        // Stop if we hit something that looks like a new item
        if is_section_header(line) || is_headline(line) && j > 2 {
            break;
        }
    }
    
    // Only return Article if we found some metadata
    if author.is_some() || timestamp.is_some() || read_time.is_some() {
        Some((
            ContentBlock::Article {
                headline: current.to_string(),
                author,
                timestamp,
                read_time,
                description,
            },
            consumed,
        ))
    } else {
        None
    }
}

/// Try to parse a numbered list item (like Hacker News posts).
///
/// Handles two formats:
/// 1. "1. Title here" (number and content on same line)
/// 2. "1." then "Title here" on next line (HN format)
///
/// Also captures metadata like "239 points · 4 hours ago · 90 comments"
fn collect_metadata(lines: &[&str], start: usize, max_lines: usize) -> (Option<String>, usize) {
    if start >= lines.len() || max_lines == 0 {
        return (None, 0);
    }

    let mut parts = Vec::new();
    let mut consumed = 0;

    for i in start..lines.len().min(start + max_lines) {
        let line = lines[i];
        if is_metadata_line(line) || line.contains('|') || line.contains("hide") {
            parts.push(line.to_string());
            consumed += 1;
        } else {
            break;
        }
    }

    if parts.is_empty() {
        (None, 0)
    } else {
        (Some(parts.join(" | ")), consumed)
    }
}

fn try_parse_numbered_item(lines: &[&str]) -> Option<(ContentBlock, usize)> {
    if lines.is_empty() {
        return None;
    }

    let current = lines[0];

    // Check for "N." pattern
    let mut chars = current.chars().peekable();
    let mut num = String::new();

    while chars.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        num.push(chars.next().unwrap());
    }

    if num.is_empty() {
        return None;
    }

    // Number must be >= 1 (avoid "0." patterns like "0.0 in")
    if num.parse::<u32>().unwrap_or(0) < 1 {
        return None;
    }

    // Must be followed by a period
    if chars.next() != Some('.') {
        return None;
    }

    let rest: String = chars.collect();
    let rest = rest.trim();

    // Case 1: Number and content on same line
    if !rest.is_empty() {
        let mut consumed = 1;
        let (metadata, meta_consumed) = collect_metadata(lines, 1, 2);
        if meta_consumed > 0 {
            consumed += meta_consumed;
        }

        return Some((
            ContentBlock::NumberedItem {
                number: num,
                text: rest.to_string(),
                metadata,
            },
            consumed,
        ));
    }

    // Case 2: Number on its own line (HN format)
    if lines.len() < 2 {
        return None;
    }

    let title = lines[1];

    // Title should not be another number or metadata
    if title
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
        && title.contains('.')
    {
        return None;
    }
    if title.contains("points") && title.contains("ago") {
        return None;
    }

    let mut consumed = 2;
    let (metadata, meta_consumed) = collect_metadata(lines, 2, 2);
    if meta_consumed > 0 {
        consumed += meta_consumed;
    }

    Some((
        ContentBlock::NumberedItem {
            number: num,
            text: title.to_string(),
            metadata,
        },
        consumed,
    ))
}

// ============================================================================
// CONTENT RENDERING
// ============================================================================

/// Render parsed content blocks to formatted terminal output.
///
/// Each content type has its own rendering style:
/// - Weather: Boxed with separators
/// - Section headers: Emphasized with symbols
/// - News items: Bulleted with metadata below
/// - Numbered items: Right-aligned numbers with metadata
/// - Plain text: Wrapped to terminal width
fn render_parsed_content(blocks: &[ContentBlock], width: usize) -> Vec<String> {
    let mut output = Vec::new();
    let mut last_was_news = false;

    for block in blocks {
        match block {
            ContentBlock::Weather {
                location,
                temp,
                forecast,
            } => {
                output.push(String::new());
                output.push(separator(width, '─'));

                let weather_line = if !location.is_empty() && !temp.is_empty() {
                    format!("  {} — {}", location, temp)
                } else if !temp.is_empty() {
                    format!("  Weather: {}", temp)
                } else {
                    "  Weather".to_string()
                };
                output.push(weather_line);

                if !forecast.is_empty() {
                    let forecast_str: Vec<String> = forecast
                        .iter()
                        .take(5)
                        .map(|(day, high, low)| {
                            if low.is_empty() {
                                format!("{} {}", day, high)
                            } else {
                                format!("{} {}/{}", day, high, low)
                            }
                        })
                        .collect();
                    output.push(format!("  Forecast: {}", forecast_str.join("  ")));
                }

                output.push(separator(width, '─'));
                output.push(String::new());
                last_was_news = false;
            }

            ContentBlock::SectionHeader(header) => {
                output.push(String::new());
                output.push(format!("━━ {} ━━", header.to_uppercase()));
                output.push(String::new());
                last_was_news = false;
            }

            ContentBlock::NewsItem {
                source,
                headline,
                timestamp,
            } => {
                if !last_was_news {
                    output.push(String::new());
                }

                // Format: • Headline
                //         [Source] (timestamp)
                let meta = match timestamp {
                    Some(ts) => format!("[{}] ({})", source, ts),
                    None => format!("[{}]", source),
                };

                let prefix = "  • ";
                let meta_line = format!("    {}", meta);

                let headline_lines = wrap_text(headline, width, prefix.len());
                output.push(format!("{}{}", prefix, headline_lines[0]));
                for line in headline_lines.iter().skip(1) {
                    output.push(format!("    {}", line));
                }
                output.push(meta_line);

                last_was_news = true;
            }

            ContentBlock::StockTicker {
                symbol,
                value,
                change,
            } => {
                let arrow = if change.starts_with('+') || change.starts_with('▲') {
                    "▲"
                } else if change.starts_with('-') || change.starts_with('▼') {
                    "▼"
                } else {
                    " "
                };
                output.push(format!("  {} {} {} {}", symbol, value, arrow, change));
                last_was_news = false;
            }

            ContentBlock::Navigation(items) => {
                let nav_line = items.join(" | ");
                if nav_line.len() <= width {
                    output.push(format!("  {}", nav_line));
                }
                last_was_news = false;
            }

            ContentBlock::NumberedItem {
                number,
                text,
                metadata,
            } => {
                // Right-align number for clean lists
                let prefix = format!("{:>2}. ", number);
                let wrapped = wrap_text(text, width, prefix.len());

                output.push(format!("{}{}", prefix, wrapped[0]));
                for line in wrapped.iter().skip(1) {
                    output.push(format!("    {}", line));
                }

                if let Some(meta) = metadata {
                    let meta_formatted = format_hn_metadata(meta);
                    output.push(format!("    {}", meta_formatted));
                }

                last_was_news = false;
            }

            ContentBlock::Preformatted(text) => {
                output.push(text.clone());
                last_was_news = false;
            }

            ContentBlock::Article {
                headline,
                author,
                timestamp,
                read_time,
                description,
            } => {
                if !last_was_news {
                    output.push(String::new());
                }

                // Headline
                let headline_lines = wrap_text(headline, width, 0);
                for line in &headline_lines {
                    output.push(line.clone());
                }

                // Description if present
                if let Some(desc) = description {
                    let desc_lines = wrap_text(desc, width, 2);
                    for line in desc_lines {
                        output.push(format!("  {}", line));
                    }
                }

                // Metadata line: Author · Timestamp · Read time
                let mut meta_parts = Vec::new();
                if let Some(auth) = author {
                    meta_parts.push(auth.clone());
                }
                if let Some(ts) = timestamp {
                    meta_parts.push(ts.clone());
                }
                if let Some(rt) = read_time {
                    meta_parts.push(rt.clone());
                }
                if !meta_parts.is_empty() {
                    output.push(format!("  {}", meta_parts.join(" · ")));
                }

                last_was_news = true;
            }

            ContentBlock::SportsScore { teams, score, status } => {
                let mut line = format!("  {}", teams);
                if let Some(sc) = score {
                    line.push_str(&format!(" {}", sc));
                }
                if let Some(st) = status {
                    line.push_str(&format!(" ({})", st));
                }
                output.push(line);
                last_was_news = false;
            }

            ContentBlock::PlainText(text) => {
                let wrapped = wrap_text(text, width, 0);
                for line in wrapped {
                    output.push(line);
                }
                last_was_news = false;
            }
        }
    }

    clean_blank_lines(output)
}

/// Format Hacker News-style metadata more compactly.
///
/// Input: "239 points by johnspurlock 4 hours ago | hide | 90 comments"
/// Output: "239 points by johnspurlock 4 hours ago · 90 comments"
fn format_hn_metadata(meta: &str) -> String {
    let parts: Vec<&str> = meta.split('|').map(|s| s.trim()).collect();

    let mut formatted_parts = Vec::new();

    for part in parts {
        let part = part.trim();
        if part == "hide" || part.is_empty() {
            continue;
        }
        if part.contains("points") || part.contains("comment") || part.contains("ago") {
            formatted_parts.push(part.to_string());
        }
    }

    if formatted_parts.is_empty() {
        meta.to_string()
    } else {
        formatted_parts.join(" · ")
    }
}

/// Remove excessive consecutive blank lines.
///
/// Limits to max 2 consecutive blank lines and trims leading/trailing blanks.
fn clean_blank_lines(lines: Vec<String>) -> Vec<String> {
    let mut result = Vec::new();
    let mut blank_count = 0;

    for line in lines {
        if line.trim().is_empty() {
            blank_count += 1;
            if blank_count <= 2 {
                result.push(String::new());
            }
        } else {
            blank_count = 0;
            result.push(line);
        }
    }

    // Trim leading blanks
    while result.first().map(|s| s.is_empty()).unwrap_or(false) {
        result.remove(0);
    }
    // Trim trailing blanks
    while result.last().map(|s| s.is_empty()).unwrap_or(false) {
        result.pop();
    }

    result
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn has_plain_text(blocks: &[ContentBlock], text: &str) -> bool {
        blocks.iter().any(|b| match b {
            ContentBlock::PlainText(t) => t == text,
            _ => false,
        })
    }

    #[test]
    fn test_is_temperature() {
        assert!(is_temperature("72°"));
        assert!(is_temperature("72°F"));
        assert!(is_temperature("22°C"));
        assert!(!is_temperature("hello"));
        assert!(!is_temperature("72"));
    }

    #[test]
    fn test_is_timestamp() {
        assert!(is_timestamp("2 hours ago"));
        assert!(is_timestamp("Yesterday"));
        assert!(is_timestamp("3 days ago"));
        assert!(!is_timestamp("Tomorrow"));
        assert!(!is_timestamp("CNN"));
    }

    #[test]
    fn test_is_preformatted() {
        assert!(is_preformatted("┌──────────┐"));
        assert!(is_preformatted("│ content  │"));
        assert!(is_preformatted(".--.  some weather art  .--."));
        assert!(is_preformatted("(___.__)__) 9 mi"));
        assert!(!is_preformatted("Hello world"));
        assert!(!is_preformatted("This is regular text content"));
    }

    #[test]
    fn test_wrap_text() {
        let wrapped = wrap_text("This is a test of text wrapping functionality", 20, 0);
        assert!(wrapped.len() > 1);
        assert!(wrapped.iter().all(|l| l.len() <= 20));
    }

    #[test]
    fn test_is_source_or_category() {
        assert!(is_source_or_category("CNN"));
        assert!(is_source_or_category("The New York Times"));
        assert!(!is_source_or_category("This is a long headline that should not be a source"));
        assert!(!is_source_or_category("2 hours ago"));
    }

    #[test]
    fn test_filter_noise() {
        let lines = vec![
            "chevron_right".to_string(),
            "Sign in".to_string(),
            "Actual content".to_string(),
            "More".to_string(),
        ];
        let filtered = filter_noise(&lines);
        assert_eq!(filtered, vec!["Actual content"]);
    }

    #[test]
    fn test_hn_parsing_merges_domain_and_metadata() {
        let lines = vec![
            "Hacker News | new | past | comments | ask | show | jobs | submit".to_string(),
            "1.".to_string(),
            "Test story".to_string(),
            "(example.com)".to_string(),
            "123 points by alice 2 hours ago | hide | 9 comments".to_string(),
        ];
        let blocks = parse_content_with_context(Some("https://news.ycombinator.com"), &lines);
        let item = blocks.iter().find_map(|b| match b {
            ContentBlock::NumberedItem { number, text, metadata } => {
                Some((number.clone(), text.clone(), metadata.clone()))
            }
            _ => None,
        });
        assert!(item.is_some());
        let (number, text, metadata) = item.unwrap();
        assert_eq!(number, "1");
        assert_eq!(text, "Test story (example.com)");
        let meta = metadata.unwrap_or_default();
        assert!(meta.contains("points"));
        assert!(meta.contains("comments"));
    }

    #[test]
    fn test_site_noise_filtered_for_picker_sites() {
        let cases = vec![
            ("https://news.google.com", "Top stories"),
            ("https://github.com/trending", "Spoken Language"),
            ("https://www.reddit.com/r/programming", "Open in app"),
            ("https://dev.to", "DEV Community"),
            ("https://www.npr.org", "Sections"),
            ("https://en.wikipedia.org", "Jump to navigation"),
            ("https://www.imdb.com/chart/top", "IMDb Charts"),
            ("https://www.reuters.com", "Latest"),
        ];

        for (url, noise) in cases {
            let lines = vec![noise.to_string(), "1. Example item".to_string()];
            let blocks = parse_content_with_context(Some(url), &lines);
            assert!(!has_plain_text(&blocks, noise));
        }
    }
}
