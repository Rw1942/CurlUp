# CurlUp Development Progress

## Status: Production Ready

CurlUp is fully functional with an extensible text parsing engine tested against 30+ websites.

## Files Created

```
CurlUp/
├── Cargo.toml           # Project manifest with dependencies
├── README.md            # User documentation
├── SITES.md             # Compatible websites list
├── guide.md             # Original development guide
├── progress.md          # This file
├── release/             # Pre-built binaries
│   └── curlup-0.1.0-darwin-arm64/
└── src/
    ├── main.rs          # Application entry point + URL normalization
    ├── cli.rs           # CLI argument parsing (clap)
    ├── error.rs         # Error types (thiserror)
    ├── browser/
    │   ├── mod.rs       # Browser module exports
    │   ├── chrome.rs    # ChromeDriver connection with capabilities
    │   ├── driver.rs    # ChromeDriver process management
    │   └── session.rs   # Page navigation and waiting
    ├── dom/
    │   ├── mod.rs       # DOM module exports
    │   └── extract.rs   # Text extraction from DOM
    └── render/
        ├── mod.rs       # Render module exports
        └── text.rs      # Terminal text rendering engine (~1200 lines)
```

## What's Implemented

1. **CLI** - Full help text, version, `-v`/`--visible` flag, `-r`/`--raw` flag, examples
2. **URL normalization** - Auto-adds `https://` (or `http://` for localhost)
3. **ChromeDriver auto-launch** - Spawns ChromeDriver in background automatically
4. **Browser connection** - Connects with headless/visible capabilities
5. **Page navigation** - Navigates to URL, waits for body + 2s for JS
6. **DOM extraction** - Uses `document.body.innerText` for reliable extraction
7. **Terminal-optimized rendering** - Modular, extensible text parsing engine:

   **Content Types Detected:**
   - `Weather` - Location, temperature, multi-day forecast
   - `SectionHeader` - Visual section breaks (━━ TOP STORIES ━━)
   - `NewsItem` - Source + headline + timestamp
   - `Article` - Headline + author + timestamp + read time + description
   - `NumberedItem` - HN-style lists with metadata
   - `StockTicker` - Symbol + value + change with arrows
   - `SportsScore` - Teams + scores + status
   - `Navigation` - Menu items grouped compactly
   - `Preformatted` - ASCII art preserved without wrapping

   **Detection Features:**
   - Terminal width detection (via `$COLUMNS`, defaults to 80)
   - Smart text wrapping at word boundaries
   - Timestamp detection: "2 hours ago", "1h", "5:00 PM", "2/2/2026"
   - Author extraction from bylines and combined author-timestamp lines
   - Read time detection ("5 MIN READ")
   - 50+ known news sources for reliable detection

   **Noise Filtering:**
   - Material Design icon ligatures (chevron_right, etc.)
   - UI chrome (Subscribe, Sign in, Skip to content, etc.)
   - Cloudflare security messages
   - Image credits (Getty, Reuters, AP)
   - Standalone bracketed numbers [1], [2], [3]

   **Architecture:**
   - `ContentBlock` enum for structured content
   - `ContentParser` trait for site-specific extensibility
   - Comprehensive rustdoc documentation
   - 6 unit tests for core detection functions

8. **Raw output mode** - `-r` flag for unformatted output (scripting)
9. **Process cleanup** - Kills ChromeDriver on exit
10. **User-friendly errors** - Actionable messages with installation help

## Tested Sites (30+)

See [SITES.md](SITES.md) for full compatibility list.

**★★★★★ Excellent:**
- news.google.com - Weather, sections, headlines
- news.ycombinator.com - Numbered lists with points/comments
- arstechnica.com - Headlines + descriptions + author/time
- wttr.in - ASCII art preserved perfectly

**★★★★☆ Good:**
- bbc.com/news, cnn.com, nytimes.com
- techcrunch.com, theverge.com
- github.com/trending, espn.com

**✗ Blocked (Cloudflare):**
- reddit.com, medium.com, producthunt.com

## Setup

### ChromeDriver

CurlUp searches for ChromeDriver in:
1. `~/.local/bin/chromedriver` (preferred)
2. System PATH

Install via Homebrew: `brew install chromedriver`

### Running

```bash
# Simple usage - just the domain
cargo run -- news.google.com

# With visible browser window (for login/debugging)
cargo run -- -v mail.google.com

# Raw output for scripting
cargo run -- -r example.com | grep "keyword"

# Local development
cargo run -- localhost:3000
```

## Completed

- [x] Add `--visible` flag handling (launch visible Chrome)
- [x] Add error messages for missing ChromeDriver
- [x] URL auto-scheme (https:// added automatically)
- [x] Comprehensive README
- [x] CLI help with examples
- [x] Add `--condensed` flag for cleaner output
- [x] Improve text formatting for terminal readability
- [x] Article metadata grouping (author · timestamp · read time)
- [x] Enhanced timestamp detection (short forms, times, dates)
- [x] Expanded noise filtering (50+ patterns)
- [x] New content types (Article, SportsScore)
- [x] Comprehensive site compatibility testing (30+ sites)
- [x] SITES.md compatibility documentation

## Next Steps

- [ ] Add login prompt workflow for authenticated sites
- [ ] Support persistent Chrome profile for session reuse
- [ ] Site-specific parsers (Gmail, GitHub issues, etc.)
- [ ] Configurable wait times for slow pages
- [ ] Link extraction and keyboard navigation
