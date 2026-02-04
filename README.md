# CurlUp

A terminal web browser. CurlUp uses Chrome to render pages and shows you the text with numbered links.

## First Time Setup

You need three things: **Chrome**, **ChromeDriver**, and **Rust**.

### Step 1: Make sure Chrome is installed

You probably already have it. If not, download from https://google.com/chrome

### Step 2: Install ChromeDriver

ChromeDriver lets CurlUp control Chrome. It must match your Chrome version.

**macOS:**
```bash
brew install chromedriver

# Fix the quarantine warning (required on macOS)
xattr -d com.apple.quarantine $(which chromedriver)
```

**Linux (Debian/Ubuntu):**
```bash
sudo apt install chromium-chromedriver
```

**Manual install (any OS):**
1. Check your Chrome version: `Chrome menu > About Google Chrome`
2. Download matching ChromeDriver from https://googlechromelabs.github.io/chrome-for-testing/
3. Put it in `~/.local/bin/` or somewhere in your PATH

### Step 3: Install Rust (if you don't have it)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### Step 4: Build CurlUp

```bash
git clone https://github.com/Rw1942/CurlUp.git
cd CurlUp
cargo build --release
```

### Step 5: Run it

```bash
# From the project directory:
./target/release/curlup

# Or copy to your PATH for global access:
cp target/release/curlup ~/.local/bin/
curlup
```

### Verify it works

```bash
curlup news.ycombinator.com
```

You should see Hacker News rendered as text with numbered links. Type `1` to follow the first link, `b` to go back, `q` to quit.

---

## Quick Reference

```bash
curlup                          # Start with site picker
curlup news.ycombinator.com     # Go directly to a site
curlup -m medium.com/article    # Multi-lens mode (cleaner articles)
curlup -N medium.com/article    # Full page (disable auto-condensed)
curlup -v mail.google.com       # Visible browser (for login)
```

### While Browsing

The command bar shows where `b` (back) and `f` (forward) will take you. Type a command and press **Enter** to execute it.

| Command | What it does |
|---------|--------------|
| `1-999` | Follow that link number |
| `b` | Go back (shows destination) |
| `f` | Go forward (shows destination) |
| `t` | Scroll to top of page |
| `h` or `0` | Return to site picker |
| `C` | Toggle condensed/reader mode |
| `r` | Refresh the page |
| `u` | Show current URL |
| `q` | Quit CurlUp |
| `help` | Show help |
| `google.com` | Navigate to any URL |

**Scrolling:** Use `t` to jump to top, or use terminal scrollback (mouse wheel, trackpad, Shift+PageUp/Down).

**Example:** To follow link 5, type `5` then press Enter. Type `b` to go back (the footer shows where you'll go).

### All Options

```
curlup [OPTIONS] [URL]

  -N, --no-condensed Disable condensed mode (show full page for articles)
  -m, --multilens    Smart extraction for articles
  -v, --visible      Show the browser window
  -u, --user-agent   Custom user-agent string
      --no-stealth   Disable anti-bot measures
      --no-focus     Show full page including navigation, ads
  -h, --help         Show help
```

---

## Common Issues

### "ChromeDriver not found"

CurlUp looks for ChromeDriver in:
1. `~/.local/bin/chromedriver`
2. Anywhere in your PATH

Check with:
```bash
which chromedriver
```

### "ChromeDriver version mismatch"

Your ChromeDriver version must match Chrome. Check your Chrome version (`Chrome > About`) and download the matching driver from https://googlechromelabs.github.io/chrome-for-testing/

### "Operation not permitted" (macOS)

Remove the quarantine flag:
```bash
xattr -d com.apple.quarantine $(which chromedriver)
```

### ChromeDriver won't start

Kill any stuck processes and try again:
```bash
pkill chromedriver
curlup example.com
```

### Page content looks wrong

Try multi-lens mode for cleaner extraction:
```bash
curlup -m thesite.com
```

Or use visible mode to see what's happening:
```bash
curlup -v thesite.com
```

---

## Condensed Mode

Condensed mode automatically activates when CurlUp detects article content, extracting just the article using Mozilla's Readability algorithm (same tech behind Firefox Reader View):

- Strips navigation, ads, sidebars, and clutter
- Shows article title, author, and estimated read time
- Preserves links within the article
- Auto-enabled when pages have readable article content

```bash
curlup medium.com/@user/some-article      # Auto-enters condensed mode
curlup -N medium.com/@user/some-article   # Force full page view
```

Press `C` while browsing to toggle between condensed and full page view.

Condensed mode works best on:
- News articles (BBC, NYT, Reuters)
- Blog posts (Medium, dev.to, personal blogs)
- Documentation pages
- Wikipedia articles

If a page doesn't have extractable article content, CurlUp will show the full page.

---

## Multi-Lens Mode

Use `-m` for articles, blog posts, and documentation. It runs multiple extraction strategies and picks the best result:

- CSS selectors (finds `<article>`, `<main>`, etc.)
- DOM traversal (skips nav, footer, sidebars)
- Text density analysis (finds content-heavy blocks)
- ARIA landmarks (uses accessibility markup)

```bash
curlup -m medium.com/@user/some-article
```

---

## Sites That Work Well

- **News:** Hacker News, Google News, BBC, NPR, Reuters
- **Tech:** GitHub, Ars Technica, TechCrunch, dev.to
- **Docs:** MDN, Python docs, Read the Docs
- **Reference:** Wikipedia, IMDb, Merriam-Webster
- **Weather:** wttr.in (ASCII art preserved)

**Needs login (use `-v`):** Gmail, LinkedIn, Medium, Twitter

**Blocked:** Product Hunt (Cloudflare)

---

## Development

```bash
cargo run -- example.com     # Run in dev mode
cargo test                   # Run tests
cargo clippy                 # Lint
```

See [docs/](docs/) for architecture details.

---

Starting CurlUp:

<img width="681" height="674" alt="image" src="https://github.com/user-attachments/assets/5c892291-f61d-40ca-8f6f-266d3c661505" />

Opening Links
<img width="660" height="678" alt="Screenshot 2026-02-04 at 3 01 07 PM" src="https://github.com/user-attachments/assets/720a212e-cb45-490a-b181-43e881ede81c" />

Opening pages in Reader mode
<img width="1034" height="643" alt="Screenshot 2026-02-04 at 3 01 43 PM" src="https://github.com/user-attachments/assets/08d453bf-8ae4-4050-b7a1-33f1a1623fe3" />

## License

MIT

## Credits

Built with [fantoccini](https://github.com/jonhoo/fantoccini), [scraper](https://github.com/rust-scraper/scraper), [dom_smoothie](https://github.com/niklak/dom_smoothie) (condensed mode), [html2text](https://github.com/jugglerchris/rust-html2text), [tokio](https://tokio.rs/), [clap](https://clap.rs/).
