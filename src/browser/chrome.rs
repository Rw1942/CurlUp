use anyhow::Result;
use fantoccini::{Client, ClientBuilder};
use serde_json::{json, Map, Value};

/// Connect to ChromeDriver with specified capabilities
pub async fn connect(webdriver_url: &str, headless: bool) -> Result<Client> {
    let mut caps = Map::new();

    // Build Chrome options
    let mut chrome_opts = Map::new();
    let mut args = vec![
        "--disable-gpu".to_string(),
        "--no-sandbox".to_string(),
        "--disable-dev-shm-usage".to_string(),
    ];

    if headless {
        args.push("--headless=new".to_string());
    }

    chrome_opts.insert("args".to_string(), json!(args));
    caps.insert("goog:chromeOptions".to_string(), Value::Object(chrome_opts));

    let client = ClientBuilder::native()
        .capabilities(caps)
        .connect(webdriver_url)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to ChromeDriver: {}", e))?;

    Ok(client)
}
