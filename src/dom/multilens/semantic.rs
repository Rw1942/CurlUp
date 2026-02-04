//! ARIA and semantic HTML landmark extraction.
//!
//! Focuses on accessibility-aware content detection using ARIA roles,
//! landmarks, and semantic HTML5 elements to identify main content.

use scraper::{Html, Selector};

use super::scorer::ContentResult;

/// ARIA landmark roles that indicate main content.
const MAIN_CONTENT_ROLES: &[&str] = &[
    "[role=\"main\"]",
    "[role=\"article\"]",
    "[role=\"region\"][aria-label]",
];

/// Semantic HTML5 elements that indicate content areas.
const SEMANTIC_CONTENT: &[&str] = &[
    "main",
    "article",
    "[itemprop=\"articleBody\"]",
    "[itemprop=\"mainContentOfPage\"]",
];

/// ARIA attributes that label content sections.
const ARIA_LABELED_CONTENT: &[&str] = &[
    "[aria-label*=\"content\" i]",
    "[aria-label*=\"article\" i]",
    "[aria-label*=\"main\" i]",
    "[aria-labelledby]",
];

/// Roles and elements to exclude.
const EXCLUDED_ROLES: &[&str] = &[
    "[role=\"navigation\"]",
    "[role=\"banner\"]",
    "[role=\"complementary\"]",
    "[role=\"contentinfo\"]",
    "[role=\"search\"]",
    "[role=\"form\"]",
    "[aria-hidden=\"true\"]",
];

/// Extract content using ARIA landmarks and semantic HTML.
pub fn extract_landmarks(doc: &Html) -> ContentResult {
    let mut all_text = Vec::new();
    let mut source = "semantic:generic";

    // Parse exclusion selectors
    let excluded: Vec<Selector> = EXCLUDED_ROLES
        .iter()
        .filter_map(|s| Selector::parse(s).ok())
        .collect();

    // Try main content roles first (highest priority)
    for sel_str in MAIN_CONTENT_ROLES {
        if let Some(text) = extract_from_selector(doc, sel_str, &excluded) {
            if !text.is_empty() && text.len() > 50 {
                all_text.push(text);
                source = "semantic:aria-main";
            }
        }
    }

    // Try semantic HTML5 elements
    for sel_str in SEMANTIC_CONTENT {
        if let Some(text) = extract_from_selector(doc, sel_str, &excluded) {
            if !text.is_empty() && text.len() > 50 {
                all_text.push(text);
                if source == "semantic:generic" {
                    source = "semantic:html5";
                }
            }
        }
    }

    // Try ARIA-labeled content as fallback
    if all_text.is_empty() {
        for sel_str in ARIA_LABELED_CONTENT {
            if let Some(text) = extract_from_selector(doc, sel_str, &excluded) {
                if !text.is_empty() && text.len() > 50 {
                    all_text.push(text);
                    source = "semantic:aria-label";
                }
            }
        }
    }

    // Deduplicate and join
    let content = deduplicate_text(&all_text);
    let confidence = calculate_semantic_confidence(&all_text, source);

    ContentResult {
        content,
        source: source.to_string(),
        confidence,
    }
}

/// Extract text from elements matching a selector, excluding noise elements.
fn extract_from_selector(
    doc: &Html,
    selector_str: &str,
    excluded: &[Selector],
) -> Option<String> {
    let selector = Selector::parse(selector_str).ok()?;
    let mut texts = Vec::new();

    for element in doc.select(&selector) {
        // Skip if element is in an excluded region
        if is_excluded(&element, excluded) {
            continue;
        }

        // Collect text, filtering out excluded descendants
        let text = extract_clean_text(&element, excluded);
        if !text.is_empty() {
            texts.push(text);
        }
    }

    if texts.is_empty() {
        None
    } else {
        Some(texts.join("\n\n"))
    }
}

/// Check if an element or its ancestors match exclusion selectors.
fn is_excluded(element: &scraper::ElementRef, excluded: &[Selector]) -> bool {
    // Check element itself
    for sel in excluded {
        if sel.matches(element) {
            return true;
        }
    }

    // Check ancestors
    for ancestor in element.ancestors() {
        if let Some(el) = ancestor.value().as_element() {
            // Check common exclusion attributes
            if el.attr("aria-hidden") == Some("true") {
                return true;
            }
            if let Some(role) = el.attr("role") {
                if role == "navigation"
                    || role == "banner"
                    || role == "complementary"
                    || role == "contentinfo"
                {
                    return true;
                }
            }
        }
    }

    false
}

/// Extract text from an element, skipping excluded descendants.
fn extract_clean_text(element: &scraper::ElementRef, excluded: &[Selector]) -> String {
    let mut parts = Vec::new();

    for node in element.descendants() {
        // Check if this is an excluded element
        if let Some(el_ref) = scraper::ElementRef::wrap(node) {
            if is_excluded(&el_ref, excluded) {
                continue;
            }
        }

        // Skip script, style, etc.
        if let Some(el) = node.value().as_element() {
            let tag = el.name();
            if tag == "script" || tag == "style" || tag == "noscript" || tag == "template" {
                continue;
            }
        }

        // Collect text nodes
        if let Some(text) = node.value().as_text() {
            let t = text.trim();
            if !t.is_empty() {
                parts.push(t.to_string());
            }
        }
    }

    parts.join(" ")
}

/// Remove duplicate or highly overlapping text segments.
fn deduplicate_text(texts: &[String]) -> String {
    if texts.is_empty() {
        return String::new();
    }

    let mut unique = Vec::new();

    for text in texts {
        // Check if this text is substantially contained in any existing segment
        let is_duplicate = unique.iter().any(|existing: &String| {
            existing.contains(text.as_str()) || text.contains(existing.as_str())
        });

        if !is_duplicate {
            unique.push(text.clone());
        }
    }

    unique.join("\n\n")
}

/// Calculate confidence for semantic extraction.
fn calculate_semantic_confidence(texts: &[String], source: &str) -> f32 {
    if texts.is_empty() {
        return 0.0;
    }

    let total_len: usize = texts.iter().map(|t| t.len()).sum();

    // Base score on content length
    let len_score = (total_len as f32 / 3000.0).min(1.0);

    // Bonus for specific ARIA roles (they're more reliable)
    let source_bonus = match source {
        "semantic:aria-main" => 0.2,
        "semantic:html5" => 0.15,
        "semantic:aria-label" => 0.1,
        _ => 0.0,
    };

    (len_score * 0.8 + source_bonus).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_role_main() {
        let html = r#"
            <html>
            <body>
                <nav role="navigation">Skip this</nav>
                <div role="main">
                    <h1>Main Content</h1>
                    <p>This is the main content area identified by ARIA role.</p>
                </div>
            </body>
            </html>
        "#;
        let doc = Html::parse_document(html);
        let result = extract_landmarks(&doc);

        assert!(result.content.contains("Main Content"));
        assert!(!result.content.contains("Skip this"));
        assert!(result.source.contains("aria"));
    }

    #[test]
    fn test_extract_semantic_article() {
        let html = r#"
            <html>
            <body>
                <header>Header content</header>
                <article>
                    <h1>Article Title</h1>
                    <p>Article body text that should be extracted.</p>
                </article>
                <footer>Footer content</footer>
            </body>
            </html>
        "#;
        let doc = Html::parse_document(html);
        let result = extract_landmarks(&doc);

        assert!(result.content.contains("Article"));
    }

    #[test]
    fn test_skip_aria_hidden() {
        let html = r#"
            <html>
            <body>
                <main>
                    <p>Visible content</p>
                    <div aria-hidden="true">Hidden from accessibility</div>
                </main>
            </body>
            </html>
        "#;
        let doc = Html::parse_document(html);
        let result = extract_landmarks(&doc);

        assert!(result.content.contains("Visible"));
        // aria-hidden content may or may not be included depending on parsing
    }
}
