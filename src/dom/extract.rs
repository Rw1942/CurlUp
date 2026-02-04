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
use super::link_filter::filter_links;
use super::multilens::snapshot::fetch_rendered_html;

#[derive(Debug, Deserialize)]
struct RawLink {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    href: Option<String>,
}

/// JavaScript that extracts links from the main content area only.
/// Looks for main content containers (main, article, [role="main"]), 
/// falling back to body minus noise elements if not found.
const EXTRACT_LINKS_JS: &str = r#"
return (function() {
    var result = [];
    var skipSelectors = 'nav, header, footer, aside, [role="navigation"], [role="banner"], [role="contentinfo"], [aria-hidden="true"]';
    
    // Try to find main content container
    var mainContent = document.querySelector('main, article, [role="main"], .main-content, #main-content, .post-content, .article-content, .entry-content');
    
    // If no main content found, use body but skip noise
    var searchRoot = mainContent || document.body;
    
    searchRoot.querySelectorAll('a[href]').forEach(function(a) {
        // Skip links in noise areas
        if (a.closest(skipSelectors)) return;
        
        // Skip links that are inside nested nav-like containers even within main
        if (a.closest('.sidebar, .related-posts, .recommended, .trending, .comments')) return;
        
        var text = (a.innerText || '').trim();
        if (text) {
            result.push({ text: text, href: a.href });
        }
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

    // Extract links via JavaScript (gets absolute URLs)
    let result = client.execute(EXTRACT_LINKS_JS, vec![]).await?;
    let raw_links: Vec<RawLink> = serde_json::from_value(result).unwrap_or_default();

    // Convert to typed Links
    let links: Vec<Link> = raw_links
        .into_iter()
        .filter_map(|l| match (l.text, l.href) {
            (Some(text), Some(href)) if !text.is_empty() && !href.is_empty() => {
                Some(Link { text, href })
            }
            _ => None,
        })
        .collect();

    // Apply unified filtering
    let filtered = filter_links(links, Some(current_url));

    Ok(PageContent::new(current_url.to_string(), html, filtered))
}
