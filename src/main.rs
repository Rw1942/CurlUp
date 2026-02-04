use anyhow::{bail, Result};
use clap::Parser;

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

    // Determine if we should use single-page mode
    // Raw mode implies single-page mode (for piping to other tools)
    let single_mode = args.single || args.raw;

    // Spawn ChromeDriver in the background
    let mut driver = browser::driver::spawn().await?;

    // Connect to ChromeDriver (headless unless --visible flag is set)
    let headless = !args.visible;
    let client = browser::chrome::connect(&driver.url(), headless).await?;

    // Get initial URL from argument or interactive picker
    let mut current_url = match args.url {
        Some(ref u) => normalize_url(u)?,
        None => picker::show_start_screen()?,
    };

    // Run in interactive browse mode (default) or single-page mode
    if single_mode {
        // Single page mode - fetch once and exit
        browser::navigation::navigate_and_wait(&client, &current_url).await?;

        // Extract text content from the DOM
        let text = dom::extract::extract_text(&client).await?;

        // Render to terminal (condensed by default, raw if requested)
        if args.raw {
            render::text::render(&text);
        } else {
            render::text::render_condensed(&text);
        }
    } else {
        // Interactive browsing mode (default)
        // This loop allows returning to the start screen with 'home' command
        loop {
            match browse::run_interactive(&client, &current_url, false).await {
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
