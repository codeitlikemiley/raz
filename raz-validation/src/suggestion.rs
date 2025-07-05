//! Option suggestion system using fuzzy matching

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

/// Provides suggestions for misspelled options
pub struct OptionSuggester {
    matcher: SkimMatcherV2,
}

impl Default for OptionSuggester {
    fn default() -> Self {
        Self::new()
    }
}

impl OptionSuggester {
    /// Create a new suggester
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
        }
    }

    /// Get suggestions for a misspelled option
    pub fn suggest_options(&self, input: &str, valid_options: &[&str]) -> Vec<String> {
        let mut scored_options: Vec<(i64, String)> = valid_options
            .iter()
            .filter_map(|option| {
                self.matcher
                    .fuzzy_match(option, input)
                    .map(|score| (score, option.to_string()))
            })
            .collect();

        // Sort by score (higher is better)
        scored_options.sort_by(|a, b| b.0.cmp(&a.0));

        // Return top 3 suggestions
        scored_options
            .into_iter()
            .take(3)
            .map(|(_, option)| option)
            .collect()
    }

    /// Get suggestions with a minimum score threshold
    pub fn suggest_options_with_threshold(
        &self,
        input: &str,
        valid_options: &[&str],
        min_score: i64,
    ) -> Vec<String> {
        let suggestions = self.suggest_options(input, valid_options);

        // Filter by minimum score
        suggestions
            .into_iter()
            .filter(|option| {
                self.matcher
                    .fuzzy_match(option, input)
                    .is_some_and(|score| score >= min_score)
            })
            .collect()
    }

    /// Check if an option is similar enough to suggest
    pub fn is_similar(&self, input: &str, option: &str, threshold: i64) -> bool {
        self.matcher
            .fuzzy_match(option, input)
            .is_some_and(|score| score >= threshold)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_suggestions() {
        let suggester = OptionSuggester::new();
        let options = vec!["--release", "--debug", "--verbose", "--quiet"];

        let suggestions = suggester.suggest_options("--relase", &options);
        assert!(suggestions.contains(&"--release".to_string()));
    }

    #[test]
    fn test_no_suggestions_for_very_different_input() {
        let suggester = OptionSuggester::new();
        let options = vec!["--release", "--debug", "--verbose"];

        let suggestions = suggester.suggest_options_with_threshold("--xyz123", &options, 20);
        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_similarity_check() {
        let suggester = OptionSuggester::new();

        assert!(suggester.is_similar("--relase", "--release", 30));
        assert!(!suggester.is_similar("--xyz", "--release", 30));
    }
}
