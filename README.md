# CurlUp

A terminal web browser that lets you browse the web by clicking numbered links. CurlUp renders pages using Chrome, displays the text in your terminal, and lets you navigate by entering link numbers.

## Why CurlUp?

- **Browse the web from your terminal** - Navigate sites by typing link numbers
- **Real browser rendering** - JavaScript, SPAs, and dynamic content work out of the box
- **Beautiful start screen** - Pick from popular sites or enter any URL
- **Back navigation** - Browse history lets you go back to previous pages
- **Zero API hacking** - If Chrome can render it, CurlUp can read it

## Quick Start

```bash
# Just run curlup - pick a site and start browsing!
curlup

# Or go directly to a site
curlup news.ycombinator.com

# For scripts/piping, use single-page mode
curlup -s news.google.com | less
```

## Installation

### Prerequisites

1. **Rust toolchain** (1.70+)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Google Chrome** - Any recent version

3. **ChromeDriver** - Must match your Chrome version

### Installing ChromeDriver

#### macOS (Homebrew)
```bash
brew install chromedriver
```

#### macOS (Manual)
```bash
# Check your Chrome version: Chrome → About Google Chrome
# Download matching ChromeDriver from https://googlechromelabs.github.io/chrome-for-testing/

# Install to ~/.local/bin (CurlUp checks here first)
mkdir -p ~/.local/bin
mv chromedriver ~/.local/bin/
chmod +x ~/.local/bin/chromedriver

# On macOS, remove quarantine attribute
xattr -d com.apple.quarantine ~/.local/bin/chromedriver
```

#### Linux
```bash
# Debian/Ubuntu
sudo apt install chromium-chromedriver

# Or download from https://googlechromelabs.github.io/chrome-for-testing/
```

#### Windows
```powershell
# Download from https://googlechromelabs.github.io/chrome-for-testing/
# Add to PATH or place in a directory on your PATH
```

### Building CurlUp

```bash
git clone <repository-url>
cd CurlUp
cargo build --release

# Binary is at target/release/curlup
# Optionally, copy to your PATH:
cp target/release/curlup ~/.local/bin/
```

Build artifacts live under `target/`. If you create packaged releases under `release/`,
keep them out of version control (they are generated outputs).

## Usage

```
curlup [OPTIONS] [URL]

Arguments:
  [URL]  URL to navigate to (optional - shows site picker if omitted)

Options:
  -s, --single   Single-page mode (fetch once and exit, good for piping)
  -v, --visible  Launch Chrome in visible mode (useful for login/debug)
  -r, --raw      Raw output mode (no formatting - implies --single)
  -h, --help     Print help
  -V, --version  Print version
```

### How It Works

**1. Start Screen**

Run `curlup` to see the welcome screen with popular sites:

```
    ╭─────────────────────────────────────────╮
    │      ██████╗██╗   ██╗██████╗ ██╗        │
    │     ██╔════╝██║   ██║██╔══██╗██║        │
    │     ██║     ██║   ██║██████╔╝██║        │
    │     ╚██████╗╚██████╔╝██║  ██║███████╗   │
    │       Terminal Web Browser v0.1         │
    ╰─────────────────────────────────────────╯

  Popular Sites

  🔶  1. Hacker News          [Tech]
  📰  2. Google News          [News]
  ⭐  3. GitHub Trending      [Dev]
  💬  4. Reddit Programming   [Community]
  ...
  🔗 11. Enter a custom URL...
```

**2. Browse Pages with Numbered Links**

Once you pick a site, CurlUp shows the page content with clickable links:

```
────────────────────────────────────────────────────────────────────
  CurlUp ← 1 https://news.ycombinator.com
────────────────────────────────────────────────────────────────────

━━ TOP STORIES ━━

 1. Show HN: I built a terminal browser
    239 points · 4 hours ago · 90 comments

 2. The future of programming languages
    189 points · 6 hours ago · 156 comments

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  🔗 12 Links (enter a number to follow)

   1 Show HN: I built a terminal browser
   2 The future of programming languages
   3 Comments (90)
   ...
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  → q:quit h:help b:back 1-12:link _
```

**3. Navigate by Entering Numbers**

| Input | Action |
|-------|--------|
| `1-20` | Follow that link |
| `b` | Go back to previous page |
| `r` | Refresh current page |
| `u` | Show current URL |
| `google.com` | Go directly to any URL |
| `home` | Return to site picker |
| `h` | Show help |
| `q` | Quit |

### Examples

```bash
# Start with site picker, then browse interactively
curlup

# Go directly to a site and browse
curlup news.ycombinator.com
curlup github.com/trending
curlup en.wikipedia.org

# Single-page mode (for piping to other tools)
curlup -s news.google.com | less
curlup -s example.com | grep "keyword"

# Raw output for scripting
curlup -s -r news.google.com > output.txt

# Visible browser (for sites requiring login)
curlup -v mail.google.com

# Local development
curlup localhost:3000
```

### Output Modes

**Formatted (default)**: Terminal-optimized output with visual hierarchy
```
───────────────────────────────────────────────────────────────────
  Boulder — 46°F
  Forecast: Thu 63°/38°  Fri 59°/40°
───────────────────────────────────────────────────────────────────

━━ TOP STORIES ━━

  • House passes funding package to end partial government shutdown
    [ABC News] (2 hours ago)

  • Live updates: House begins votes to end government shutdown
    [CNN] (1 hour ago)

  Dow Jones 49,240.99 ▼ -0.34%

━━ LOCAL NEWS ━━

  • Boulder protects nesting eagles: Seasonal closures in effect
    [9News] (4 hours ago)
```

**Raw (`-r`)**: Shows every line separately (useful for scripting)
```
ABC News
More
House passes funding package to end shutdown
2 hours ago
By Lauren Peller
```

### Terminal Features

- **Auto line wrapping**: Respects your terminal width (reads `$COLUMNS`)
- **Visual sections**: Clear separators between content types
- **Weather formatting**: Compact forecast display
- **News hierarchy**: Headlines with source/timestamp metadata
- **Stock tickers**: Clean formatting with up/down indicators

## How It Works

```
┌─────────────┐
│   CurlUp    │  You run: curlup <url>
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ ChromeDriver│  Spawned automatically in background
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Chrome    │  Renders page (headless by default)
│  (headless) │  JavaScript executes, DOM builds
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Page Ready  │  Waits for readyState + content
│   Check     │  with retry and backoff
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  DOM Text   │  Extracts visible text content
│  Extraction │  via document.body.innerText
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Terminal   │  Clean text output
└─────────────┘
```

## Troubleshooting

### "ChromeDriver not found"

CurlUp searches for ChromeDriver in this order:
1. `~/.local/bin/chromedriver`
2. System PATH

Make sure ChromeDriver is installed and accessible:
```bash
# Check if chromedriver is found
which chromedriver

# Or verify the local path exists
ls ~/.local/bin/chromedriver
```

### "session not created: This version of ChromeDriver only supports Chrome version X"

Your ChromeDriver version doesn't match your Chrome version. Download the matching version from:
https://googlechromelabs.github.io/chrome-for-testing/

Check your Chrome version: `Chrome menu → About Google Chrome`

### "ChromeDriver failed to start within 10 seconds"

- Check if another ChromeDriver process is running: `pkill chromedriver`
- Verify ChromeDriver works manually: `chromedriver --port=9515`
- On macOS, ensure quarantine is removed: `xattr -d com.apple.quarantine ~/.local/bin/chromedriver`

### Page content looks incomplete

CurlUp uses a robust 3-step approach to wait for JavaScript-heavy pages:
1. Waits for `document.readyState === 'complete'`
2. Polls until the page body has actual text content (up to 5 seconds)
3. Retries content extraction with backoff if initial attempt fails

If content still appears incomplete, the site may be loading content via infinite scroll or lazy loading that requires user interaction.

## Development

```bash
# Run in development
cargo run -- https://example.com

# Run tests
cargo test

# Check for issues
cargo clippy
```

## Architecture

CurlUp's text parsing engine is designed for extensibility. The core pipeline:

```
Raw Lines → Filter → Parse → Render
    │          │        │       │
    │          │        │       └─► Terminal output with formatting
    │          │        └─► ContentBlock enum (structured data)
    │          └─► Remove noise, icons, UI elements
    └─► Vec<String> from DOM extraction
```

### Content Types

The parser recognizes these content patterns (see `src/render/text.rs`):

| Type | Example | Detection |
|------|---------|-----------|
| `NewsItem` | Headline with source/timestamp | Known sources + headline length |
| `NumberedItem` | HN-style "1. Title" lists | Number + period pattern |
| `Weather` | Temperature + forecast | °F/°C patterns + day names |
| `StockTicker` | "Dow Jones 49,240 ▲ +0.5%" | Known symbols + percentage |
| `SectionHeader` | "TOP STORIES" | Known header list |
| `Preformatted` | ASCII art, tables | Box chars + special char ratio |
| `Navigation` | Menu items | Short items in sequence |

### Adding Site-Specific Parsers

The `ContentParser` trait allows custom parsing for specific sites:

```rust
pub trait ContentParser {
    fn can_parse(&self, url: &str) -> bool;
    fn parse(&self, lines: &[&str]) -> Vec<ContentBlock>;
}
```

See `src/render/text.rs` for the full API documentation.

## Compatible Sites

See [SITES.md](SITES.md) for a full list of tested websites and compatibility ratings.

**Best results:**
- News aggregators: Google News, Hacker News, NPR, Reuters
- Tech blogs: Ars Technica, TechCrunch, DEV Community
- Documentation: MDN Web Docs, Python Docs
- Reference: Merriam-Webster, Wikipedia
- Weather: wttr.in (ASCII art preserved)
- Entertainment: IMDb, Rotten Tomatoes, Goodreads

**Note:** Some sites (Product Hunt) use Cloudflare protection that blocks automated browsers. Sites like Twitter/X, LinkedIn, and Medium require login to access content (use `-v` flag).

## Roadmap

- [ ] Login workflow with session persistence
- [x] Smart page loading with readyState check, content polling, and retry
- [x] Link extraction and navigation (`-b` browse mode)
- [ ] Site-specific extractors (Gmail, GitHub, etc.)

## License

MIT

## Acknowledgments

Built with:
- [fantoccini](https://github.com/jonhoo/fantoccini) - WebDriver client for Rust
- [tokio](https://tokio.rs/) - Async runtime
- [clap](https://clap.rs/) - CLI argument parsing
- [dialoguer](https://github.com/console-rs/dialoguer) - Interactive terminal prompts
