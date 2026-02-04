//! Text-density based content extraction using CETD algorithm.
//!
//! Uses the dom-content-extraction crate which implements Content Extraction
//! via Text Density (CETD) to identify content-rich blocks and filter out
//! navigation, ads, and other boilerplate.

use dom_content_extraction::scraper::Html;
use dom_content_extraction::DensityTree;

use super::scorer::ContentResult;

/// Extract main content using text-density analysis (CETD algorithm).
///
/// This algorithm analyzes the ratio of text to markup in different parts
/// of the document to identify the main content block.
pub fn extract_by_density(html: &str) -> ContentResult {
    // Parse HTML using dom_content_extraction's scraper
    let doc = Html::parse_document(html);

    // Build density tree and extract content
    match DensityTree::from_document(&doc) {
        Ok(tree) => {
            // Get the extracted content (pass the document for text extraction)
            let content = tree.extract_content(&doc).unwrap_or_default();

            if content.is_empty() {
                return ContentResult {
                    content: String::new(),
                    source: "density:empty".to_string(),
                    confidence: 0.0,
                };
            }

            // CETD is a well-established algorithm, so higher confidence
            let confidence = calculate_density_confidence(&content);

            ContentResult {
                content,
                source: "density:cetd".to_string(),
                confidence,
            }
        }
        Err(_) => ContentResult {
            content: String::new(),
            source: "density:error".to_string(),
            confidence: 0.0,
        },
    }
}

/// Calculate confidence score for density-based extraction.
fn calculate_density_confidence(content: &str) -> f32 {
    let len = content.len();

    // Empty or very short content is low confidence
    if len < 100 {
        return 0.1;
    }

    // Very long content might include noise
    if len > 50000 {
        return 0.6;
    }

    // Sweet spot for article content
    if len > 500 && len < 20000 {
        return 0.9;
    }

    // Reasonable content length
    if len >= 100 && len <= 500 {
        return 0.7;
    }

    0.75
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_article_content() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head><title>Test</title></head>
            <body>
                <nav>
                    <a href="/">Home</a>
                    <a href="/about">About</a>
                    <a href="/contact">Contact</a>
                </nav>
                <article>
                    <h1>Main Article Title</h1>
                    <p>This is the first paragraph of the article with substantial content
                    that should be extracted by the text density algorithm.</p>
                    <p>This is another paragraph with more content. The algorithm should
                    identify this as the main content area because it has high text density
                    compared to the navigation and other boilerplate areas.</p>
                    <p>A third paragraph continues the article with even more meaningful
                    content that a reader would want to see.</p>
                </article>
                <aside>
                    <h3>Related Links</h3>
                    <a href="/link1">Link 1</a>
                    <a href="/link2">Link 2</a>
                </aside>
                <footer>
                    <p>Copyright 2024</p>
                </footer>
            </body>
            </html>
        "#;

        let result = extract_by_density(html);

        // Should extract the article content
        assert!(
            result.content.contains("Main Article")
                || result.content.contains("first paragraph")
                || result.content.len() > 100
        );
        assert!(result.confidence > 0.0);
    }

    #[test]
    fn test_empty_html() {
        let html = "<html><body></body></html>";
        let result = extract_by_density(html);
        assert!(result.content.is_empty() || result.confidence < 0.5);
    }
}
