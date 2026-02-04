//! DOM content extraction from rendered pages.
//!
//! Extraction is done in a single WebDriver call that retrieves:
//! - Text from block-level HTML elements (paragraphs, headings, list items)
//! - All meaningful links from the page
//!
//! By extracting from individual elements using textContent (not innerText),
//! we get raw text without CSS-imposed line breaks, enabling proper rewrapping
//! to any terminal width.

use anyhow::Result;
use fantoccini::Client;
use serde::Deserialize;

use super::content::{Link, PageContent};

/// Raw extraction result from the consolidated JavaScript call.
#[derive(Debug, Deserialize, Default)]
struct ExtractResult {
    /// Text paragraphs extracted from block-level elements
    #[serde(default)]
    paragraphs: Vec<String>,
    /// Extracted links
    #[serde(default)]
    links: Vec<RawLink>,
}

#[derive(Debug, Deserialize)]
struct RawLink {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    href: Option<String>,
}

/// JavaScript that extracts text from block elements and links in one call.
/// 
/// Uses textContent instead of innerText to get raw text without CSS line breaks.
/// This enables proper rewrapping to terminal width.
const EXTRACT_JS: &str = r#"
return (function() {
    var result = { paragraphs: [], links: [] };
    
    // Block-level elements that contain content
    var blockSelectors = 'p, h1, h2, h3, h4, h5, h6, li, blockquote, figcaption, pre, td, th, dt, dd';
    
    // Elements to skip (navigation, headers, footers, etc.)
    var skipSelectors = 'nav, header, footer, aside, [role="navigation"], [role="banner"], [role="contentinfo"], [aria-hidden="true"]';
    
    // Extract text from each block element
    document.querySelectorAll(blockSelectors).forEach(function(el) {
        // Skip if inside navigation/header/footer/aside
        if (el.closest(skipSelectors)) return;
        
        // Skip hidden elements
        var style = window.getComputedStyle(el);
        if (style.display === 'none' || style.visibility === 'hidden') return;
        
        // Get raw textContent (no CSS line breaks)
        var text = (el.textContent || '').trim();
        
        // Skip very short or empty text
        if (text.length < 3) return;
        
        // Add paragraph with tag hint for headings
        var tag = el.tagName.toLowerCase();
        if (tag.match(/^h[1-6]$/)) {
            result.paragraphs.push('__heading__' + text);
        } else {
            result.paragraphs.push(text);
        }
    });
    
    // Extract meaningful links
    var seen = {};
    document.querySelectorAll('a[href]').forEach(function(a) {
        var href = a.href;
        var text = (a.innerText || '').trim();
        
        // Skip short or overly long text
        if (text.length < 8 || text.length > 200) return;
        
        // Skip javascript: and anchor-only links
        if (href.indexOf('javascript:') === 0) return;
        if (href === window.location.href) return;
        if (href.indexOf('#') === href.length - 1) return;
        
        // Skip duplicates
        if (seen[href]) return;
        
        // Skip if inside nav/header/footer
        if (a.closest(skipSelectors)) return;
        
        seen[href] = true;
        result.links.push({ text: text, href: href });
    });
    
    return result;
})();
"#;

/// Extract page content (text and links) in a single WebDriver call.
pub async fn extract_page_content(client: &Client, current_url: &str) -> Result<PageContent> {
    // Single consolidated extraction call
    let result = client.execute(EXTRACT_JS, vec![]).await?;
    let extracted: ExtractResult = serde_json::from_value(result).unwrap_or_default();

    // Process paragraphs into clean lines
    let lines = clean_paragraphs(&extracted.paragraphs);

    // Convert raw links to typed Links
    let links = extracted
        .links
        .into_iter()
        .filter_map(|l| match (l.text, l.href) {
            (Some(text), Some(href)) if !text.is_empty() && !href.is_empty() => {
                Some(Link { text, href })
            }
            _ => None,
        })
        .collect();

    Ok(PageContent::new(current_url.to_string(), lines, links))
}

// ============================================================================
// Text Cleanup
// ============================================================================

/// Clean and normalize extracted paragraphs.
/// 
/// Each paragraph is a separate block element from the page.
/// We normalize whitespace and filter junk, preserving paragraph boundaries.
fn clean_paragraphs(paragraphs: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    let mut prev_text = String::new();
    
    for para in paragraphs {
        // Check if this is a heading (marked by JS extraction)
        let is_heading = para.starts_with("__heading__");
        let text = if is_heading {
            para.strip_prefix("__heading__").unwrap_or(para)
        } else {
            para.as_str()
        };
        
        // Normalize whitespace (collapse multiple spaces/newlines to single space)
        let normalized = normalize_whitespace(text);
        
        // Skip empty, too short, or junk lines
        if normalized.is_empty() || normalized.len() < 3 || is_junk_line(&normalized) {
            continue;
        }
        
        // Skip exact duplicates of previous paragraph
        if normalized == prev_text {
            continue;
        }
        
        // Add blank line before headings for visual separation
        if is_heading && !result.is_empty() {
            result.push(String::new());
        }
        
        prev_text = normalized.clone();
        result.push(normalized);
    }
    
    result
}

/// Normalize whitespace within text.
/// Collapses multiple spaces, tabs, and newlines into single spaces.
fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
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
