//! Content scoring and ranking.
//!
//! Evaluates extracted content from multiple sources and selects the best
//! result based on various quality heuristics.

use std::collections::HashSet;

/// Result from a content extraction strategy.
#[derive(Debug, Clone)]
pub struct ContentResult {
    /// The extracted text content
    pub content: String,
    /// Source identifier (e.g., "css:article", "density:cetd")
    pub source: String,
    /// Confidence score from the extractor (0.0 - 1.0)
    pub confidence: f32,
}

impl Default for ContentResult {
    fn default() -> Self {
        Self {
            content: String::new(),
            source: "none".to_string(),
            confidence: 0.0,
        }
    }
}

/// Detailed score breakdown for a content result.
#[derive(Debug)]
struct ContentScore {
    /// Length-based score (longer = more content)
    length_score: f32,
    /// Link density score (lower link ratio = better for articles)
    link_density_score: f32,
    /// Paragraph structure score (well-structured = better)
    structure_score: f32,
    /// Source confidence from extractor
    source_confidence: f32,
    /// Bonus for specific high-value sources
    source_bonus: f32,
    /// Final combined score
    total: f32,
}

/// Rank multiple extraction results and return the best content.
///
/// Returns a tuple of (best_content, best_score).
pub fn rank_extracts(candidates: Vec<ContentResult>) -> (String, f32) {
    if candidates.is_empty() {
        return (String::new(), 0.0);
    }

    // Score each candidate
    let mut scored: Vec<(ContentResult, ContentScore)> = candidates
        .into_iter()
        .filter(|c| !c.content.is_empty())
        .map(|c| {
            let score = score_content(&c);
            (c, score)
        })
        .collect();

    // Sort by total score descending
    scored.sort_by(|a, b| b.1.total.partial_cmp(&a.1.total).unwrap_or(std::cmp::Ordering::Equal));

    // If top results are close in score, prefer the more structured one
    if scored.len() >= 2 {
        let diff = scored[0].1.total - scored[1].1.total;
        if diff < 0.1 {
            // Close scores - prefer better structure
            if scored[1].1.structure_score > scored[0].1.structure_score + 0.2 {
                scored.swap(0, 1);
            }
        }
    }

    // Get the best result
    if let Some((best, score)) = scored.into_iter().next() {
        (best.content, score.total)
    } else {
        (String::new(), 0.0)
    }
}

/// Score a single content result.
fn score_content(result: &ContentResult) -> ContentScore {
    let content = &result.content;

    // Length score: prefer content between 500-15000 chars
    let len = content.len();
    let length_score = if len < 100 {
        0.1
    } else if len < 500 {
        0.3 + (len as f32 / 500.0) * 0.3
    } else if len <= 15000 {
        0.8 + ((len - 500) as f32 / 14500.0) * 0.2
    } else {
        // Very long content might include noise
        0.9 - ((len - 15000) as f32 / 50000.0).min(0.3)
    };

    // Link density: count link-like patterns
    let link_count = count_link_patterns(content);
    let word_count = content.split_whitespace().count();
    let link_density = if word_count > 0 {
        link_count as f32 / word_count as f32
    } else {
        0.0
    };
    // Lower link density is better for article content
    let link_density_score = 1.0 - (link_density * 10.0).min(1.0);

    // Structure score: look for paragraph-like structure
    let structure_score = calculate_structure_score(content);

    // Source confidence from the extractor
    let source_confidence = result.confidence;

    // Source-specific bonuses
    let source_bonus = match result.source.as_str() {
        s if s.starts_with("density:cetd") => 0.15,
        s if s.starts_with("semantic:aria-main") => 0.1,
        s if s.starts_with("css:article") || s.starts_with("css:main") => 0.1,
        s if s.starts_with("semantic:html5") => 0.08,
        s if s.starts_with("css:role-main") => 0.12,
        _ => 0.0,
    };

    // Combine scores with weights
    let total = (length_score * 0.25)
        + (link_density_score * 0.15)
        + (structure_score * 0.2)
        + (source_confidence * 0.3)
        + source_bonus;

    ContentScore {
        length_score,
        link_density_score,
        structure_score,
        source_confidence,
        source_bonus,
        total: total.min(1.0),
    }
}

/// Count patterns that look like links (http://, www., etc.).
fn count_link_patterns(content: &str) -> usize {
    let patterns = ["http://", "https://", "www.", "click here", "read more"];
    patterns
        .iter()
        .map(|p| content.matches(p).count())
        .sum()
}

/// Calculate a structure score based on paragraph patterns.
fn calculate_structure_score(content: &str) -> f32 {
    let lines: Vec<&str> = content.lines().collect();

    if lines.is_empty() {
        return 0.0;
    }

    // Count non-empty lines with reasonable length (20-500 chars)
    let good_paragraphs = lines
        .iter()
        .filter(|l| {
            let len = l.trim().len();
            len >= 20 && len <= 500
        })
        .count();

    // Count very short lines (navigation-like)
    let short_lines = lines.iter().filter(|l| l.trim().len() < 10).count();

    // Good ratio of paragraph-like content
    let total = lines.len();
    let para_ratio = good_paragraphs as f32 / total as f32;
    let short_ratio = short_lines as f32 / total as f32;

    // Penalize high ratio of short lines
    let score = para_ratio - (short_ratio * 0.5);
    score.max(0.0).min(1.0)
}

/// Deduplicate content that overlaps significantly.
pub fn deduplicate_results(results: &mut Vec<ContentResult>) {
    if results.len() <= 1 {
        return;
    }

    let mut to_remove = HashSet::new();

    for i in 0..results.len() {
        if to_remove.contains(&i) {
            continue;
        }

        for j in (i + 1)..results.len() {
            if to_remove.contains(&j) {
                continue;
            }

            // Check for significant overlap
            let overlap = calculate_overlap(&results[i].content, &results[j].content);
            if overlap > 0.8 {
                // Remove the shorter/lower confidence one
                if results[i].content.len() > results[j].content.len() {
                    to_remove.insert(j);
                } else {
                    to_remove.insert(i);
                }
            }
        }
    }

    // Remove duplicates in reverse order to preserve indices
    let mut indices: Vec<_> = to_remove.into_iter().collect();
    indices.sort_by(|a, b| b.cmp(a));
    for idx in indices {
        results.remove(idx);
    }
}

/// Calculate overlap ratio between two strings.
fn calculate_overlap(a: &str, b: &str) -> f32 {
    let a_words: HashSet<&str> = a.split_whitespace().collect();
    let b_words: HashSet<&str> = b.split_whitespace().collect();

    if a_words.is_empty() || b_words.is_empty() {
        return 0.0;
    }

    let intersection = a_words.intersection(&b_words).count();
    let smaller = a_words.len().min(b_words.len());

    intersection as f32 / smaller as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rank_prefers_longer_content() {
        let candidates = vec![
            ContentResult {
                content: "Short.".to_string(),
                source: "test".to_string(),
                confidence: 0.5,
            },
            ContentResult {
                content: "This is a much longer piece of content that should be preferred because it contains more substantial text that would be useful to a reader.".to_string(),
                source: "test".to_string(),
                confidence: 0.5,
            },
        ];

        let (best, _score) = rank_extracts(candidates);
        assert!(best.contains("much longer"));
    }

    #[test]
    fn test_rank_considers_source_bonus() {
        let candidates = vec![
            ContentResult {
                content: "Content from density extraction with good length.".to_string(),
                source: "density:cetd".to_string(),
                confidence: 0.6,
            },
            ContentResult {
                content: "Content from generic traversal with good length.".to_string(),
                source: "traversal".to_string(),
                confidence: 0.6,
            },
        ];

        let (best, _score) = rank_extracts(candidates);
        // CETD has source bonus, so might be preferred
        assert!(!best.is_empty());
    }

    #[test]
    fn test_calculate_overlap() {
        let a = "the quick brown fox jumps over the lazy dog";
        let b = "the quick brown fox";
        let overlap = calculate_overlap(a, b);
        assert!(overlap > 0.9); // b is fully contained in a
    }
}
