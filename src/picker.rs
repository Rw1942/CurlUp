//! Interactive start screen for CurlUp.
//!
//! Provides a beautiful welcome experience with:
//! - Curated list of popular sites
//! - Option to enter a custom URL
//! - Keyboard navigation

use anyhow::Result;
use console::style;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use std::io::{self, Write};

/// A curated site for the picker
pub struct Site {
    pub name: &'static str,
    pub url: &'static str,
    pub category: &'static str,
    pub emoji: &'static str,
}

/// Curated list of well-tested sites (from SITES.md with ★★★★☆+ ratings)
pub const SITES: &[Site] = &[
    Site {
        name: "Hacker News",
        url: "https://news.ycombinator.com",
        category: "Tech",
        emoji: "🔶",
    },
    Site {
        name: "Google News",
        url: "https://news.google.com",
        category: "News",
        emoji: "📰",
    },
    Site {
        name: "GitHub Trending",
        url: "https://github.com/trending",
        category: "Dev",
        emoji: "⭐",
    },
    Site {
        name: "Reddit Programming",
        url: "https://www.reddit.com/r/programming",
        category: "Community",
        emoji: "💬",
    },
    Site {
        name: "DEV Community",
        url: "https://dev.to",
        category: "Tech",
        emoji: "👩‍💻",
    },
    Site {
        name: "NPR News",
        url: "https://www.npr.org",
        category: "News",
        emoji: "🎙️",
    },
    Site {
        name: "Wikipedia",
        url: "https://en.wikipedia.org",
        category: "Reference",
        emoji: "📚",
    },
    Site {
        name: "Weather",
        url: "https://wttr.in",
        category: "Weather",
        emoji: "🌤️",
    },
    Site {
        name: "IMDb Top Movies",
        url: "https://www.imdb.com/chart/top",
        category: "Movies",
        emoji: "🎬",
    },
    Site {
        name: "Reuters",
        url: "https://www.reuters.com",
        category: "News",
        emoji: "🌍",
    },
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
                " {}  {:>2}. {:<20} {}",
                site.emoji,
                i + 1,
                site.name,
                style(format!("[{}]", site.category)).dim()
            )
        })
        .collect();
    
    // Add custom URL option
    items.push(format!(
        " {}  {:>2}. {}",
        "🔗",
        items.len() + 1,
        style("Enter a custom URL...").italic()
    ));
    
    println!("  {}", style("Popular Sites").white().bold());
    println!("  {}\n", style("Use ↑↓ arrows or type a number, Enter to select").dim());

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
        Some(_) => {
            // Custom URL option selected
            prompt_custom_url()
        }
        None => {
            // User cancelled (Esc or q)
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

/// Print the welcome banner
fn print_welcome_banner() {
    let banner = r#"
    ╭─────────────────────────────────────────╮
    │                                         │
    │      ██████╗██╗   ██╗██████╗ ██╗        │
    │     ██╔════╝██║   ██║██╔══██╗██║        │
    │     ██║     ██║   ██║██████╔╝██║        │
    │     ██║     ██║   ██║██╔══██╗██║        │
    │     ╚██████╗╚██████╔╝██║  ██║███████╗   │
    │      ╚═════╝ ╚═════╝ ╚═╝  ╚═╝╚══════╝   │
    │                                         │
    │       Terminal Web Browser v0.1         │
    │                                         │
    ╰─────────────────────────────────────────╯
"#;
    
    println!("{}", style(banner).cyan());
}

/// Print loading message
fn print_loading(target: &str) {
    println!();
    println!(
        "  {} Loading {}...",
        style("⟳").cyan(),
        style(target).white().bold()
    );
    println!();
}

/// Clear terminal screen
fn clear_screen() {
    print!("\x1b[2J\x1b[1;1H");
    let _ = io::stdout().flush();
}
