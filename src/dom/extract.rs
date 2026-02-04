//! DOM content extraction using Readability on rendered HTML.
//!
//! Extraction pipeline:
//! 1. Chrome renders the page (JavaScript executes)
//! 2. Get the rendered HTML
//! 3. Try Readability algorithm to extract main content
//! 4. Fall back to body text if Readability returns sparse content
//! 5. Clean up extracted text
//!
//! This works for both static and dynamic pages.

use anyhow::Result;
use fantoccini::Client;
use readability::extractor;
use serde::Deserialize;
use std::io::Cursor;
use url::Url;

use super::content::{Link, PageContent};

/// Minimum lines for Readability result to be considered "good enough".
/// Below this threshold, we fall back to body text extraction.
const MIN_CONTENT_LINES: usize = 3;

#[derive(Debug, Deserialize)]
struct RawLink {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    href: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct LinkExtractResult {
    #[serde(default)]
    links: Option<Vec<RawLink>>,
}

/// Extract visible text content from the page.
pub async fn extract_text(client: &Client) -> Result<Vec<String>> {
    let url = client.current_url().await?;
    let content = extract_page_content(client, url.as_str()).await?;
    Ok(content.lines)
}

/// Extract page content including text and links.
pub async fn extract_page_content(client: &Client, current_url: &str) -> Result<PageContent> {
    // Step 1: Get rendered HTML
    let html = get_rendered_html(client).await?;

    // Step 2: Try Readability extraction
    let mut lines = extract_with_readability(&html, current_url);

    // Step 3: If Readability returned sparse content, fall back to body text
    // This handles sites like Hacker News that use table-based layouts
    if lines.len() < MIN_CONTENT_LINES {
        if let Ok(fallback) = extract_body_text(client).await {
            if fallback.len() > lines.len() {
                lines = fallback;
            }
        }
    }

    // Step 4: Clean up the extracted text
    let lines = clean_text(lines);

    // Step 5: Extract links
    let links = extract_links(client).await?;

    Ok(PageContent::new(current_url.to_string(), lines, links))
}

// ============================================================================
// HTML Extraction
// ============================================================================

/// Get rendered HTML from the DOM.
async fn get_rendered_html(client: &Client) -> Result<String> {
    let script = "return document.documentElement.outerHTML;";
    let result = client.execute(script, vec![]).await?;

    match result.as_str() {
        Some(html) => Ok(html.to_string()),
        None => Ok(client.source().await?),
    }
}

// ============================================================================
// Content Extraction
// ============================================================================

/// Extract main content using Mozilla's Readability algorithm.
/// Works well for article-style pages with semantic HTML.
fn extract_with_readability(html: &str, url_str: &str) -> Vec<String> {
    let url = match Url::parse(url_str) {
        Ok(u) => u,
        Err(_) => return vec!["[Invalid URL]".to_string()],
    };

    let mut cursor = Cursor::new(html.as_bytes());
    match extractor::extract(&mut cursor, &url) {
        Ok(product) => product
            .text
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        Err(_) => vec![],
    }
}

/// Fallback: extract all visible text from the page body.
/// Used when Readability fails (e.g., table-based layouts like Hacker News).
async fn extract_body_text(client: &Client) -> Result<Vec<String>> {
    let script = "return document.body ? document.body.innerText : '';";

    let result = client.execute(script, vec![]).await?;
    let text = result.as_str().unwrap_or("");

    Ok(text
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

// ============================================================================
// Text Cleanup
// ============================================================================

/// Clean up extracted text lines.
fn clean_text(lines: Vec<String>) -> Vec<String> {
    lines
        .into_iter()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed.len() >= 2 && !is_junk_line(trimmed)
        })
        .collect()
}

/// Check if a line is likely junk (boilerplate, not real content).
fn is_junk_line(line: &str) -> bool {
    // Lines that are just symbols or punctuation
    if line.chars().all(|c| !c.is_alphanumeric()) {
        return true;
    }

    // Common footer/boilerplate patterns
    let lower = line.to_lowercase();
    let junk_patterns = [
        "all rights reserved",
        "copyright ©",
        "privacy policy",
        "terms of service",
        "cookie policy",
        "powered by",
    ];

    junk_patterns.iter().any(|p| lower.contains(p))
}

// ============================================================================
// Link Extraction
// ============================================================================

/// Extract links via JavaScript.
/// Skips links inside nav/header/footer elements.
async fn extract_links(client: &Client) -> Result<Vec<Link>> {
    let script = r#"
        return (function() {
            if (!document.body) return { links: [] };
            
            var links = [];
            var seen = {};
            
            document.querySelectorAll('a[href]').forEach(function(a) {
                var href = a.href;
                var text = (a.innerText || '').trim();
                
                // Skip based on text length
                if (text.length < 3 || text.length > 200) return;
                
                // Skip javascript: and anchor-only links
                if (href.indexOf('javascript:') === 0) return;
                if (href === window.location.href) return;
                if (href.indexOf('#') === href.length - 1) return;
                
                // Skip duplicates
                if (seen[href]) return;
                
                // Skip if link is inside nav/header/footer
                var parent = a.closest('nav, header, footer, [role="navigation"]');
                if (parent) return;
                
                seen[href] = true;
                links.push({ text: text, href: href });
            });
            
            return { links: links };
        })();
    "#;

    let result = client.execute(script, vec![]).await?;

    if result.is_null() {
        return Ok(vec![]);
    }

    let extracted: LinkExtractResult = serde_json::from_value(result).unwrap_or_default();

    Ok(extracted
        .links
        .unwrap_or_default()
        .into_iter()
        .filter_map(|l| match (l.text, l.href) {
            (Some(text), Some(href)) if !text.is_empty() && !href.is_empty() => {
                Some(Link { text, href })
            }
            _ => None,
        })
        .collect())
}
