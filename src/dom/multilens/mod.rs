//! Multi-lens content extraction system.
//!
//! This module fetches rendered HTML and extracts links using Rust-side
//! DOM parsing with the scraper crate.

pub mod snapshot;

use anyhow::Result;
use fantoccini::Client;
use scraper::{Html, Selector};

use super::content::{Link, PageContent};
use super::filter::clean_html;
use super::link_filter::{filter_links, resolve_url};

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

    // Extract and filter links
    let links = extract_links(&doc, url);

    Ok(PageContent::new(url.to_string(), html, links))
}

/// Extract links from a parsed HTML document.
fn extract_links(doc: &Html, base_url: &str) -> Vec<Link> {
    let selector = match Selector::parse("a[href]") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut links = Vec::new();

    for element in doc.select(&selector) {
        // Get href attribute
        let href = match element.value().attr("href") {
            Some(h) => h.to_string(),
            None => continue,
        };

        // Skip if inside nav/header/footer
        let mut is_in_nav = false;
        for ancestor in element.ancestors() {
            if let Some(el) = ancestor.value().as_element() {
                let name = el.name();
                if name == "nav" || name == "header" || name == "footer" || name == "aside" {
                    is_in_nav = true;
                    break;
                }
                if el.attr("role").map_or(false, |r| {
                    r == "navigation" || r == "banner" || r == "contentinfo"
                }) {
                    is_in_nav = true;
                    break;
                }
            }
        }
        if is_in_nav {
            continue;
        }

        // Get visible text
        let text: String = element.text().collect::<Vec<_>>().join(" ");
        let text = text.trim().to_string();

        if text.is_empty() {
            continue;
        }

        // Resolve relative URLs to absolute
        let resolved_href = resolve_url(&href, base_url);

        links.push(Link {
            text,
            href: resolved_href,
        });
    }

    // Apply unified filtering (handles length, duplicates, anchors, etc.)
    filter_links(links, Some(base_url))
}
