//! Fuzzy matching utilities for command palette, file tree, and symbol search
//!
//! Supports:
//! - Exact match (highest score)
//! - Prefix match
//! - Substring match
//! - Acronym match (hw -> hello_world)
//! - Character-by-character match (lowest score)

/// Basic fuzzy match - returns true if pattern matches text in any way
pub fn fuzzy_match(text: &str, pattern: &str) -> bool {
    fuzzy_match_score(text, pattern) > 0
}

/// Advanced fuzzy matching with scoring
///
/// Returns a score indicating match quality:
/// - 10000+: Exact match
/// - 5000+: Prefix match
/// - 3000+: Acronym match (hw -> hello_world, hwrap -> hello_world_rust_application)
/// - 1000+: Word boundary match (typing "world" matches "hello_world")
/// - 500+: Substring match
/// - 100+: Character-by-character fuzzy match
/// - 0: No match
pub fn fuzzy_match_score(text: &str, pattern: &str) -> i32 {
    if pattern.is_empty() {
        return 0;
    }

    let text_lower = text.to_lowercase();
    let pattern_lower = pattern.to_lowercase();

    // Exact match (highest priority)
    if text_lower == pattern_lower {
        return 10000;
    }

    // Prefix match
    if text_lower.starts_with(&pattern_lower) {
        return 5000;
    }

    // ✅ Acronym match (hw -> hello_world)
    if let Some(acronym_score) = try_acronym_match(&text_lower, &pattern_lower) {
        return acronym_score;
    }

    // Word boundary match (typing "world" matches "hello_world")
    if let Some(word_score) = try_word_boundary_match(&text_lower, &pattern_lower) {
        return word_score;
    }

    // Substring match
    if text_lower.contains(&pattern_lower) {
        // Bonus if substring is at word boundary
        let bonus = if text_lower
            .split(|c: char| !c.is_alphanumeric())
            .any(|word| word.starts_with(&pattern_lower))
        {
            200
        } else {
            0
        };
        return 500 + bonus;
    }

    // Character-by-character fuzzy match
    if let Some(char_score) = try_char_by_char_match(&text_lower, &pattern_lower) {
        return char_score;
    }

    0 // No match
}

/// Try acronym matching: "hw" matches "hello_world", "hwrap" matches "hello_world_rust_app"
fn try_acronym_match(text: &str, pattern: &str) -> Option<i32> {
    let _pattern_chars: Vec<char> = pattern.chars().collect();

    // Extract acronym from text (first char + chars after underscore/dash/camelCase)
    let mut acronym = String::new();
    let mut prev_was_separator = true;
    let mut prev_was_lowercase = false;

    for ch in text.chars() {
        if ch == '_' || ch == '-' || ch == '/' || ch == '.' {
            prev_was_separator = true;
            prev_was_lowercase = false;
        } else if prev_was_separator || (prev_was_lowercase && ch.is_uppercase()) {
            acronym.push(ch.to_lowercase().next().unwrap_or(ch));
            prev_was_separator = false;
            prev_was_lowercase = ch.is_lowercase();
        } else {
            prev_was_lowercase = ch.is_lowercase();
            prev_was_separator = false;
        }
    }

    // Check if pattern matches acronym
    if acronym.starts_with(pattern) {
        // Score based on how much of the acronym matched
        let match_ratio = pattern.len() as f32 / acronym.len().max(1) as f32;
        return Some(3000 + (match_ratio * 500.0) as i32);
    }

    None
}

/// Try word boundary matching
fn try_word_boundary_match(text: &str, pattern: &str) -> Option<i32> {
    let words: Vec<&str> = text.split(|c: char| !c.is_alphanumeric()).collect();

    for (i, word) in words.iter().enumerate() {
        if word.starts_with(pattern) {
            // Earlier words get higher scores
            let position_bonus = (words.len() - i) * 100;
            return Some(1000 + position_bonus as i32);
        }
    }

    None
}

/// Character-by-character fuzzy matching
fn try_char_by_char_match(text: &str, pattern: &str) -> Option<i32> {
    let mut pattern_idx = 0;
    let pattern_chars: Vec<char> = pattern.chars().collect();
    let text_chars: Vec<char> = text.chars().collect();

    let mut last_match_idx = 0;
    let mut consecutive_matches = 0;
    let mut max_consecutive = 0;

    for (i, &ch) in text_chars.iter().enumerate() {
        if pattern_idx < pattern_chars.len() && ch == pattern_chars[pattern_idx] {
            pattern_idx += 1;

            // Track consecutive matches for bonus scoring
            if i == last_match_idx + 1 {
                consecutive_matches += 1;
                max_consecutive = max_consecutive.max(consecutive_matches);
            } else {
                consecutive_matches = 1;
            }

            last_match_idx = i;
        }
    }

    if pattern_idx == pattern_chars.len() {
        // Bonus for consecutive character matches
        let consecutive_bonus = max_consecutive * 10;
        return Some(100 + consecutive_bonus);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        assert!(fuzzy_match_score("hello", "hello") > 9000);
    }

    #[test]
    fn test_prefix_match() {
        let score = fuzzy_match_score("hello_world", "hello");
        assert!(score > 4000 && score < 6000);
    }

    #[test]
    fn test_acronym_match() {
        // hw -> hello_world
        assert!(fuzzy_match("hello_world", "hw"));
        let score = fuzzy_match_score("hello_world", "hw");
        assert!(score > 3000, "Expected acronym score > 3000, got {}", score);

        // hwrap -> hello_world_rust_app
        assert!(fuzzy_match("hello_world_rust_app", "hwrap"));

        // camelCase: hW -> helloWorld
        assert!(fuzzy_match("helloWorld", "hw"));
    }

    #[test]
    fn test_substring_match() {
        assert!(fuzzy_match("hello_world", "world"));
        let score = fuzzy_match_score("hello_world", "world");
        assert!(score > 500 && score < 2000);
    }

    #[test]
    fn test_no_match() {
        assert!(!fuzzy_match("hello_world", "xyz"));
        assert_eq!(fuzzy_match_score("hello_world", "xyz"), 0);
    }

    #[test]
    fn test_char_by_char_match() {
        assert!(fuzzy_match("hello_world", "hlwrd"));
        let score = fuzzy_match_score("hello_world", "hlwrd");
        assert!(score > 0 && score < 500);
    }

    #[test]
    fn test_word_boundary_match() {
        let score = fuzzy_match_score("src/common/fuzzy.rs", "fuzzy");
        assert!(
            score > 1000,
            "Expected word boundary score > 1000, got {}",
            score
        );
    }

    #[test]
    fn test_scoring_priority() {
        let text = "hello_world";

        let exact = fuzzy_match_score(text, "hello_world");
        let prefix = fuzzy_match_score(text, "hello");
        let acronym = fuzzy_match_score(text, "hw");
        let substring = fuzzy_match_score(text, "world");
        let fuzzy = fuzzy_match_score(text, "hlwrd");

        // Verify scoring hierarchy
        assert!(exact > prefix);
        assert!(prefix > acronym);
        assert!(acronym > substring);
        assert!(substring > fuzzy);
    }
}
