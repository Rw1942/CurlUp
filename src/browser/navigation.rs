use anyhow::Result;
use fantoccini::Client;
use std::time::Duration;

/// Navigate to URL and wait for page to be fully ready.
///
/// Simple 2-step approach:
/// 1. Poll for document.readyState === 'complete' (max 8 seconds)
/// 2. Fixed buffer for JS rendering to settle
pub async fn navigate_and_wait(client: &Client, url: &str) -> Result<()> {
    client.goto(url).await?;

    // Wait for document ready (max 8 seconds)
    let ready_script = r#"return document.readyState === 'complete';"#;
    for _ in 0..32 {
        if client.execute(ready_script, vec![]).await? == serde_json::Value::Bool(true) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    // Fixed buffer for JS rendering
    tokio::time::sleep(Duration::from_millis(3000)).await;

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
