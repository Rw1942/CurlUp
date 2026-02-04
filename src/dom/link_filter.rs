//! Shared link filtering logic.
//!
//! Centralizes all link filtering rules to ensure consistency
//! between JavaScript and Rust extraction paths.

use std::collections::HashSet;

use super::content::Link;

/// Maximum number of links to display/annotate.
pub const MAX_LINKS: usize = 999;

/// Filter and deduplicate a list of links.
///
/// Applies consistent rules:
/// - No javascript: URLs
/// - No anchor-only links (# or #section)
/// - No duplicates by href
/// - Skips current page URL if provided
pub fn filter_links(links: Vec<Link>, current_url: Option<&str>) -> Vec<Link> {
    let mut seen: HashSet<String> = HashSet::new();
    let current = current_url.unwrap_or("");

    links
        .into_iter()
        .filter(|link| {
            // Skip javascript: links
            if link.href.starts_with("javascript:") {
                return false;
            }

            // Skip anchor-only links
            if link.href.starts_with('#') {
                return false;
            }

            // Skip current page URL
            if !current.is_empty() && link.href == current {
                return false;
            }

            // Skip duplicates
            if seen.contains(&link.href) {
                return false;
            }

            seen.insert(link.href.clone());
            true
        })
        .collect()
}

/// Resolve a potentially relative URL against a base URL.
pub fn resolve_url(href: &str, base_url: &str) -> String {
    // Already absolute
    if href.starts_with("http://") || href.starts_with("https://") {
        return href.to_string();
    }

    // Protocol-relative
    if href.starts_with("//") {
        let protocol = if base_url.starts_with("https://") {
            "https:"
        } else {
            "http:"
        };
        return format!("{}{}", protocol, href);
    }

    // Parse base URL to get origin
    let base = base_url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let protocol = if base_url.starts_with("https://") {
        "https://"
    } else {
        "http://"
    };

    // Get origin (host + port)
    let origin = base.split('/').next().unwrap_or(base);

    if href.starts_with('/') {
        // Absolute path
        format!("{}{}{}", protocol, origin, href)
    } else {
        // Relative path - append to base directory
        let base_path = base.rfind('/').map(|i| &base[..i]).unwrap_or(base);
        format!("{}{}/{}", protocol, base_path, href)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_javascript() {
        let links = vec![Link {
            text: "Click here now".to_string(),
            href: "javascript:void(0)".to_string(),
        }];
        assert!(filter_links(links, None).is_empty());
    }

    #[test]
    fn test_filter_anchor() {
        let links = vec![Link {
            text: "Jump to section".to_string(),
            href: "#section".to_string(),
        }];
        assert!(filter_links(links, None).is_empty());
    }

    #[test]
    fn test_resolve_absolute() {
        assert_eq!(
            resolve_url("https://other.com/page", "https://example.com"),
            "https://other.com/page"
        );
    }

    #[test]
    fn test_resolve_relative() {
        assert_eq!(
            resolve_url("/about", "https://example.com/page"),
            "https://example.com/about"
        );
    }
}
