CurlUp Developer Guide (Headless Chrome Edition)

A terminal-first, text-only browser powered by real web rendering

1. User Flow (Start to Finish)

This section describes exactly what happens from the moment a user runs CurlUp to the moment they read content in the terminal.

1.1 First Run (No Existing Session)

User runs:

curlup https://mail.google.com


CurlUp:

Detects no existing browser session

Launches Chrome in headless or visible mode

Uses a persistent user data directory (~/.curlup/chrome-profile)

Chrome navigates to the URL.

User is prompted:

Please complete login in the browser window.
Press ENTER when finished.


User logs in normally (OAuth, 2FA, etc.).

CurlUp does not handle credentials

Chrome stores cookies/session state

User presses ENTER.

CurlUp:

Extracts the fully-rendered DOM

Converts visible content to structured text

Renders it to the terminal

1.2 Subsequent Runs (Session Reuse)

User runs:

curlup https://mail.google.com


CurlUp:

Reuses the existing Chrome profile

Session is already authenticated

Page loads immediately

DOM is extracted and rendered as text.

No login required.

1.3 Key Guarantees

CurlUp never sees passwords

CurlUp never reverse-engineers APIs

CurlUp behaves like a real browser

If Chrome can render it, CurlUp can read it

2. High-Level Architecture

CurlUp is a browser orchestrator + text renderer, not a browser implementation.

┌─────────────┐
│     CLI     │  (arguments, commands)
└──────┬──────┘
       │
┌──────▼──────┐
│   Runtime   │  (tokio, lifecycle)
└──────┬──────┘
       │
┌──────▼─────────┐
│ Browser Driver │  (WebDriver / Chrome)
└──────┬─────────┘
       │
┌──────▼─────────┐
│ DOM Extractor  │  (JS executed in page)
└──────┬─────────┘
       │
┌──────▼─────────┐
│ Text Renderer  │  (terminal output)
└────────────────┘


Everything upstream of Chrome is Rust.
Everything downstream of Chrome is text.

3. Core Design Principle

Chrome owns correctness. Rust owns control.

Rust does:

Lifecycle

Configuration

Data flow

Error handling

Chrome does:

JavaScript

DOM mutation

CSS resolution

Auth

Cookies

Security

4. Crate Selection
[dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
clap = { version = "4.5", features = ["derive"] }
fantoccini = "0.19"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
anyhow = "1.0"

Why these crates
Crate	Purpose
fantoccini	WebDriver client
tokio	Async orchestration
clap	CLI parsing
serde_json	JS ↔ Rust data exchange
thiserror	Typed internal errors
anyhow	Top-level ergonomics
5. Directory Layout
curlup/
├── Cargo.toml
└── src/
    ├── main.rs
    ├── cli.rs
    ├── browser/
    │   ├── mod.rs
    │   ├── chrome.rs
    │   └── session.rs
    ├── dom/
    │   ├── mod.rs
    │   └── extract.rs
    ├── render/
    │   └── text.rs
    └── error.rs


Each directory corresponds to a conceptual primitive, not a feature.

6. CLI Primitive
cli.rs
use clap::Parser;

#[derive(Parser)]
pub struct Cli {
    pub url: String,

    #[arg(long)]
    pub visible: bool,
}


--visible launches Chrome non-headless (useful for login/debug)

7. Browser Lifecycle (Chrome + WebDriver)
7.1 Chrome Expectations

CurlUp assumes:

chromedriver is installed

Compatible Chrome is available

Chrome is launched outside Rust (simplest, most stable).

7.2 Connecting via WebDriver
browser/chrome.rs
use fantoccini::Client;

pub async fn connect() -> Result<Client, fantoccini::error::CmdError> {
    Client::new("http://localhost:9515").await
}


Chrome should be launched like:

chromedriver --port=9515 \
  --user-data-dir=$HOME/.curlup/chrome-profile

8. Page Loading and Readiness
Navigate and wait for JS
client.goto(url).await?;
client.wait().for_element(fantoccini::Locator::Css("body")).await?;


This ensures:

JS executed

DOM populated

Page usable

9. DOM Extraction (Critical Section)

This is the most important abstraction in CurlUp.

Principle

DOM traversal happens inside the browser

Rust receives clean, serialized data

dom/extract.rs
use serde::Deserialize;

#[derive(Deserialize)]
pub struct DomText {
    pub blocks: Vec<String>,
}

pub async fn extract_text(client: &fantoccini::Client) -> anyhow::Result<Vec<String>> {
    let script = r#"
        (() => {
            const elements = document.querySelectorAll(
              'h1,h2,h3,p,li,article,section'
            );

            return {
                blocks: Array.from(elements)
                  .map(e => e.innerText.trim())
                  .filter(t => t.length > 0)
            };
        })();
    "#;

    let result = client.execute(script, vec![]).await?;
    let dom: DomText = serde_json::from_value(result)?;
    Ok(dom.blocks)
}

Why this works

Uses browser’s DOM APIs

Respects JS mutations

Ignores hidden content

Zero HTML parsing in Rust

10. Text Rendering
render/text.rs
pub fn render(lines: &[String]) {
    for line in lines {
        println!("{line}\n");
    }
}


No styling yet. That comes later.

11. main.rs (System Wiring)
use anyhow::Result;
use clap::Parser;

mod cli;
mod browser;
mod dom;
mod render;

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::Cli::parse();

    let client = browser::chrome::connect().await?;
    client.goto(&args.url).await?;

    let text = dom::extract::extract_text(&client).await?;
    render::text::render(&text);

    Ok(())
}


This is intentionally boring.
Boring code survives.

12. Security Model (Brief, Explicit)

All auth handled by Chrome

CurlUp never inspects cookies

Profile directory is user-owned

No credential handling in Rust

That’s it.

13. What CurlUp Is (and Is Not)
CurlUp IS

A terminal reader for real web apps

A browser orchestrator

A JS-aware content extractor

CurlUp IS NOT

A browser engine

A Gmail client

A scraping framework

14. Extension Points (Future)

This architecture cleanly supports:

Link navigation

Keyboard interaction

Scroll regions

Site-specific extractors (Gmail, GitHub)

TUI rendering (ratatui)

Read-only automation

All without rethinking fundamentals.

15. Mental Model for Developers

If you remember only one thing:

Below is an **Appendix: Rust Language and Engineering Practices for CurlUp**.
This is meant to be read *after* the main guide and focuses exclusively on **Rust-specific design decisions, idioms, and best practices** used in CurlUp. It explains *why* things are structured the way they are and how to extend them correctly.

---

# Appendix A — Rust Language Guide for CurlUp

---

## A.1 Rust’s Role in CurlUp

Rust is **not** used to understand the web. Rust is used to:

* Coordinate asynchronous systems
* Enforce correctness at compile time
* Make illegal states unrepresentable
* Own lifecycle, errors, and data flow

Chrome is the “oracle of truth.”
Rust is the control plane.

This framing is important: **Rust code should stay boring**.

---

## A.2 Crate Boundaries and Module Design

### Rule: One concept per module

Each module in CurlUp corresponds to a *conceptual boundary*, not a feature.

| Module    | Responsibility          |
| --------- | ----------------------- |
| `cli`     | User intent             |
| `browser` | External system control |
| `dom`     | Data extraction         |
| `render`  | Output                  |
| `error`   | Failure semantics       |

If a module grows beyond ~300 lines, split it by *responsibility*, not type.

---

## A.3 Ownership and Borrowing Strategy

CurlUp deliberately avoids exposing lifetimes in public APIs.

### Design rules

* Own `String`, not `&str`, at boundaries
* Borrow only inside functions
* Clone small data freely (clarity > micro-optimizations)

### Example: DOM extraction

```rust
pub async fn extract_text(client: &Client) -> Result<Vec<String>>
```

* `Client` is borrowed (long-lived)
* Returned text is **owned**
* Caller controls lifetime

This avoids:

* Lifetime propagation
* Tying data to browser lifetime
* Accidental use-after-close bugs

---

## A.4 Error Handling Philosophy

### Internal: Typed errors

### External: Erased errors

This is intentional.

---

### A.4.1 Internal Errors (`thiserror`)

Use enums to model **distinct failure modes**.

```rust
#[derive(thiserror::Error, Debug)]
pub enum BrowserError {
    #[error("failed to connect to WebDriver")]
    Connection,

    #[error("navigation failed")]
    Navigation,

    #[error("DOM extraction failed")]
    Dom,
}
```

Typed errors answer the question:

> *What kind of failure occurred?*

---

### A.4.2 Application Boundary (`anyhow`)

At `main()`:

```rust
fn main() -> anyhow::Result<()> { ... }
```

This answers:

> *Did the app succeed or fail?*

Details still exist (via error chain), but **control flow stays simple**.

---

### A.4.3 Never Use

* `unwrap()`
* `expect()`
* panics for control flow

If something “cannot fail,” encode it in the type system.

---

## A.5 Async Rust: Practical Guidelines

CurlUp uses async **only** because:

* WebDriver is networked
* Chrome is slow
* Blocking would degrade UX

### Rules for Async in CurlUp

1. **Async at I/O boundaries only**
2. No async traits
3. No boxed futures
4. No manual `Pin`

If async complexity appears, architecture is wrong.

---

### A.5.1 Tokio Runtime

```rust
#[tokio::main]
async fn main()
```

* Multi-threaded runtime
* Default scheduler
* No custom executors

This is the “don’t be clever” configuration.

---

## A.6 Data Exchange: JavaScript ↔ Rust

This is a critical seam.

### Principle

> JavaScript returns **plain JSON data**.
> Rust deserializes into **strong types**.

---

### Example

JavaScript:

```javascript
return {
  blocks: ["Inbox", "Unread mail", "Subject line"]
};
```

Rust:

```rust
#[derive(Deserialize)]
struct DomText {
    blocks: Vec<String>,
}
```

Benefits:

* Schema validation
* No string parsing
* Compiler enforces correctness

Never parse ad-hoc JS strings in Rust.

---

## A.7 Trait Usage (Minimal, Strategic)

Traits are used **only** to abstract system boundaries.

### Example: Page Source

```rust
#[async_trait]
pub trait PageSource {
    async fn load(&self, url: &str) -> Result<Vec<String>>;
}
```

Implementations:

* `BrowserSource`
* (Future) `StaticSource`

Traits are **not** used for:

* “Code reuse”
* Polymorphism for its own sake
* Modeling domain objects

If there is only one implementation, do not introduce a trait.

---

## A.8 Struct Design Rules

### Prefer Plain Data

```rust
struct Page {
    url: String,
    blocks: Vec<String>,
}
```

Avoid:

* Smart pointers in structs
* Interior mutability
* Generic-heavy types

---

### Immutability by Default

Most structs should be immutable after creation.

Mutation:

* Happens in small scopes
* Is obvious
* Is localized

---

## A.9 Logging and Diagnostics

CurlUp should use **structured logging**, but sparingly.

Recommended:

* `tracing`
* Log lifecycle events only

Examples:

* Browser started
* Page loaded
* DOM extracted
* Rendering complete

Avoid logging:

* DOM content
* Cookies
* User data

---

## A.10 Testing Strategy in Rust

### Unit tests first

Test:

* DOM extraction JS snippets
* Rust-side deserialization
* Renderer formatting

Avoid:

* Browser-based tests initially
* Flaky timing-dependent tests

---

### Example Test

```rust
#[test]
fn deserialize_dom_text() {
    let json = r#"{ "blocks": ["Hello", "World"] }"#;
    let parsed: DomText = serde_json::from_str(json).unwrap();
    assert_eq!(parsed.blocks.len(), 2);
}
```

Browser integration tests come later.

---

## A.11 Performance Model (Don’t Overthink It)

CurlUp performance is dominated by:

* Chrome startup
* Network latency
* JS execution

Rust-side performance is irrelevant by comparison.

Therefore:

* Favor clarity
* Favor allocation
* Favor copying

Optimize only when Chrome is no longer the bottleneck (unlikely).

---

## A.12 Unsafe Rust Policy

**No `unsafe` is allowed** in CurlUp core.

If unsafe becomes “necessary,” it means:

* Wrong abstraction
* Wrong crate
* Wrong approach

---

## A.13 Versioning and Stability

Rust code should be:

* Conservative
* Explicit
* Boring to read

Breaking changes are acceptable early, but **interfaces should feel stable** even before they are.

---

## A.14 How to Think While Writing CurlUp Rust Code

Before writing any Rust code, ask:

1. Does Chrome already do this better?
2. Can this be expressed as data instead of logic?
3. Can this be a pure function?
4. Can this be owned instead of borrowed?
5. Will this still make sense in 6 months?

If the answer feels fuzzy, stop and simplify.

---

## A.15 Final Guiding Principle

> CurlUp succeeds not because it is clever,
> but because it refuses to be.

Rust enforces that discipline—if you let it.

---
