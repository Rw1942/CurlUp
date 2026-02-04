//! Page content and link types.

/// A clickable link extracted from the page.
#[derive(Debug, Clone)]
pub struct Link {
    /// The URL this link points to
    pub href: String,
}

/// Page content with HTML and extracted links.
#[derive(Debug, Clone)]
pub struct PageContent {
    /// Raw HTML from the rendered page (for html2text rendering)
    pub html: String,
    /// Links extracted from the page (indexed by their display number)
    pub links: Vec<Link>,
}

impl PageContent {
    pub fn new(html: String, links: Vec<Link>) -> Self {
        Self { html, links }
    }

    /// Get a link by its 1-based display number
    pub fn get_link(&self, number: usize) -> Option<&Link> {
        if number == 0 || number > self.links.len() {
            None
        } else {
            Some(&self.links[number - 1])
        }
    }
}
