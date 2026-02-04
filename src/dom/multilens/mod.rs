//! Multi-lens content extraction system.
//!
//! This module fetches rendered HTML and extracts links using Rust-side
//! DOM parsing with the scraper crate.

pub mod snapshot;

use anyhow::Result;
use fantoccini::Client;
use scraper::Html;

use super::content::{Link, PageContent};
use super::filter::clean_html;

/// Extract page content using Rust-side DOM parsing.
///
/// Fetches the full rendered HTML and extracts links using scraper.
/// The HTML is passed to html2text for terminal rendering.
/// If `focus` is true, applies content filtering to remove noise elements.
pub async fn extract_multilens(client: &Client, url: &str, focus: bool) -> Result<PageContent> {
    // Get full rendered HTML
    let raw_html = snapshot::fetch_rendered_html(client).await?;
    
    // Apply content filtering if focus mode is enabled
    let html = if focus {
        clean_html(&raw_html)
    } else {
        raw_html
    };
    
    let doc = Html::parse_document(&html);

    // Extract links from the parsed document
    let links = extract_links(&doc);

    Ok(PageContent::new(url.to_string(), html, links))
}

/// Extract links from a parsed HTML document.
fn extract_links(doc: &Html) -> Vec<Link> {
    use scraper::Selector;

    let selector = match Selector::parse("a[href]") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let nav_selector = Selector::parse("nav, header, footer, [role=\"navigation\"]").ok();

    let mut seen = std::collections::HashSet::new();
    let mut links = Vec::new();

    for element in doc.select(&selector) {
        // Get href attribute
        let href = match element.value().attr("href") {
            Some(h) => h.to_string(),
            None => continue,
        };

        // Skip javascript: and anchor-only links
        if href.starts_with("javascript:") || href == "#" {
            continue;
        }

        // Get visible text
        let text: String = element.text().collect::<Vec<_>>().join(" ");
        let text = text.trim().to_string();

        // Skip short or overly long text
        if text.len() < 8 || text.len() > 200 {
            continue;
        }

        // Skip duplicates
        if seen.contains(&href) {
            continue;
        }

        // Skip if inside nav/header/footer (check ancestors)
        if nav_selector.is_some() {
            let mut is_in_nav = false;
            // Check if any ancestor matches nav selector
            for ancestor in element.ancestors() {
                if let Some(el) = ancestor.value().as_element() {
                    if el.name() == "nav"
                        || el.name() == "header"
                        || el.name() == "footer"
                        || el.attr("role") == Some("navigation")
                    {
                        is_in_nav = true;
                        break;
                    }
                }
            }
            if is_in_nav {
                continue;
            }
        }

        seen.insert(href.clone());
        links.push(Link { text, href });
    }

    links
}
