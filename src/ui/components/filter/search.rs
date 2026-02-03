//! Fuzzy search implementation for filter bar.

use strsim::jaro_winkler;

/// Default fuzzy match threshold (0.0 - 1.0)
pub const DEFAULT_THRESHOLD: f64 = 0.7;

/// Perform fuzzy search on text
///
/// Returns true if the query matches the text with a score above the threshold.
/// Uses Jaro-Winkler similarity for fuzzy matching.
///
/// # Arguments
/// * `text` - The text to search in
/// * `query` - The search query
/// * `threshold` - Minimum similarity score (0.0 - 1.0)
///
/// # Returns
/// True if the text matches the query
pub fn fuzzy_match(text: &str, query: &str, threshold: f64) -> bool {
    if query.is_empty() {
        return true;
    }

    let text_lower = text.to_lowercase();
    let query_lower = query.to_lowercase();

    // Exact substring match is always a hit
    if text_lower.contains(&query_lower) {
        return true;
    }

    // Try fuzzy matching with Jaro-Winkler
    let score = jaro_winkler(&text_lower, &query_lower);
    score >= threshold
}

/// Perform fuzzy search with default threshold
pub fn fuzzy_match_default(text: &str, query: &str) -> bool {
    fuzzy_match(text, query, DEFAULT_THRESHOLD)
}

/// Multi-field fuzzy search
///
/// Returns true if the query matches any of the provided fields.
pub fn fuzzy_match_any(fields: &[&str], query: &str, threshold: f64) -> bool {
    if query.is_empty() {
        return true;
    }

    fields.iter().any(|field| fuzzy_match(field, query, threshold))
}

/// Calculate match score for ranking results
///
/// Returns a score from 0.0 to 1.0 indicating match quality.
/// Exact matches get 1.0, fuzzy matches get their similarity score.
pub fn match_score(text: &str, query: &str) -> f64 {
    if query.is_empty() {
        return 1.0;
    }

    let text_lower = text.to_lowercase();
    let query_lower = query.to_lowercase();

    // Exact match
    if text_lower == query_lower {
        return 1.0;
    }

    // Substring match gets high score
    if text_lower.contains(&query_lower) {
        return 0.95;
    }

    // Fuzzy match score
    jaro_winkler(&text_lower, &query_lower)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_match_exact() {
        assert!(fuzzy_match("Hello World", "hello world", 0.7));
    }

    #[test]
    fn test_fuzzy_match_substring() {
        assert!(fuzzy_match("Hello World", "world", 0.7));
    }

    #[test]
    fn test_fuzzy_match_empty_query() {
        assert!(fuzzy_match("Hello World", "", 0.7));
    }

    #[test]
    fn test_fuzzy_match_typo() {
        assert!(fuzzy_match("Uchigatana", "uchigatna", 0.7));
    }

    #[test]
    fn test_no_match() {
        assert!(!fuzzy_match("Hello World", "xyz123", 0.7));
    }
}
