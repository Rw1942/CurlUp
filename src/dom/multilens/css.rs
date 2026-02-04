//! CSS selector-based content extraction.
//!
//! Uses prioritized CSS selectors to identify and extract main content,
//! filtering out navigation, sidebars, and other boilerplate elements.

use scraper::{Html, Selector};

use super::scorer::ContentResult;

/// Selectors that typically contain main page content, in priority order.
/// Earlier selectors are more likely to contain the primary content.
const CONTENT_SELECTORS: &[&str] = &[
    // Explicit main content markers
    "[role=\"main\"]",
    "main",
    "article",
    "[itemprop=\"articleBody\"]",
    "[itemprop=\"mainContentOfPage\"]",
    // Common content class patterns
    ".post-content",
    ".article-content",
    ".entry-content",
    ".content-body",
    ".story-body",
    ".article-body",
    // Generic content containers
    "#content",
    ".content",
    "#main-content",
    ".main-content",
    // Section as fallback
    "section:not([role=\"navigation\"])",
];

/// Selectors for elements that should be excluded (noise).
const NOISE_SELECTORS: &[&str] = &[
    "nav",
    "header",
    "footer",
    "aside",
    "[role=\"navigation\"]",
    "[role=\"banner\"]",
    "[role=\"complementary\"]",
    "[role=\"contentinfo\"]",
    ".sidebar",
    ".navigation",
    ".nav",
    ".menu",
    ".advertisement",
    ".ad",
    ".ads",
    ".social-share",
    ".share-buttons",
    ".comments",
    ".related-posts",
    ".recommended",
    "script",
    "style",
    "noscript",
    "iframe",
];

/// Extract content using CSS selectors.
///
/// Tries each content selector in priority order, collecting text
/// and filtering out noise elements.
pub fn extract_by_selectors(doc: &Html) -> ContentResult {
    let mut all_text = Vec::new();
    let mut source = "css:fallback";

    // Parse noise selectors once
    let noise_selectors: Vec<Selector> = NOISE_SELECTORS
        .iter()
        .filter_map(|s| Selector::parse(s).ok())
        .collect();

    // Try each content selector in priority order
    for (idx, sel_str) in CONTENT_SELECTORS.iter().enumerate() {
        if let Ok(selector) = Selector::parse(sel_str) {
            for element in doc.select(&selector) {
                // Skip if this element or ancestors are noise
                if is_noise_element(&element, &noise_selectors) {
                    continue;
                }

                // Collect text, filtering out noise descendants
                let text = extract_text_excluding_noise(&element, &noise_selectors);
                let text = text.trim();

                if !text.is_empty() && text.len() > 50 {
                    all_text.push(text.to_string());
                    if idx < 4 {
                        // High-priority selector
                        source = match idx {
                            0 => "css:role-main",
                            1 => "css:main",
                            2 => "css:article",
                            3 => "css:itemprop",
                            _ => "css:content-class",
                        };
                    }
                }
            }
        }
    }

    // Join all collected text
    let content = all_text.join("\n\n");

    ContentResult {
        content,
        source: source.to_string(),
        confidence: calculate_confidence(&all_text),
    }
}

/// Check if an element matches any noise selector.
fn is_noise_element(
    element: &scraper::ElementRef,
    noise_selectors: &[Selector],
) -> bool {
    // Check the element itself
    for sel in noise_selectors {
        if sel.matches(element) {
            return true;
        }
    }

    // Check ancestors
    for ancestor in element.ancestors() {
        if let Some(el) = ancestor.value().as_element() {
            let name = el.name();
            if name == "nav"
                || name == "header"
                || name == "footer"
                || name == "aside"
                || name == "script"
                || name == "style"
            {
                return true;
            }
            if let Some(role) = el.attr("role") {
                if role == "navigation" || role == "banner" || role == "complementary" {
                    return true;
                }
            }
        }
    }

    false
}

/// Extract text from an element, excluding noise descendants.
fn extract_text_excluding_noise(
    element: &scraper::ElementRef,
    _noise_selectors: &[Selector],
) -> String {
    let mut text_parts = Vec::new();

    for node in element.descendants() {
        // Skip noise elements
        if let Some(el) = node.value().as_element() {
            let name = el.name();
            if name == "script"
                || name == "style"
                || name == "nav"
                || name == "noscript"
                || name == "iframe"
            {
                continue;
            }
        }

        // Collect text nodes
        if let Some(text) = node.value().as_text() {
            let t = text.trim();
            if !t.is_empty() {
                text_parts.push(t.to_string());
            }
        }
    }

    text_parts.join(" ")
}

/// Calculate confidence score based on extracted text quality.
fn calculate_confidence(texts: &[String]) -> f32 {
    if texts.is_empty() {
        return 0.0;
    }

    let total_len: usize = texts.iter().map(|t| t.len()).sum();
    let avg_len = total_len / texts.len();

    // Score based on:
    // - Total content length (more is generally better for articles)
    // - Average segment length (longer segments = more coherent content)
    // - Number of segments (fewer = more focused)
    let len_score = (total_len as f32 / 5000.0).min(1.0);
    let avg_score = (avg_len as f32 / 500.0).min(1.0);
    let segment_score = if texts.len() <= 3 { 1.0 } else { 0.7 };

    (len_score * 0.4 + avg_score * 0.4 + segment_score * 0.2).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_simple_article() {
        let html = r#"
            <html>
                <body>
                    <nav>Navigation here</nav>
                    <article>
                        <h1>Article Title</h1>
                        <p>This is the main article content that should be extracted.</p>
                    </article>
                    <footer>Footer content</footer>
                </body>
            </html>
        "#;
        let doc = Html::parse_document(html);
        let result = extract_by_selectors(&doc);

        assert!(result.content.contains("main article content"));
        assert!(!result.content.contains("Navigation"));
        assert!(!result.content.contains("Footer"));
    }
}
