# Architecture

## Core Idea

CurlUp orchestrates Chrome to render pages, then extracts the text. Chrome handles JavaScript, auth, cookies. Rust handles lifecycle and output.

## Data Flow

```
User runs curlup
       │
       ▼
Spawn ChromeDriver (port 9515)
       │
       ▼
Connect via WebDriver protocol
       │
       ▼
Navigate + wait for page ready
       │
       ▼
Inject stealth JS (hide automation)
       │
       ▼
Scroll to trigger lazy content
       │
       ▼
Extract text + links
       │
       ▼
Render to terminal
```

## Module Map

| Module | Job |
|--------|-----|
| `cli` | Parse arguments |
| `browser/driver` | Start/stop ChromeDriver |
| `browser/chrome` | Connect, configure stealth |
| `browser/navigation` | Load pages, wait for ready |
| `dom/extract` | Standard innerText extraction |
| `dom/multilens` | Smart content extraction |
| `render/text` | Terminal formatting |
| `browse` | Interactive loop |
| `picker` | Start screen menu |

## Extraction Modes

**Standard:** Single JS call gets `document.body.innerText`. Fast, simple.

**Multi-lens (`-m`):** Fetches full HTML, parses in Rust, runs 4 strategies, picks best result.

## Design Principles

1. **Chrome is truth** - Don't reimplement browser features
2. **Small modules** - One concept per file
3. **Explicit errors** - Use `Result`, no panics
4. **Terminal-first** - Clean, readable output
