use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::time::sleep;

const CHROMEDRIVER_PORT: u16 = 9515;
const STARTUP_TIMEOUT_SECS: u64 = 10;
const POLL_INTERVAL_MS: u64 = 100;

/// Holds the ChromeDriver child process handle
/// When dropped, the process is killed
pub struct ChromeDriverProcess {
    child: Child,
    port: u16,
}

impl ChromeDriverProcess {
    /// Get the WebDriver URL for this ChromeDriver instance
    pub fn url(&self) -> String {
        format!("http://localhost:{}", self.port)
    }

    /// Kill the ChromeDriver process
    pub async fn kill(&mut self) -> Result<()> {
        self.child.kill().await?;
        Ok(())
    }
}

impl Drop for ChromeDriverProcess {
    fn drop(&mut self) {
        // Best-effort kill on drop
        // Note: This is synchronous, so we use start_kill() which doesn't wait
        let _ = self.child.start_kill();
    }
}

/// Find the chromedriver executable
fn find_chromedriver() -> Result<PathBuf> {
    // Check ~/.local/bin/chromedriver first (per project setup)
    if let Some(home) = dirs::home_dir() {
        let local_path = home.join(".local/bin/chromedriver");
        if local_path.exists() {
            return Ok(local_path);
        }
    }

    // Fall back to PATH lookup
    if let Ok(path) = which::which("chromedriver") {
        return Ok(path);
    }

    Err(anyhow!(
        r#"ChromeDriver not found!

CurlUp requires ChromeDriver to control Chrome. To install:

  macOS (Homebrew):
    brew install chromedriver

  macOS (Manual):
    1. Check Chrome version: Chrome → About Google Chrome
    2. Download matching driver: https://googlechromelabs.github.io/chrome-for-testing/
    3. Install: mv chromedriver ~/.local/bin/ && chmod +x ~/.local/bin/chromedriver
    4. Remove quarantine: xattr -d com.apple.quarantine ~/.local/bin/chromedriver

  Linux:
    sudo apt install chromium-chromedriver

CurlUp searches: ~/.local/bin/chromedriver, then PATH"#
    ))
}

/// Spawn ChromeDriver as a background process
pub async fn spawn() -> Result<ChromeDriverProcess> {
    let chromedriver_path = find_chromedriver()?;

    let child = Command::new(&chromedriver_path)
        .arg(format!("--port={}", CHROMEDRIVER_PORT))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| anyhow!("Failed to spawn ChromeDriver: {}\n\nIs ChromeDriver executable? Try: chmod +x ~/.local/bin/chromedriver", e))?;

    let process = ChromeDriverProcess {
        child,
        port: CHROMEDRIVER_PORT,
    };

    // Wait for ChromeDriver to be ready
    wait_for_ready(&process).await?;

    Ok(process)
}

/// Poll ChromeDriver status endpoint until it's ready
async fn wait_for_ready(process: &ChromeDriverProcess) -> Result<()> {
    let status_url = format!("{}/status", process.url());
    let client = reqwest::Client::new();

    let deadline = tokio::time::Instant::now() + Duration::from_secs(STARTUP_TIMEOUT_SECS);

    while tokio::time::Instant::now() < deadline {
        match client.get(&status_url).send().await {
            Ok(response) if response.status().is_success() => {
                return Ok(());
            }
            _ => {
                sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
            }
        }
    }

    Err(anyhow!(
        r#"ChromeDriver failed to start within {} seconds.

Possible causes:
  • Another ChromeDriver is already running (try: pkill chromedriver)
  • ChromeDriver version doesn't match Chrome version
  • On macOS: quarantine not removed (try: xattr -d com.apple.quarantine ~/.local/bin/chromedriver)

To debug, try running ChromeDriver manually:
  chromedriver --port=9515"#,
        STARTUP_TIMEOUT_SECS
    ))
}
