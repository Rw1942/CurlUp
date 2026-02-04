//! HTML content filtering for cleaner terminal output.
//!
//! Removes noise elements (ads, navigation, cookie banners, etc.)
//! while preserving the page's main content structure.
//! Works on diverse sites like Reddit, HN, GitHub, and articles.

use scraper::{Html, Selector};

/// Tags that are always removed (non-content elements).
const REMOVE_TAGS: &[&str] = &[
    "script", "style", "noscript", "iframe", "svg", "canvas",
    "video", "audio", "object", "embed", "nav", "header", "footer", "aside",
];

/// CSS selectors for noise elements to remove.
/// These use class/id patterns common across many sites.
const NOISE_SELECTORS: &[&str] = &[
    // ARIA roles for non-content
    "[role='navigation']",
    "[role='banner']",
    "[role='contentinfo']",
    "[role='complementary']",
    "[role='search']",
    "[aria-hidden='true']",
    "[hidden]",
    
    // Ads and sponsored content
    "[class*='ad-wrapper']",
    "[class*='ad-container']",
    "[class*='advertisement']",
    "[class*='sponsored']",
    "[class*='promoted']",
    "[id*='ad-wrapper']",
    "[id*='ad-container']",
    "[data-ad]",
    "[data-advertisement]",
    
    // Cookie/consent/GDPR banners
    "[class*='cookie-banner']",
    "[class*='cookie-notice']",
    "[class*='cookie-consent']",
    "[class*='consent-banner']",
    "[class*='gdpr']",
    "[class*='privacy-banner']",
    "[id*='cookie-banner']",
    "[id*='cookie-notice']",
    "[id*='consent']",
    "[id*='gdpr']",
    
    // Popups and modals
    "[class*='modal-overlay']",
    "[class*='popup-overlay']",
    "[class*='newsletter-popup']",
    "[class*='subscribe-modal']",
    "[class*='paywall']",
    
    // Social sharing widgets
    "[class*='share-buttons']",
    "[class*='social-share']",
    "[class*='sharing-buttons']",
    "[class*='follow-buttons']",
    
    // Site-specific noise
    "[class*='promotedlink']",
    "[class*='premium-banner']",
    "[class*='metabar']",
    "[class*='postActions']",
    "[class*='related-posts']",
    "[class*='recommended']",
    "[class*='trending']",
    "[class*='popular-posts']",
];

/// Clean HTML by removing noise elements while preserving content structure.
///
/// This function:
/// 1. Removes script/style/iframe/nav/header/footer tags entirely
/// 2. Removes elements matching noise selectors (ads, banners, etc.)
/// 3. Returns cleaned HTML for html2text rendering
pub fn clean_html(html: &str) -> String {
    // Build a combined selector for all elements to remove
    let mut selectors_to_remove: Vec<&str> = REMOVE_TAGS.to_vec();
    selectors_to_remove.extend(NOISE_SELECTORS.iter());
    
    // Create a mutable copy of the HTML
    let mut result = html.to_string();
    
    // Parse the document
    let doc = Html::parse_document(&result);
    
    // Collect all HTML fragments to remove (we collect them first to avoid modifying while iterating)
    let mut fragments_to_remove: Vec<String> = Vec::new();
    
    for selector_str in &selectors_to_remove {
        if let Ok(selector) = Selector::parse(selector_str) {
            for element in doc.select(&selector) {
                // Get the outer HTML of this element
                let outer_html = element.html();
                if !outer_html.is_empty() {
                    fragments_to_remove.push(outer_html);
                }
            }
        }
    }
    
    // Sort by length descending to remove larger fragments first
    // This helps avoid partial matches
    fragments_to_remove.sort_by_key(|b| std::cmp::Reverse(b.len()));
    
    // Remove each fragment
    for fragment in fragments_to_remove {
        result = result.replace(&fragment, "");
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_removes_script_tags() {
        let html = "<html><body><p>Content</p><script>alert('hi')</script></body></html>";
        let cleaned = clean_html(html);
        assert!(!cleaned.contains("script"));
        assert!(cleaned.contains("Content"));
    }

    #[test]
    fn test_removes_nav() {
        let html = "<html><body><nav>Menu</nav><main>Content</main></body></html>";
        let cleaned = clean_html(html);
        assert!(!cleaned.contains("Menu"));
        assert!(cleaned.contains("Content"));
    }

    #[test]
    fn test_removes_cookie_banner() {
        let html = r#"<html><body><div class="cookie-banner">Accept cookies</div><p>Article</p></body></html>"#;
        let cleaned = clean_html(html);
        assert!(!cleaned.contains("cookies"));
        assert!(cleaned.contains("Article"));
    }

    #[test]
    fn test_preserves_main_content() {
        let html = r#"<html><body>
            <header>Site Header</header>
            <article><h1>Title</h1><p>Important content here.</p></article>
            <footer>Copyright</footer>
        </body></html>"#;
        let cleaned = clean_html(html);
        assert!(cleaned.contains("Title"));
        assert!(cleaned.contains("Important content"));
        assert!(!cleaned.contains("Site Header"));
        assert!(!cleaned.contains("Copyright"));
    }
}
