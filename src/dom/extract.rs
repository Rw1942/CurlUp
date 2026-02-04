//! DOM content extraction using Readability on rendered HTML.
//!
//! Extraction pipeline:
//! 1. Chrome renders the page (JavaScript executes)
//! 2. Remove hidden elements from DOM
//! 3. Get the rendered HTML
//! 4. Run Readability to extract main content
//! 5. Fall back to semantic tags if Readability returns sparse content
//! 6. Clean up extracted text
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
/// Below this threshold, we try semantic tag fallback.
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
    // Step 1: Remove hidden elements before extraction
    remove_hidden_elements(client).await?;

    // Step 2: Get rendered HTML
    let html = get_rendered_html(client).await?;

    // Step 3: Extract with Readability
    let mut lines = extract_with_readability(&html, current_url);

    // Step 4: If Readability returned sparse content, try semantic fallback
    if lines.len() < MIN_CONTENT_LINES {
        if let Ok(fallback) = extract_semantic_content(client).await {
            if fallback.len() >= lines.len() {
                lines = fallback;
            }
        }
    }

    // Step 5: Clean up the extracted text
    let lines = clean_text(lines);

    // Step 6: Extract links
    let links = extract_links(client).await?;

    Ok(PageContent::new(current_url.to_string(), lines, links))
}

// ============================================================================
// DOM Preparation
// ============================================================================

/// Remove hidden elements from the DOM before extraction.
/// This catches content that's rendered but not visible to users.
async fn remove_hidden_elements(client: &Client) -> Result<()> {
    let script = r#"
        (function() {
            // Remove elements with inline hidden styles
            var selectors = [
                '[style*="display: none"]',
                '[style*="display:none"]',
                '[style*="visibility: hidden"]',
                '[style*="visibility:hidden"]',
                '[hidden]',
                '[aria-hidden="true"]'
            ];
            
            selectors.forEach(function(sel) {
                document.querySelectorAll(sel).forEach(function(el) {
                    el.remove();
                });
            });
            
            // Remove elements hidden via computed style
            document.querySelectorAll('*').forEach(function(el) {
                var style = window.getComputedStyle(el);
                if (style.display === 'none' || style.visibility === 'hidden') {
                    el.remove();
                }
            });
        })();
    "#;

    let _ = client.execute(script, vec![]).await;
    Ok(())
}

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

/// Fallback: extract text from HTML5 semantic containers.
/// Tries <article>, <main>, then largest <section>.
async fn extract_semantic_content(client: &Client) -> Result<Vec<String>> {
    let script = r#"
        (function() {
            // Priority order: article > main > largest section
            var container = document.querySelector('article') 
                         || document.querySelector('main')
                         || document.querySelector('[role="main"]');
            
            // If no semantic container, find the section with most text
            if (!container) {
                var sections = document.querySelectorAll('section');
                var maxLen = 0;
                sections.forEach(function(s) {
                    var len = (s.innerText || '').length;
                    if (len > maxLen) {
                        maxLen = len;
                        container = s;
                    }
                });
            }
            
            // Last resort: body
            if (!container) {
                container = document.body;
            }
            
            return container ? (container.innerText || '') : '';
        })();
    "#;

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
        // Remove lines that are just punctuation or very short
        .filter(|line| {
            let trimmed = line.trim();
            trimmed.len() >= 2 && !is_junk_line(trimmed)
        })
        .collect()
}

/// Check if a line is likely junk (navigation remnants, etc).
fn is_junk_line(line: &str) -> bool {
    let lower = line.to_lowercase();

    // Lines that are just symbols or punctuation
    if line.chars().all(|c| !c.is_alphanumeric()) {
        return true;
    }

    // Common footer/boilerplate patterns
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
/// Filters out navigation-style links and duplicates.
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
