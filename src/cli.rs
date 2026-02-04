use clap::Parser;

#[derive(Parser)]
#[command(name = "curlup")]
#[command(version)]
#[command(about = "A terminal-first, text-only browser powered by real web rendering")]
#[command(long_about = r#"CurlUp is a terminal web browser that renders pages using Chrome and lets you
navigate by clicking on numbered links - all from your command line.

USAGE:
    curlup                              Start with site picker, then browse
    curlup news.ycombinator.com         Go directly to a site and browse

EXAMPLES:
    curlup                              Pick a site interactively, then browse
    curlup github.com/trending          Browse GitHub trending repos
    curlup -v mail.google.com           Visible browser for login
    curlup --user-agent "MyBot/1.0"     Use custom user-agent
    curlup --no-stealth -v              Debug mode without stealth
    curlup --no-focus reddit.com        Show full page including nav/ads
    curlup -R medium.com/@user/article  Reader mode for articles

REQUIREMENTS:
    • Google Chrome installed
    • ChromeDriver matching your Chrome version"#)]
pub struct Cli {
    /// URL to navigate to (if omitted, shows interactive site picker)
    #[arg(value_name = "URL")]
    pub url: Option<String>,

    /// Launch Chrome in visible mode (useful for login or debugging)
    #[arg(long, short = 'v')]
    pub visible: bool,

    /// Custom user-agent string (overrides default stealth user-agent)
    #[arg(long, short = 'u', value_name = "STRING")]
    pub user_agent: Option<String>,

    /// Disable stealth mode (anti-detection measures)
    #[arg(long)]
    pub no_stealth: bool,

    /// Use multi-lens content extraction (CSS, ARIA, text-density heuristics)
    #[arg(long, short = 'm')]
    pub multilens: bool,

    /// Disable focus mode - show full page including navigation, ads, etc.
    #[arg(long)]
    pub no_focus: bool,

    /// Enable reader mode - extract just the article content (toggle with 'R' while browsing)
    #[arg(long, short = 'R')]
    pub reader: bool,
}

impl Cli {
    /// Returns true if focus mode is enabled (default: true, disabled with --no-focus)
    pub fn focus(&self) -> bool {
        !self.no_focus
    }
}
