//! Link extraction from web pages.
//!
//! This module extracts clickable links from the page and associates
//! them with reference numbers for interactive navigation.

use serde::Deserialize;

/// A clickable link extracted from the page.
#[derive(Debug, Clone, Deserialize)]
pub struct Link {
    /// The visible text of the link
    pub text: String,
    /// The URL this link points to
    pub href: String,
}

/// Page content with both text and extracted links.
#[derive(Debug, Clone)]
pub struct PageContent {
    /// The URL the content was extracted from
    pub url: String,
    /// Text lines extracted from the page
    pub lines: Vec<String>,
    /// Links extracted from the page (indexed by their display number)
    pub links: Vec<Link>,
}

impl PageContent {
    pub fn new(url: String, lines: Vec<String>, links: Vec<Link>) -> Self {
        Self { url, lines, links }
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
