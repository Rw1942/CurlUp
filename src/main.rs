use anyhow::{bail, Result};
use clap::Parser;

mod brand;
mod browse;
mod browser;
mod cli;
mod dom;
mod error;
mod picker;
mod render;
mod term;

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::Cli::parse();

    // Spawn ChromeDriver in the background
    let mut driver = browser::driver::spawn().await?;

    // Build Chrome configuration
    let chrome_config = browser::chrome::ChromeConfig {
        headless: !args.visible,
        stealth: !args.no_stealth,
        user_agent: args.user_agent.clone(),
    };

    // Connect to ChromeDriver with stealth settings
    let client = browser::chrome::connect(&driver.url(), &chrome_config).await?;

    // Get initial URL from argument or interactive picker
    let mut current_url = match args.url {
        Some(ref u) => normalize_url(u)?,
        None => picker::show_start_screen()?,
    };

    // Track stealth mode for navigation
    let stealth = chrome_config.stealth;

    // Track focus mode (enabled by default)
    let focus = args.focus();

    // Track condensed mode (enabled by default, can be disabled with -N flag)
    let condensed = args.condensed();

    // Interactive browsing mode
    // This loop allows returning to the start screen with 'home' command
    loop {
        match browse::run_interactive_with_options(&client, &current_url, stealth, args.multilens, focus, condensed).await {
            Ok(()) => {
                // User exited browse mode (quit or home)
                // Try to show start screen again
                match picker::show_start_screen() {
                    Ok(url) => {
                        current_url = url;
                        // Continue loop to browse new site
                    }
                    Err(_) => break, // User quit from start screen
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
        }
    }

    // Close the browser session
    client.close().await?;

    // Clean up ChromeDriver process
    driver.kill().await?;

    Ok(())
}

/// Normalize URL, adding https:// if no scheme is present
fn normalize_url(url: &str) -> Result<String> {
    let url = url.trim();

    if url.is_empty() {
        bail!("URL cannot be empty");
    }

    // If URL already has a scheme, use it as-is
    if url.starts_with("http://") || url.starts_with("https://") {
        return Ok(url.to_string());
    }

    // If URL starts with localhost or an IP, use http
    if url.starts_with("localhost") || url.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        return Ok(format!("http://{}", url));
    }

    // Otherwise, assume https
    Ok(format!("https://{}", url))
}
