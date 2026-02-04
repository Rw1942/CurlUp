//! DOM tree traversal for text extraction.
//!
//! Walks the entire DOM tree collecting text nodes while tracking
//! context (depth, parent tags) for scoring purposes.

use scraper::{ElementRef, Html, Node, Selector};

use super::scorer::ContentResult;

/// Tags that should be completely skipped (no text extraction).
const EXCLUDED_TAGS: &[&str] = &[
    "script", "style", "noscript", "iframe", "svg", "canvas", "video", "audio",
    "object", "embed", "applet", "template",
];

/// Tags that typically contain navigation/boilerplate content.
const NAV_TAGS: &[&str] = &["nav", "header", "footer", "aside", "menu"];

/// Tags that indicate content blocks.
const CONTENT_TAGS: &[&str] = &[
    "p", "article", "section", "main", "div", "span", "li", "td", "th",
    "h1", "h2", "h3", "h4", "h5", "h6", "blockquote", "pre", "code",
];

/// A text segment with context about where it came from.
#[derive(Debug, Clone)]
pub struct TextSegment {
    /// The actual text content
    pub text: String,
    /// Depth in the DOM tree (root = 0)
    pub depth: usize,
    /// Whether this is inside a navigation-like element
    pub in_nav: bool,
    /// Whether this is inside a known content element
    pub in_content: bool,
    /// Parent tag name
    pub parent_tag: String,
}

/// Extract text nodes from the DOM with context tracking.
pub fn extract_text_nodes(doc: &Html) -> ContentResult {
    let segments = traverse_with_context(doc);

    // Filter and score segments
    let content_segments: Vec<&TextSegment> = segments
        .iter()
        .filter(|s| !s.in_nav && s.text.len() > 10)
        .collect();

    // Prioritize segments that are in content containers
    let mut content_text: Vec<String> = content_segments
        .iter()
        .filter(|s| s.in_content)
        .map(|s| s.text.clone())
        .collect();

    // If we didn't find much in content containers, use all non-nav text
    if content_text.iter().map(|t| t.len()).sum::<usize>() < 200 {
        content_text = content_segments.iter().map(|s| s.text.clone()).collect();
    }

    let content = content_text.join("\n");
    let confidence = calculate_traversal_confidence(&content_segments);

    ContentResult {
        content,
        source: "traversal".to_string(),
        confidence,
    }
}

/// Traverse the DOM tree collecting text nodes with context.
fn traverse_with_context(doc: &Html) -> Vec<TextSegment> {
    let mut segments = Vec::new();

    // Use scraper's select with a universal selector to get all elements
    // Then walk their text content
    let body_selector = Selector::parse("body").ok();

    if let Some(sel) = body_selector {
        for body in doc.select(&sel) {
            collect_text_from_element(&body, 0, false, false, "body", &mut segments);
        }
    }

    segments
}

/// Recursively collect text from an element and its descendants.
fn collect_text_from_element(
    element: &ElementRef,
    depth: usize,
    in_nav: bool,
    in_content: bool,
    _parent_tag: &str,
    segments: &mut Vec<TextSegment>,
) {
    let tag = element.value().name();

    // Skip excluded tags entirely
    if EXCLUDED_TAGS.contains(&tag) {
        return;
    }

    // Track navigation context
    let is_nav_tag = NAV_TAGS.contains(&tag)
        || element.value().attr("role") == Some("navigation")
        || element.value().attr("role") == Some("banner")
        || element.value().attr("role") == Some("complementary");

    // Track content context
    let is_content_tag = CONTENT_TAGS.contains(&tag)
        || element.value().attr("role") == Some("main")
        || element.value().attr("role") == Some("article")
        || element.value().attr("itemprop") == Some("articleBody");

    let new_in_nav = in_nav || is_nav_tag;
    let new_in_content = in_content || is_content_tag;

    // Iterate through children
    for child in element.children() {
        match child.value() {
            Node::Text(text) => {
                let t = text.trim();
                if !t.is_empty() {
                    segments.push(TextSegment {
                        text: t.to_string(),
                        depth,
                        in_nav: new_in_nav,
                        in_content: new_in_content,
                        parent_tag: tag.to_string(),
                    });
                }
            }
            Node::Element(_) => {
                // Recursively process child elements
                if let Some(child_el) = ElementRef::wrap(child) {
                    collect_text_from_element(
                        &child_el,
                        depth + 1,
                        new_in_nav,
                        new_in_content,
                        tag,
                        segments,
                    );
                }
            }
            _ => {}
        }
    }
}

/// Calculate confidence score for traversal results.
fn calculate_traversal_confidence(segments: &[&TextSegment]) -> f32 {
    if segments.is_empty() {
        return 0.0;
    }

    let total_len: usize = segments.iter().map(|s| s.text.len()).sum();
    let content_len: usize = segments
        .iter()
        .filter(|s| s.in_content)
        .map(|s| s.text.len())
        .sum();

    // Score based on total content and how much is in content containers
    let len_score = (total_len as f32 / 3000.0).min(1.0);
    let content_ratio = if total_len > 0 {
        content_len as f32 / total_len as f32
    } else {
        0.0
    };

    // Traversal is a fallback method, so cap confidence
    ((len_score * 0.5 + content_ratio * 0.5) * 0.8).min(0.8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_traverse_excludes_scripts() {
        let html = r#"
            <html>
                <body>
                    <p>Visible content</p>
                    <script>var x = "script content";</script>
                </body>
            </html>
        "#;
        let doc = Html::parse_document(html);
        let result = extract_text_nodes(&doc);

        assert!(result.content.contains("Visible content"));
        assert!(!result.content.contains("script content"));
    }

    #[test]
    fn test_traverse_marks_nav_content() {
        let html = r#"
            <html>
                <body>
                    <nav>Nav link</nav>
                    <article>Article content here</article>
                </body>
            </html>
        "#;
        let doc = Html::parse_document(html);
        let segments = traverse_with_context(&doc);

        let nav_segment = segments.iter().find(|s| s.text.contains("Nav"));
        let article_segment = segments.iter().find(|s| s.text.contains("Article"));

        assert!(nav_segment.map(|s| s.in_nav).unwrap_or(false));
        assert!(article_segment.map(|s| s.in_content).unwrap_or(false));
    }
}
