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
