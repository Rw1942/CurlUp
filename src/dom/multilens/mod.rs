//! Multi-lens content extraction system.
//!
//! This module orchestrates multiple extraction strategies to intelligently
//! extract main content from web pages:
//! - CSS selector-based extraction
//! - DOM tree traversal
//! - Text-density (CETD) analysis
//! - ARIA/semantic landmark detection
//!
//! Results are scored, deduplicated, and ranked to produce the best output.

mod css;
mod density;
mod normalize;
mod scorer;
mod semantic;
mod snapshot;
mod traversal;

use anyhow::Result;
use fantoccini::Client;
use scraper::Html;

use super::content::{Link, PageContent};
use super::extract::extract_page_content;

/// Minimum quality score threshold. If best extraction falls below this,
/// fall back to the simple innerText approach.
const MINIMUM_QUALITY_THRESHOLD: f32 = 0.2;

/// Extract page content using multiple extraction strategies.
///
/// This function:
/// 1. Fetches the full rendered HTML from the browser
/// 2. Parses it with scraper for Rust-side DOM manipulation
/// 3. Runs multiple extraction passes (CSS, traversal, density, semantic)
/// 4. Scores and ranks results to pick the best content
/// 5. Falls back to simple extraction if quality is too low
pub async fn extract_multilens(client: &Client, url: &str) -> Result<PageContent> {
    // Get full rendered HTML
    let html = snapshot::fetch_rendered_html(client).await?;
    let doc = Html::parse_document(&html);

    // Run all extraction strategies
    let css_result = css::extract_by_selectors(&doc);
    let traversal_result = traversal::extract_text_nodes(&doc);
    let density_result = density::extract_by_density(&html);
    let semantic_result = semantic::extract_landmarks(&doc);

    // Collect all candidates
    let candidates = vec![
        css_result,
        traversal_result,
        density_result,
        semantic_result,
    ];

    // Score and rank
    let (best_content, best_score) = scorer::rank_extracts(candidates);

    // Fall back if quality is too low
    if best_score < MINIMUM_QUALITY_THRESHOLD || best_content.is_empty() {
        return extract_page_content(client, url).await;
    }

    // Clean up the text
    let lines = normalize::clean_text(&best_content);

    // Extract links from the parsed document
    let links = extract_links(&doc, url);

    Ok(PageContent::new(url.to_string(), lines, links))
}

/// Extract links from a parsed HTML document.
fn extract_links(doc: &Html, _base_url: &str) -> Vec<Link> {
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
