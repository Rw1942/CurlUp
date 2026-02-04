use anyhow::Result;
use fantoccini::{Client, ClientBuilder};
use serde_json::{json, Map, Value};

/// Default user-agent string (Chrome on macOS)
const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36";

/// Configuration for Chrome connection
#[derive(Clone, Debug)]
pub struct ChromeConfig {
    pub headless: bool,
    pub stealth: bool,
    pub user_agent: Option<String>,
}

impl Default for ChromeConfig {
    fn default() -> Self {
        Self {
            headless: true,
            stealth: true,
            user_agent: None,
        }
    }
}

/// Connect to ChromeDriver with specified capabilities
pub async fn connect(webdriver_url: &str, config: &ChromeConfig) -> Result<Client> {
    let mut caps = Map::new();

    // Build Chrome options
    let mut chrome_opts = Map::new();
    let mut args = vec![
        "--disable-gpu".to_string(),
        "--no-sandbox".to_string(),
        "--disable-dev-shm-usage".to_string(),
    ];

    if config.headless {
        args.push("--headless=new".to_string());
    }

    // Stealth mode: add anti-detection flags
    if config.stealth {
        // Disable automation indicators that websites detect
        args.push("--disable-blink-features=AutomationControlled".to_string());
        
        // Set realistic window size
        args.push("--window-size=1920,1080".to_string());
        
        // Disable infobars like "Chrome is being controlled by automated software"
        args.push("--disable-infobars".to_string());
    }

    // Set user-agent (custom or default for stealth)
    let user_agent = config.user_agent.clone()
        .or_else(|| if config.stealth { Some(DEFAULT_USER_AGENT.to_string()) } else { None });
    
    if let Some(ua) = user_agent {
        args.push(format!("--user-agent={}", ua));
    }

    chrome_opts.insert("args".to_string(), json!(args));

    // Stealth mode: add experimental options to further hide automation
    if config.stealth {
        // Exclude switches that indicate automation
        chrome_opts.insert("excludeSwitches".to_string(), json!(["enable-automation"]));
        // Disable automation extension
        chrome_opts.insert("useAutomationExtension".to_string(), json!(false));
    }

    caps.insert("goog:chromeOptions".to_string(), Value::Object(chrome_opts));

    let client = ClientBuilder::native()
        .capabilities(caps)
        .connect(webdriver_url)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to ChromeDriver: {}", e))?;

    Ok(client)
}
