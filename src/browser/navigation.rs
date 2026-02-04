//! Browser navigation utilities.
//!
//! Handles page navigation with:
//! - Stealth mode to avoid bot detection
//! - Smart wait for content stability (replaces fixed delays)
//! - Scroll handling for lazy-loaded content

use anyhow::Result;
use fantoccini::Client;
use std::time::Duration;

/// JavaScript to inject for stealth mode - hides automation indicators.
const STEALTH_JS: &str = r#"
    // Hide webdriver property
    Object.defineProperty(navigator, 'webdriver', {
        get: () => undefined,
        configurable: true
    });
    
    // Add fake plugins array (headless Chrome has empty plugins)
    Object.defineProperty(navigator, 'plugins', {
        get: () => [1, 2, 3, 4, 5],
        configurable: true
    });
    
    // Set realistic languages
    Object.defineProperty(navigator, 'languages', {
        get: () => ['en-US', 'en'],
        configurable: true
    });
    
    // Add chrome runtime object (missing in headless)
    if (!window.chrome) {
        window.chrome = {};
    }
    if (!window.chrome.runtime) {
        window.chrome.runtime = {};
    }
    
    // Hide automation-related properties
    const originalQuery = window.navigator.permissions.query;
    window.navigator.permissions.query = (parameters) => (
        parameters.name === 'notifications' ?
            Promise.resolve({ state: Notification.permission }) :
            originalQuery(parameters)
    );
"#;

/// JavaScript that waits for content to stabilize.
/// 
/// Polls document.body.innerText.length until it stops changing,
/// indicating that dynamic content has finished loading.
/// Returns early if content stabilizes, with a max timeout of 5 seconds.
const WAIT_FOR_CONTENT_JS: &str = r#"
return new Promise(function(resolve) {
    var maxWait = 5000;      // Maximum wait time (ms)
    var pollInterval = 300;  // How often to check (ms)
    var stableCount = 0;     // Consecutive stable readings needed
    var stableThreshold = 2; // How many stable readings = done
    var lastLength = -1;
    var elapsed = 0;
    
    function check() {
        if (elapsed >= maxWait) {
            resolve(true);
            return;
        }
        
        var currentLength = document.body ? document.body.innerText.length : 0;
        
        if (currentLength === lastLength && currentLength > 0) {
            stableCount++;
            if (stableCount >= stableThreshold) {
                resolve(true);
                return;
            }
        } else {
            stableCount = 0;
        }
        
        lastLength = currentLength;
        elapsed += pollInterval;
        setTimeout(check, pollInterval);
    }
    
    // Start checking after initial delay for JS to begin executing
    setTimeout(check, 500);
});
"#;

/// Navigate to URL and wait for page to be fully ready (with stealth enabled).
#[allow(dead_code)]
pub async fn navigate_and_wait(client: &Client, url: &str) -> Result<()> {
    navigate_and_wait_with_stealth(client, url, true).await
}

/// Navigate to URL with optional stealth mode.
/// 
/// Uses smart content detection instead of fixed delays:
/// 1. Navigate to the URL
/// 2. Inject stealth scripts (if enabled)
/// 3. Wait for document.readyState === 'complete'
/// 4. Wait for content to stabilize (no more changes to innerText)
pub async fn navigate_and_wait_with_stealth(client: &Client, url: &str, stealth: bool) -> Result<()> {
    client.goto(url).await?;

    // Inject stealth JavaScript immediately after navigation
    if stealth {
        // Ignore errors - some pages may block script execution
        let _ = client.execute(STEALTH_JS, vec![]).await;
    }

    // Wait for document ready (max 8 seconds)
    let ready_script = r#"return document.readyState === 'complete';"#;
    for _ in 0..32 {
        if client.execute(ready_script, vec![]).await? == serde_json::Value::Bool(true) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    // Re-inject stealth JS after page is ready (in case page overwrote properties)
    if stealth {
        let _ = client.execute(STEALTH_JS, vec![]).await;
    }

    // Smart wait: poll until content stabilizes instead of fixed delay
    let _ = client.execute(WAIT_FOR_CONTENT_JS, vec![]).await;

    Ok(())
}

/// Scroll down to trigger lazy content, then return to top.
pub async fn scroll_to_top_after_load(client: &Client) -> Result<()> {
    let height_script = r#"return window.innerHeight || 800;"#;
    let height_value = client.execute(height_script, vec![]).await?;
    let viewport = height_value.as_i64().unwrap_or(800).max(200);

    for step in 1..=3 {
        let offset = viewport * step as i64;
        let scroll_script = format!("window.scrollTo(0, {});", offset);
        let _ = client.execute(&scroll_script, vec![]).await?;
        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    let _ = client.execute("window.scrollTo(0, 0);", vec![]).await?;
    tokio::time::sleep(Duration::from_millis(200)).await;

    Ok(())
}
