//! DOM content extraction from rendered pages.
//!
//! Extracts:
//! - Full rendered HTML for html2text rendering
//! - Meaningful links from the page for navigation

use anyhow::Result;
use fantoccini::Client;
use serde::Deserialize;

use super::content::{Link, PageContent};
use super::filter::clean_html;
use super::multilens::snapshot::fetch_rendered_html;

#[derive(Debug, Deserialize)]
struct RawLink {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    href: Option<String>,
}

/// JavaScript that extracts links from the page.
const EXTRACT_LINKS_JS: &str = r#"
return (function() {
    var result = [];
    
    // Elements to skip (navigation, headers, footers, etc.)
    var skipSelectors = 'nav, header, footer, aside, [role="navigation"], [role="banner"], [role="contentinfo"], [aria-hidden="true"]';
    
    // Extract meaningful links
    var seen = {};
    document.querySelectorAll('a[href]').forEach(function(a) {
        var href = a.href;
        var text = (a.innerText || '').trim();
        
        if (text.length < 8 || text.length > 200) return;
        if (href.indexOf('javascript:') === 0) return;
        if (href === window.location.href) return;
        if (href.indexOf('#') === href.length - 1) return;
        if (seen[href]) return;
        if (a.closest(skipSelectors)) return;
        
        seen[href] = true;
        result.push({ text: text, href: href });
    });
    
    return result;
})();
"#;

/// Extract page content (HTML and links) from the browser.
///
/// If `focus` is true, applies content filtering to remove noise elements.
pub async fn extract_page_content(client: &Client, current_url: &str, focus: bool) -> Result<PageContent> {
    // Fetch rendered HTML for html2text rendering
    let raw_html = fetch_rendered_html(client).await?;
    
    // Apply content filtering if focus mode is enabled
    let html = if focus {
        clean_html(&raw_html)
    } else {
        raw_html
    };
    
    // Extract links separately
    let result = client.execute(EXTRACT_LINKS_JS, vec![]).await?;
    let raw_links: Vec<RawLink> = serde_json::from_value(result).unwrap_or_default();

    // Convert raw links to typed Links
    let links = raw_links
        .into_iter()
        .filter_map(|l| match (l.text, l.href) {
            (Some(text), Some(href)) if !text.is_empty() && !href.is_empty() => {
                Some(Link { text, href })
            }
            _ => None,
        })
        .collect();

    Ok(PageContent::new(current_url.to_string(), html, links))
}
