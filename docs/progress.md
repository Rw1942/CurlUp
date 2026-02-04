# Development Progress

## Current: v0.2.0

CurlUp is fully functional with interactive browsing and smart content extraction.

## What's Working

- **Interactive browsing** - Navigate with numbered links, back button, history
- **Multi-lens extraction** - Four strategies for smart content detection
- **Stealth mode** - Anti-detection enabled by default
- **Single-page mode** - Pipe output to other tools
- **Site picker** - Start screen with curated sites
- **ChromeDriver auto-launch** - No manual setup needed

## Recent Changes

### v0.2.0
- Added multi-lens extraction (`-m` flag)
- CSS selector, DOM traversal, CETD, ARIA strategies
- Scoring and fallback system
- New dependencies: scraper, dom-content-extraction

### v0.1.0
- Interactive browsing with numbered links
- Back navigation and history
- Stealth mode with realistic browser fingerprint
- Start screen with popular sites
- Link extraction and display
- Terminal-optimized rendering

## File Structure

```
src/
├── main.rs          # Entry point
├── cli.rs           # CLI parsing
├── browse.rs        # Interactive loop
├── picker.rs        # Start screen
├── term.rs          # Terminal helpers
├── browser/         # Chrome/WebDriver control
├── dom/             # Content extraction
│   └── multilens/   # Smart extraction
└── render/          # Terminal output
```

## Next Up

- [ ] Session persistence for logged-in sites
- [ ] Site-specific extractors
- [ ] Configurable timeouts
