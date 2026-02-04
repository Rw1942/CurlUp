# Developer Guide

How to work on CurlUp.

## Getting Started

### 1. Clone and build

```bash
git clone https://github.com/Rw1942/CurlUp.git
cd CurlUp
cargo build
```

### 2. Run in dev mode

```bash
cargo run -- example.com
cargo run -- -s news.ycombinator.com
cargo run -- -m -v medium.com/@user/article
```

### 3. Run tests

```bash
cargo test
cargo clippy
```

### 4. Make changes

Edit code, then `cargo run` to test. The main files you'll touch:

- `src/cli.rs` - Add CLI flags
- `src/browse.rs` - Change interactive behavior
- `src/dom/extract.rs` - Change how text is extracted
- `src/dom/multilens/` - Add/modify extraction strategies
- `src/render/text.rs` - Change terminal output formatting

---

## Architecture

CurlUp orchestrates Chrome - it doesn't render pages itself.

```
┌─────────────┐
│   CurlUp    │  CLI + orchestration (Rust)
└──────┬──────┘
       │
┌──────▼──────┐
│ ChromeDriver│  Spawned on port 9515
└──────┬──────┘
       │
┌──────▼──────┐
│   Chrome    │  Renders page (headless)
└──────┬──────┘
       │
┌──────▼──────┐
│  Extract    │  Pull text + links from DOM
└──────┬──────┘
       │
┌──────▼──────┐
│   Render    │  Format for terminal
└─────────────┘
```

---

## Project Structure

```
src/
├── main.rs              # Entry point, wires everything together
├── cli.rs               # CLI argument parsing (clap)
├── browse.rs            # Interactive browsing loop
├── picker.rs            # Start screen with site menu
├── term.rs              # Terminal utilities (clear, cursor, size)
├── error.rs             # Error types
│
├── browser/
│   ├── mod.rs           # Module exports
│   ├── driver.rs        # Start/stop ChromeDriver process
│   ├── chrome.rs        # Connect to Chrome, stealth settings
│   └── navigation.rs    # Navigate to URL, wait for page ready
│
├── dom/
│   ├── mod.rs           # Module exports
│   ├── content.rs       # PageContent and Link types
│   ├── extract.rs       # Standard extraction (innerText)
│   └── multilens/       # Smart multi-strategy extraction
│       ├── mod.rs       # Orchestrator, runs all strategies
│       ├── snapshot.rs  # Get full HTML from browser
│       ├── css.rs       # Extract via CSS selectors
│       ├── traversal.rs # Walk DOM tree
│       ├── density.rs   # Text-density algorithm (CETD)
│       ├── semantic.rs  # ARIA landmarks
│       ├── scorer.rs    # Rank results, pick best
│       └── normalize.rs # Clean up extracted text
│
└── render/
    ├── mod.rs           # Module exports
    └── text.rs          # Terminal output formatting
```

---

## Key Dependencies

| Crate | What it does |
|-------|--------------|
| fantoccini | WebDriver client (controls Chrome) |
| tokio | Async runtime |
| scraper | HTML parsing for multilens mode |
| dom-content-extraction | Text density algorithm |
| clap | CLI argument parsing |
| dialoguer | Interactive terminal prompts |
| console | Terminal styling |

---

## How Extraction Works

### Standard Mode (default)

Runs JavaScript in Chrome:
```javascript
document.body.innerText  // Get all visible text
document.querySelectorAll('a[href]')  // Get all links
```

Fast and works on any site.

### Multi-Lens Mode (`-m`)

Gets full HTML, parses in Rust, runs 4 strategies:

```
Chrome: document.documentElement.outerHTML
                    │
                    ▼
         scraper::Html::parse_document()
                    │
    ┌───────┬───────┼───────┬───────┐
    │       │       │       │       │
   CSS    DOM    CETD    ARIA    (future)
    │       │       │       │
    └───────┴───────┴───────┘
                    │
                    ▼
              Scorer picks best
                    │
                    ▼
              Clean + normalize
```

Each strategy returns content + confidence score. Scorer picks the winner.

---

## Common Tasks

### Add a CLI flag

1. Edit `src/cli.rs`:
```rust
#[arg(long, short = 'x')]
pub my_flag: bool,
```

2. Use it in `src/main.rs` or `src/browse.rs`:
```rust
if args.my_flag {
    // do something
}
```

### Add a new extraction strategy

1. Create `src/dom/multilens/mystrategy.rs`
2. Make it return a `ContentResult`:
```rust
pub fn extract_my_way(doc: &Html) -> ContentResult {
    ContentResult {
        content: extracted_text,
        source: "mystrategy".to_string(),
        confidence: 0.7,  // 0.0 to 1.0
    }
}
```
3. Add to `src/dom/multilens/mod.rs`:
```rust
mod mystrategy;
// ...
let my_result = mystrategy::extract_my_way(&doc);
candidates.push(my_result);
```

### Change terminal output

Edit `src/render/text.rs`. The main functions:
- `render()` - Raw output
- `render_condensed()` - Formatted output
- `render_with_links_limited()` - Output with numbered links

---

## Error Handling

- Use `Result<T, E>` everywhere
- Internal errors: `thiserror` enums in `src/error.rs`
- Top-level: `anyhow::Result` for easy error context
- Never use `unwrap()` in production code paths

---

## Testing

```bash
cargo test                      # Run unit tests
cargo test multilens            # Run multilens tests only
cargo run -- -s example.com     # Manual smoke test
```

Most tests are in the multilens modules. They test extraction against sample HTML.

Browser integration tests are hard because of timing. Manual testing is usually easier.

---

## Performance Notes

Don't worry about Rust performance. The bottlenecks are:
1. Chrome startup (~1-2 seconds)
2. Network requests
3. JavaScript execution

Rust code is negligible. Optimize for readability.

---

## Security

- All auth happens in Chrome (we never see passwords)
- No credential storage
- Stealth mode hides automation but doesn't do anything sketchy
- Profile directory is user-owned (`~/.curlup/` if used)
