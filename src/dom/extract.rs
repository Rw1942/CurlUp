//! DOM content extraction from rendered pages.
//!
//! Extraction is done in a single WebDriver call that retrieves:
//! - All visible text via innerText
//! - All meaningful links from the page
//!
//! The text is then cleaned to remove boilerplate and junk lines.

use anyhow::Result;
use fantoccini::Client;
use serde::Deserialize;

use super::content::{Link, PageContent};

/// Raw extraction result from the consolidated JavaScript call.
#[derive(Debug, Deserialize, Default)]
struct ExtractResult {
    /// All visible text from document.body.innerText
    #[serde(default)]
    text: String,
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

/// JavaScript that extracts text and links in one call.
/// 
/// This consolidates what was previously 2-3 separate WebDriver calls
/// into a single round-trip for better performance.
const EXTRACT_JS: &str = r#"
return (function() {
    var result = { text: '', links: [] };
    
    // Get all visible text
    if (document.body) {
        result.text = document.body.innerText || '';
    }
    
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
        var parent = a.closest('nav, header, footer, [role="navigation"]');
        if (parent) return;
        
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

    // Process text into clean lines
    let lines = clean_text(&extracted.text);

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

/// Parse and clean text content into lines.
fn clean_text(text: &str) -> Vec<String> {
    text.lines()
        .map(|s| s.trim().to_string())
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && trimmed.len() >= 2 && !is_junk_line(trimmed)
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
