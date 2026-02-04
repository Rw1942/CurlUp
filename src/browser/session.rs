use anyhow::Result;
use fantoccini::Client;
use std::time::Duration;

/// Navigate to URL and wait for page to be fully ready.
///
/// This uses a robust 3-step approach for JavaScript-heavy sites:
/// 1. Poll for document.readyState === 'complete'
/// 2. Poll until document.body has actual text content
/// 3. Small buffer for any final JS rendering
pub async fn navigate_and_wait(client: &Client, url: &str) -> Result<()> {
    client.goto(url).await?;
    
    // Step 1: Poll for document.readyState === 'complete'
    // This ensures the page and all resources (images, scripts) are loaded
    // Note: WebDriver execute() is synchronous - don't use Promises
    let ready_script = r#"return document.readyState === 'complete';"#;
    
    for _ in 0..20 {
        let result = client.execute(ready_script, vec![]).await?;
        if result == serde_json::Value::Bool(true) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    
    // Step 2: Wait for actual content in the body
    // JavaScript-heavy sites (like Google News) may load content after 'load' event
    let content_check = r#"
        return document.body && document.body.innerText && document.body.innerText.trim().length > 10;
    "#;
    
    // Poll up to 5 seconds for content to appear
    for _ in 0..10 {
        let result = client.execute(content_check, vec![]).await?;
        if result == serde_json::Value::Bool(true) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    
    // Step 3: Small buffer for final rendering
    tokio::time::sleep(Duration::from_millis(300)).await;

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
