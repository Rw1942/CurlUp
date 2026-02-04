use anyhow::{bail, Result};
use clap::Parser;
use std::io::Write;
use std::process::{Command, Stdio};

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
    // Raw mode and pager mode imply single-page mode (for piping to other tools)
    let single_mode = args.single || args.raw || args.pager;

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

        // Build output lines using html2text for clean terminal rendering
        let output_lines = render::text::render_html(&content.html);

        // Add footer
        let footer = format!(
            "\n────────────────────────────────────────────────────────────\n  {} links found. Run without -s/-r/-p for interactive browsing.",
            link_count
        );

        if args.pager {
            // Pager mode - pipe to system pager
            let output = output_lines.join("\n") + &footer + "\n";
            output_to_pager(&output)?;
        } else {
            // Direct output to stdout
            for line in output_lines {
                println!("{}", line);
            }
            println!("{}", footer);
        }
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

/// Output content to the system pager (less or $PAGER)
fn output_to_pager(content: &str) -> Result<()> {
    // Get pager from environment or default to less
    let pager = std::env::var("PAGER").unwrap_or_else(|_| "less".to_string());

    // Try to spawn the pager
    let mut child = match Command::new(&pager)
        .stdin(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => {
            // Fallback: try less explicitly
            match Command::new("less").stdin(Stdio::piped()).spawn() {
                Ok(child) => child,
                Err(_) => {
                    // No pager available, just print to stdout
                    print!("{}", content);
                    return Ok(());
                }
            }
        }
    };

    // Write content to pager's stdin
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(content.as_bytes());
    }

    // Wait for pager to exit
    let _ = child.wait();

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
