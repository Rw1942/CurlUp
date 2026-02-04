# CurlUp Compatible Sites

CurlUp has been tested against these websites. The rating indicates how well the terminal formatting works.

## Rating Key

- ★★★★★ Excellent - Clean, well-structured output
- ★★★★☆ Good - Minor formatting issues
- ★★★☆☆ Fair - Readable but some noise
- ★★☆☆☆ Poor - Significant issues
- ✗ Blocked - Site blocks automated browsers
- 🔐 Requires Auth - Needs login to access content

---

## News Sites

| Site | Rating | Notes |
|------|--------|-------|
| news.google.com | ★★★★★ | Weather, sections, headlines, timestamps |
| bbc.com/news | ★★★★☆ | Good structure, live indicators work |
| cnn.com | ★★★★☆ | Live updates, sections detected |
| nytimes.com | ★★★★☆ | Headlines and timestamps clean |
| theguardian.com | ★★★★☆ | Section headers work well |
| politico.com | ★★★★☆ | News items formatted correctly |
| axios.com | ★★★★☆ | Clean bullet-point style |
| npr.org | ★★★★☆ | Headlines, sections, audio content links |
| reuters.com | ★★★★☆ | Headlines, market tickers, breaking news |

## Tech News

| Site | Rating | Notes |
|------|--------|-------|
| news.ycombinator.com | ★★★★★ | Numbered lists with points/comments |
| arstechnica.com | ★★★★★ | Headlines + descriptions + author/time |
| techcrunch.com | ★★★★☆ | Categories and timestamps work |
| theverge.com | ★★★☆☆ | Some metadata parsing issues |
| wired.com | ★★★★☆ | Article format works well |
| lobste.rs | ★★★★☆ | Similar to HN, tags visible |
| dev.to | ★★★★☆ | Blog posts, tags, community highlights |

## Developer Tools & Documentation

| Site | Rating | Notes |
|------|--------|-------|
| github.com/trending | ★★★★☆ | Repo names, descriptions, stars |
| github.com (repos) | ★★★★☆ | README content extracts well |
| stackoverflow.com | ★★★☆☆ | Questions readable, answers verbose |
| developer.mozilla.org | ★★★★★ | Excellent docs structure, navigation links |
| docs.python.org | ★★★★★ | Clean documentation, version lists, sections |

## Sports

| Site | Rating | Notes |
|------|--------|-------|
| espn.com | ★★★★☆ | Headlines, scores visible |

## Weather

| Site | Rating | Notes |
|------|--------|-------|
| wttr.in | ★★★★★ | ASCII art preserved perfectly |
| weather.com | ★★★☆☆ | Basic info extracts |

## Reference & Dictionary

| Site | Rating | Notes |
|------|--------|-------|
| en.wikipedia.org | ★★★☆☆ | Content extracts, infobox messy |
| merriam-webster.com | ★★★★★ | Word of day, definitions, trending words |

## Finance

| Site | Rating | Notes |
|------|--------|-------|
| finance.yahoo.com | ★★★★☆ | Stock quotes, headlines, market sections |

## Entertainment & Media

| Site | Rating | Notes |
|------|--------|-------|
| imdb.com | ★★★★☆ | Movie lists, ratings, cast info |
| rottentomatoes.com | ★★★★☆ | Movie reviews, scores, categories |
| goodreads.com | ★★★★☆ | Book lists, genres, quotes, awards |

## Recipes & Food

| Site | Rating | Notes |
|------|--------|-------|
| allrecipes.com | ★★★★☆ | Recipe titles, categories, trending articles |

## Jobs & Careers

| Site | Rating | Notes |
|------|--------|-------|
| indeed.com | ★★★★☆ | Job listings, filters, company info |

## E-commerce

| Site | Rating | Notes |
|------|--------|-------|
| amazon.com | ★★★☆☆ | Product categories visible, somewhat noisy |

## Classifieds

| Site | Rating | Notes |
|------|--------|-------|
| craigslist.org | ★★★★★ | Clean text-based structure, categories |

## Search Engines

| Site | Rating | Notes |
|------|--------|-------|
| duckduckgo.com | ★★★★☆ | Clean search interface |

## Social & Community

| Site | Rating | Notes |
|------|--------|-------|
| reddit.com | ★★★☆☆ | Posts visible, rules/sidebar clean, some noise |

## Blocked Sites

These sites block headless/automated browsers (Cloudflare protection):

| Site | Status |
|------|--------|
| producthunt.com | ✗ Blocked |

## Sites Requiring Authentication

These sites work but require login to see content (use `-v` flag for visible mode):

| Site | Status | Notes |
|------|--------|-------|
| x.com (Twitter) | 🔐 Requires Auth | Shows login/signup wall |
| linkedin.com | 🔐 Requires Auth | Redirects to login page |
| quora.com | 🔐 Requires Auth | Shows login wall |
| medium.com | 🔐 Requires Auth | Limited content without login |

---

## Tips for Best Results

1. **Use `-v` flag** for sites that require login or have bot detection
2. **News aggregators** (Google News, HN) work best
3. **Tech blogs** and **documentation sites** with consistent formatting work well
4. **Dynamic SPAs** may need the page to fully load
5. **Reference sites** (dictionaries, docs) have excellent structure

## Adding New Sites

If a site doesn't format well, you can:
1. Use `-r` (raw mode) for unformatted output
2. Pipe to `grep` or other tools for filtering
3. Open an issue to request site-specific parsing

---

*Last updated: February 2026*
*Tested with CurlUp v0.1.0*
