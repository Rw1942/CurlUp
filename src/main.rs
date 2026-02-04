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

    // Run in interactive browse mode (default) or single-page mode
    if single_mode {
        // Single page mode - fetch once and exit
        browser::navigation::navigate_and_wait_with_stealth(&client, &current_url, stealth).await?;

        // Extract content including links (use multilens if enabled)
        let content = if args.multilens {
            dom::multilens::extract_multilens(&client, &current_url).await?
        } else {
            dom::extract::extract_page_content(&client, &current_url).await?
        };
        let link_count = content.links.len();

        // Render to terminal (condensed by default, raw if requested)
        if args.raw {
            render::text::render(&content.lines);
        } else {
            render::text::render_condensed(&content.lines);
        }

        // Print link count footer
        println!();
        println!("────────────────────────────────────────────────────────────");
        println!("  {} links found. Run without -s/-r for interactive browsing.", link_count);
    } else {
        // Interactive browsing mode (default)
        // This loop allows returning to the start screen with 'home' command
        loop {
            match browse::run_interactive_with_options(&client, &current_url, stealth, args.multilens).await {
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
