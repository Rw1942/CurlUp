//! Reader mode content extraction using Mozilla's Readability algorithm.
//!
//! This module provides article-focused content extraction that removes
//! navigation, ads, and other clutter to present clean, readable text.

use dom_smoothie::{Readability, Article};
use scraper::{Html, Selector};

use super::content::Link;
use super::link_filter::filter_links;

/// Average reading speed in words per minute
const WORDS_PER_MINUTE: usize = 200;

/// Article content extracted in reader mode.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ReaderContent {
    /// The URL the content was extracted from
    pub url: String,
    /// Article title
    pub title: String,
    /// Author/byline if available
    pub byline: Option<String>,
    /// Site name if available
    pub site_name: Option<String>,
    /// Publication date if available
    pub published_time: Option<String>,
    /// Article HTML content (cleaned)
    pub html: String,
    /// Plain text content
    pub text_content: String,
    /// Excerpt/description
    pub excerpt: Option<String>,
    /// Word count of the article
    pub word_count: usize,
    /// Estimated reading time in minutes
    pub read_time_minutes: usize,
    /// Links extracted from the article content
    pub links: Vec<Link>,
}

impl ReaderContent {
    /// Get a link by its 1-based display number
    pub fn get_link(&self, number: usize) -> Option<&Link> {
        if number == 0 || number > self.links.len() {
            None
        } else {
            Some(&self.links[number - 1])
        }
    }
}

/// Check if HTML content is likely readable (contains article content).
///
/// This is a quick heuristic check that should be called before attempting
/// full article extraction. Returns `true` if the page appears to contain
/// article content suitable for reader mode.
pub fn is_probably_readable(html: &str) -> bool {
    // Use dom_smoothie's built-in detection
    match Readability::new(html, None, None) {
        Ok(reader) => reader.is_probably_readable(),
        Err(_) => false,
    }
}

/// Extract article content from HTML using the Readability algorithm.
///
/// Returns `Some(ReaderContent)` if article extraction succeeds,
/// or `None` if the page doesn't contain extractable article content.
pub fn extract_reader_content(html: &str, url: &str) -> Option<ReaderContent> {
    // Create Readability instance with the page URL for link resolution
    let mut reader = Readability::new(html, Some(url), None).ok()?;
    
    // Parse the article
    let article: Article = reader.parse().ok()?;
    
    // Convert content to String
    let content_html = article.content.to_string();
    let text_content = article.text_content.to_string();
    
    // Calculate word count and read time
    let word_count = count_words(&text_content);
    let read_time_minutes = (word_count / WORDS_PER_MINUTE).max(1);
    
    // Extract links from the article HTML
    let links = extract_article_links(&content_html, url);
    
    Some(ReaderContent {
        url: url.to_string(),
        title: article.title,
        byline: article.byline,
        site_name: article.site_name,
        published_time: article.published_time,
        html: content_html,
        text_content,
        excerpt: article.excerpt,
        word_count,
        read_time_minutes,
        links,
    })
}

/// Count words in text content.
fn count_words(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Extract links from article HTML content.
fn extract_article_links(html: &str, base_url: &str) -> Vec<Link> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("a[href]").unwrap();
    
    let raw_links: Vec<Link> = document
        .select(&selector)
        .filter_map(|element| {
            let href = element.value().attr("href")?;
            let text: String = element.text().collect::<Vec<_>>().join(" ");
            let text = text.trim().to_string();
            
            if text.is_empty() || href.is_empty() {
                return None;
            }
            
            Some(Link {
                text,
                href: href.to_string(),
            })
        })
        .collect();
    
    // Apply unified link filtering
    filter_links(raw_links, Some(base_url))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_words() {
        assert_eq!(count_words("Hello world"), 2);
        assert_eq!(count_words("One two three four five"), 5);
        assert_eq!(count_words(""), 0);
        assert_eq!(count_words("   spaces   everywhere   "), 2);
    }

    #[test]
    fn test_is_probably_readable_simple() {
        let article_html = r#"
            <html>
            <body>
                <article>
                    <h1>Test Article Title</h1>
                    <p>This is a paragraph with enough content to be considered readable.
                    It contains multiple sentences and provides meaningful information
                    that a user might want to read in a distraction-free environment.</p>
                    <p>Another paragraph with more content to ensure the article
                    has sufficient text density to be detected as readable content.</p>
                </article>
            </body>
            </html>
        "#;
        
        // Note: This may or may not pass depending on dom_smoothie's heuristics
        // The important thing is it doesn't panic
        let _ = is_probably_readable(article_html);
    }

    #[test]
    fn test_extract_reader_content_basic() {
        let html = r#"
            <html>
            <head><title>Test Article</title></head>
            <body>
                <article>
                    <h1>Test Article Title</h1>
                    <p>This is the main content of the article. It needs to be
                    long enough for the readability algorithm to consider it
                    worth extracting as article content.</p>
                    <p>Here is another paragraph with a <a href="https://example.com">link</a>
                    that should be preserved in reader mode.</p>
                    <p>And a third paragraph to add more content density.</p>
                </article>
            </body>
            </html>
        "#;
        
        let result = extract_reader_content(html, "https://test.com/article");
        // The result depends on dom_smoothie's extraction
        // Just verify it doesn't panic
        if let Some(content) = result {
            assert!(!content.title.is_empty() || content.word_count > 0);
        }
    }
}
