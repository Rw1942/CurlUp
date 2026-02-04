# Architecture Overview

## Purpose
CurlUp is a browser orchestrator + text renderer. Chrome renders pages; Rust coordinates lifecycle, extraction, and output.

## High-Level Flow
1. Launch or connect to ChromeDriver.
2. Navigate to the target URL and wait for readiness.
3. Extract DOM text and links inside the browser.
4. Parse and format text into structured terminal output.
5. Render content and interactive link controls.

## Module Boundaries

```
src/
├── browse.rs          # Interactive browsing loop
├── cli.rs             # CLI parsing
├── picker.rs          # Start screen and curated sites
├── term.rs            # Terminal helpers (screen/layout)
├── browser/
│   ├── chrome.rs      # WebDriver connection
│   ├── driver.rs      # ChromeDriver process lifecycle
│   └── navigation.rs  # Page navigation + readiness + scroll
├── dom/
│   ├── content.rs     # Link + PageContent types
│   └── extract.rs     # DOM extraction (text + links)
└── render/
    └── text.rs        # Render API + parsing pipeline
        └── text/      # Submodules (links, util, parsing helpers)
```

## Key Design Choices
- **Chrome is the source of truth**: no HTML parsing in Rust.
- **Small modules**: each module maps to a clear concept.
- **Explicit errors**: prefer `Result` and typed errors for internal flow.
- **Terminal-first UX**: rendering and controls stay readable and consistent.
