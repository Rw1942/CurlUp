//! Interactive start screen for CurlUp.
//!
//! Provides a clean welcome experience with:
//! - Curated list of popular sites
//! - Option to enter a custom URL
//! - Keyboard navigation

use anyhow::Result;
use console::style;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use std::io::{self, Write};

use crate::brand;

/// A curated site for the picker
struct Site {
    name: &'static str,
    url: &'static str,
    category: &'static str,
}

/// Curated list of well-tested sites for developers and tech enthusiasts
const SITES: &[Site] = &[
    // News & Aggregators
    Site { name: "Hacker News", url: "https://news.ycombinator.com", category: "News" },
    Site { name: "Lobsters", url: "https://lobste.rs", category: "News" },
    Site { name: "Product Hunt", url: "https://www.producthunt.com", category: "News" },
    Site { name: "Ars Technica", url: "https://arstechnica.com", category: "News" },
    // Developer Platforms
    Site { name: "GitHub Trending", url: "https://github.com/trending", category: "Dev" },
    Site { name: "GitLab Explore", url: "https://gitlab.com/explore", category: "Dev" },
    Site { name: "Stack Overflow", url: "https://stackoverflow.com", category: "Dev" },
    Site { name: "DEV Community", url: "https://dev.to", category: "Dev" },
    // Communities
    Site { name: "Reddit Programming", url: "https://www.reddit.com/r/programming", category: "Community" },
    Site { name: "Reddit LocalLLaMA", url: "https://www.reddit.com/r/LocalLLaMA", category: "Community" },
    Site { name: "Reddit Rust", url: "https://www.reddit.com/r/rust", category: "Community" },
    // Documentation & Reference
    Site { name: "MDN Web Docs", url: "https://developer.mozilla.org", category: "Docs" },
    Site { name: "Can I Use", url: "https://caniuse.com", category: "Reference" },
    Site { name: "Rust Docs", url: "https://doc.rust-lang.org/book", category: "Docs" },
    // Package Registries
    Site { name: "npm Registry", url: "https://www.npmjs.com", category: "Packages" },
    Site { name: "crates.io", url: "https://crates.io", category: "Packages" },
    Site { name: "PyPI", url: "https://pypi.org", category: "Packages" },
    // AI/ML
    Site { name: "Hugging Face", url: "https://huggingface.co", category: "AI/ML" },
    Site { name: "Papers With Code", url: "https://paperswithcode.com", category: "AI/ML" },
    // Web Dev
    Site { name: "Smashing Magazine", url: "https://www.smashingmagazine.com", category: "Web" },
];

/// Show the welcome screen and return the selected URL
pub fn show_start_screen() -> Result<String> {
    clear_screen();
    print_welcome_banner();

    // Build menu items
    let mut items: Vec<String> = SITES
        .iter()
        .enumerate()
        .map(|(i, site)| {
            format!(
                " {:>2}. {:<20} {}",
                i + 1,
                site.name,
                style(format!("[{}]", site.category)).dim()
            )
        })
        .collect();

    // Add custom URL option
    items.push(format!(
        " {:>2}. {}",
        items.len() + 1,
        style("Enter a custom URL...").italic()
    ));

    println!("  {}", style("Popular Sites").white().bold());
    println!("  {}\n", style("Use arrows or type a number, Enter to select").dim());

    let selection = Select::with_theme(&ColorfulTheme::default())
        .items(&items)
        .default(0)
        .max_length(14)
        .interact_opt()?;

    match selection {
        Some(index) if index < SITES.len() => {
            let site = &SITES[index];
            print_loading(site.name);
            Ok(site.url.to_string())
        }
        Some(_) => prompt_custom_url(),
        None => {
            println!("\n  {}\n", style("Goodbye!").dim());
            std::process::exit(0);
        }
    }
}

/// Prompt for a custom URL
fn prompt_custom_url() -> Result<String> {
    println!();

    let url: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("  Enter URL")
        .validate_with(|input: &String| {
            let trimmed = input.trim();
            if trimmed.is_empty() {
                Err("URL cannot be empty")
            } else if trimmed.contains(' ') {
                Err("URL cannot contain spaces")
            } else {
                Ok(())
            }
        })
        .interact_text()?;

    let url = normalize_url(&url);
    print_loading(&url);
    Ok(url)
}

/// Normalize URL (add https:// if needed)
fn normalize_url(url: &str) -> String {
    let url = url.trim();

    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else if url.starts_with("localhost") || url.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        format!("http://{}", url)
    } else {
        format!("https://{}", url)
    }
}

/// Print the welcome banner using brand colors
fn print_welcome_banner() {
    let banner = include_str!("assets/banner.txt");
    let build_time = env!("BUILD_TIMESTAMP");
    // Use brand teal for the banner
    println!("{}{}{}", brand::TEAL, format!("\n{banner}"), brand::RESET);
    println!("  {}{}{}\n", brand::MIST, format!("Built: {}", build_time), brand::RESET);
}

/// Print loading message
fn print_loading(target: &str) {
    println!();
    println!("  Loading {}...", style(target).white().bold());
    println!();
}

/// Clear terminal screen
fn clear_screen() {
    print!("\x1b[2J\x1b[1;1H");
    let _ = io::stdout().flush();
}
