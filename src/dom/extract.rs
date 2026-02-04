use anyhow::Result;
use fantoccini::Client;
use serde::Deserialize;
use std::time::Duration;

use super::content::{Link, PageContent};

/// Raw extraction result from JavaScript.
/// Fields are Option to handle null values from Chrome.
#[derive(Debug, Deserialize, Default)]
struct ExtractResult {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    links: Option<Vec<RawLink>>,
}

#[derive(Debug, Deserialize)]
struct RawLink {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    href: Option<String>,
}

/// Extract visible text content from the page
pub async fn extract_text(client: &Client) -> Result<Vec<String>> {
    let url = client.current_url().await?;
    let content = extract_page_content(client, url.as_str()).await?;
    Ok(content.lines)
}

/// Extract page content including text and links.
///
/// Uses retry logic with backoff to handle JavaScript-heavy pages
/// that may not have content ready immediately.
pub async fn extract_page_content(client: &Client, current_url: &str) -> Result<PageContent> {
    // Note: WebDriver's execute() requires an explicit `return` statement
    // at the script level to get values back. The IIFE alone won't work.
    let script = r#"
        return (function() {
            // Safety check: ensure body exists
            if (!document.body) {
                return { text: '', links: [] };
            }
            
            // Get all text
            var text = document.body.innerText || '';
            
            // Get all links with their text and href
            var linkElements = document.querySelectorAll('a[href]');
            var links = [];
            var seen = {};
            
            for (var i = 0; i < linkElements.length; i++) {
                var a = linkElements[i];
                var href = a.href;
                var linkText = (a.innerText || a.textContent || '').trim();
                
                // Filter out:
                // - Empty text or very short text
                // - JavaScript links
                // - Anchor-only links
                // - Already seen links (by href)
                // - Very long link text (likely not a real link)
                if (linkText.length < 3 || linkText.length > 200) continue;
                if (href.indexOf('javascript:') === 0) continue;
                if (href === window.location.href) continue;
                if (href === window.location.href + '#') continue;
                if (seen[href]) continue;
                
                // Skip common navigation/UI links
                var lowerText = linkText.toLowerCase();
                var skipWords = ['sign in', 'sign up', 'log in', 'subscribe', 'more', 'menu', 
                     'search', 'home', 'close', 'skip', 'advertisement'];
                var shouldSkip = false;
                for (var j = 0; j < skipWords.length; j++) {
                    if (lowerText === skipWords[j]) {
                        shouldSkip = true;
                        break;
                    }
                }
                if (shouldSkip) continue;
                
                seen[href] = true;
                links.push({ text: linkText, href: href });
            }
            
            return { text: text, links: links };
        })();
    "#;

    // Retry extraction up to 3 times with increasing delays
    // This handles pages where content loads asynchronously
    let mut extracted: Option<ExtractResult> = None;
    
    for attempt in 0..3 {
        // Try to execute the script - handle errors gracefully
        let result = match client.execute(script, vec![]).await {
            Ok(r) => r,
            Err(_) => {
                // Script execution failed, wait and retry
                if attempt < 2 {
                    tokio::time::sleep(Duration::from_millis(500 * (attempt + 1) as u64)).await;
                }
                continue;
            }
        };
        
        // If the driver returns null, treat it as empty content
        if result.is_null() {
            extracted = Some(ExtractResult::default());
            break;
        }

        // Try to deserialize - handles malformed responses
        match serde_json::from_value::<ExtractResult>(result) {
            Ok(ext) => {
                extracted = Some(ext);
                break;
            }
            Err(_) => {
                // Deserialization failed (null or wrong shape), will retry
            }
        }
        
        // Wait before retrying (500ms, 1000ms)
        if attempt < 2 {
            tokio::time::sleep(Duration::from_millis(500 * (attempt + 1) as u64)).await;
        }
    }
    
    // Use extracted content or return a fallback
    let extracted = extracted.unwrap_or_default();
    
    // Process text into lines (handle None as empty string)
    let text = extracted.text.unwrap_or_default();
    let lines: Vec<String> = text
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    
    let lines = if lines.is_empty() {
        vec!["[Page loaded but no text content found]".to_string()]
    } else {
        lines
    };
    
    // Convert links (handle None fields gracefully)
    let raw_links = extracted.links.unwrap_or_default();
    let links: Vec<Link> = raw_links
        .into_iter()
        .filter_map(|l| {
            // Only include links where both text and href are present
            match (l.text, l.href) {
                (Some(text), Some(href)) if !text.is_empty() && !href.is_empty() => {
                    Some(Link { text, href })
                }
                _ => None,
            }
        })
        .collect();
    
    Ok(PageContent::new(current_url.to_string(), lines, links))
}
