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
curlup -s example.com | less    # Single page mode (for piping)
curlup -m medium.com/article    # Multi-lens mode (cleaner articles)
curlup -v mail.google.com       # Visible browser (for login)
```

### While Browsing

| Key | What it does |
|-----|--------------|
| `1-20` | Follow that link |
| `b` | Go back |
| `r` | Refresh |
| `u` | Show current URL |
| `h` | Help |
| `q` | Quit |
| `google.com` | Go to any URL |

### All Options

```
curlup [OPTIONS] [URL]

  -s, --single       Fetch once and exit (good for piping)
  -r, --raw          Raw output, no formatting (implies -s)
  -m, --multilens    Smart extraction for articles
  -v, --visible      Show the browser window
  -u, --user-agent   Custom user-agent string
      --no-stealth   Disable anti-bot measures
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

## Multi-Lens Mode

Use `-m` for articles, blog posts, and documentation. It runs four extraction strategies and picks the best result:

- CSS selectors (finds `<article>`, `<main>`, etc.)
- DOM traversal (skips nav, footer, sidebars)
- Text density analysis (finds content-heavy blocks)
- ARIA landmarks (uses accessibility markup)

```bash
curlup -m medium.com/@user/some-article
curlup -m -s docs.python.org/3/tutorial | less
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

## License

MIT

## Credits

Built with [fantoccini](https://github.com/jonhoo/fantoccini), [scraper](https://github.com/rust-scraper/scraper), [dom-content-extraction](https://github.com/oiwn/dom-content-extraction), [tokio](https://tokio.rs/), [clap](https://clap.rs/).
