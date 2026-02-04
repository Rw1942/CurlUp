use clap::Parser;

#[derive(Parser)]
#[command(name = "curlup")]
#[command(version)]
#[command(about = "A terminal-first, text-only browser powered by real web rendering")]
#[command(long_about = r#"CurlUp is a terminal web browser that renders pages using Chrome and lets you
navigate by clicking on numbered links - all from your command line.

INTERACTIVE MODE (default):
    curlup                              Start with site picker, then browse
    curlup news.ycombinator.com         Go directly to a site and browse

SINGLE-PAGE MODE:
    curlup -s news.google.com           Fetch page once and exit (good for piping)
    curlup -s -r news.google.com        Raw output for scripting

EXAMPLES:
    curlup                              Pick a site interactively, then browse
    curlup github.com/trending          Browse GitHub trending repos
    curlup -p example.com               View page in pager (less) for easy scrolling
    curlup -s example.com | less        Pipe single page to less (manual)
    curlup -v mail.google.com           Visible browser for login
    curlup --user-agent "MyBot/1.0"     Use custom user-agent
    curlup --no-stealth -v              Debug mode without stealth
    curlup -m medium.com/article        Multi-lens extraction for better content

REQUIREMENTS:
    • Google Chrome installed
    • ChromeDriver matching your Chrome version"#)]
pub struct Cli {
    /// URL to navigate to (if omitted, shows interactive site picker)
    #[arg(value_name = "URL")]
    pub url: Option<String>,

    /// Single-page mode - fetch once and exit (disables interactive browsing)
    #[arg(long, short = 's')]
    pub single: bool,

    /// Launch Chrome in visible mode (useful for login or debugging)
    #[arg(long, short = 'v')]
    pub visible: bool,

    /// Raw output mode (no formatting, all lines separate - implies --single)
    #[arg(long, short = 'r')]
    pub raw: bool,

    /// Custom user-agent string (overrides default stealth user-agent)
    #[arg(long, short = 'u', value_name = "STRING")]
    pub user_agent: Option<String>,

    /// Disable stealth mode (anti-detection measures)
    #[arg(long)]
    pub no_stealth: bool,

    /// Use multi-lens content extraction (CSS, ARIA, text-density heuristics)
    #[arg(long, short = 'm')]
    pub multilens: bool,

    /// Pipe output to system pager (less) for easy scrolling - implies --single
    #[arg(long, short = 'p')]
    pub pager: bool,
}
