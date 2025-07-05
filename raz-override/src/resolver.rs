use crate::{
    OverrideEntry,
    detector::FunctionDetector,
    error::Result,
    key::{FunctionContext, OverrideKey},
    storage::OverrideStorage,
};
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};

/// Resolution strategy for finding overrides
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResolutionStrategy {
    /// Exact match on primary key
    Exact,
    /// Match on any fallback key
    Fallback,
    /// Fuzzy match on function name
    FuzzyFunction,
    /// Line-based proximity match
    LineProximity,
    /// Hash-based ultimate fallback
    HashFallback,
}

/// Resolves overrides using multiple strategies
pub struct OverrideResolver {
    storage: OverrideStorage,
    detector: FunctionDetector,
    fuzzy_matcher: SkimMatcherV2,
}

impl OverrideResolver {
    /// Create a new resolver
    pub fn new(storage: OverrideStorage) -> Result<Self> {
        Ok(Self {
            storage,
            detector: FunctionDetector::new()?,
            fuzzy_matcher: SkimMatcherV2::default(),
        })
    }

    /// Resolve an override using all available strategies
    pub fn resolve(
        &mut self,
        context: &FunctionContext,
    ) -> Result<Option<(OverrideEntry, ResolutionStrategy)>> {
        // Try each strategy in order
        let strategies = [
            ResolutionStrategy::Exact,
            ResolutionStrategy::Fallback,
            ResolutionStrategy::FuzzyFunction,
            ResolutionStrategy::LineProximity,
            ResolutionStrategy::HashFallback,
        ];

        for strategy in strategies {
            if let Some(entry) = self.try_resolve_with_strategy(context, strategy)? {
                log::debug!("Resolved override using strategy: {strategy:?}");
                return Ok(Some((entry, strategy)));
            }
        }

        Ok(None)
    }

    /// Try to resolve with a specific strategy
    fn try_resolve_with_strategy(
        &mut self,
        context: &FunctionContext,
        strategy: ResolutionStrategy,
    ) -> Result<Option<OverrideEntry>> {
        match strategy {
            ResolutionStrategy::Exact => self.resolve_exact(context),
            ResolutionStrategy::Fallback => self.resolve_fallback(context),
            ResolutionStrategy::FuzzyFunction => self.resolve_fuzzy_function(context),
            ResolutionStrategy::LineProximity => self.resolve_line_proximity(context),
            ResolutionStrategy::HashFallback => self.resolve_hash_fallback(context),
        }
    }

    /// Try exact match on primary key
    fn resolve_exact(&self, context: &FunctionContext) -> Result<Option<OverrideEntry>> {
        let key = OverrideKey::new(context)?;
        self.storage.get_by_primary_key(&key.primary)
    }

    /// Try fallback keys
    fn resolve_fallback(&self, context: &FunctionContext) -> Result<Option<OverrideEntry>> {
        let key = OverrideKey::new(context)?;

        for fallback in &key.fallbacks {
            if let Some(entry) = self.storage.get_by_key(fallback)? {
                return Ok(Some(entry));
            }
        }

        Ok(None)
    }

    /// Try fuzzy matching on function name
    fn resolve_fuzzy_function(
        &mut self,
        context: &FunctionContext,
    ) -> Result<Option<OverrideEntry>> {
        // Need a function name for fuzzy matching
        let func_name = match &context.function_name {
            Some(name) => name,
            None => return Ok(None),
        };

        // Get all overrides for this file
        let file_overrides = self.storage.get_by_file(&context.file_path)?;

        // Find best fuzzy match
        let mut best_match = None;
        let mut best_score = 0;

        for entry in file_overrides {
            if let Some(stored_func) = &entry.metadata.function_name {
                if let Some(score) = self.fuzzy_matcher.fuzzy_match(stored_func, func_name) {
                    if score > best_score {
                        best_score = score;
                        best_match = Some(entry);
                    }
                }
            }
        }

        // Only accept good matches (threshold)
        if best_score > 50 {
            Ok(best_match)
        } else {
            Ok(None)
        }
    }

    /// Try to find override by line proximity
    fn resolve_line_proximity(
        &mut self,
        context: &FunctionContext,
    ) -> Result<Option<OverrideEntry>> {
        // Read the source file
        let source = match std::fs::read_to_string(&context.file_path) {
            Ok(s) => s,
            Err(_) => return Ok(None),
        };

        // Find function at the current line
        let current_func = match self
            .detector
            .find_function_at_line(&source, context.line_number)?
        {
            Some(f) => f,
            None => return Ok(None),
        };

        // Look for overrides with matching function name
        let file_overrides = self.storage.get_by_file(&context.file_path)?;

        for entry in file_overrides {
            if let Some(stored_func) = &entry.metadata.function_name {
                if stored_func == &current_func.name {
                    return Ok(Some(entry));
                }
            }
        }

        Ok(None)
    }

    /// Ultimate fallback using hash
    fn resolve_hash_fallback(&self, context: &FunctionContext) -> Result<Option<OverrideEntry>> {
        let key = OverrideKey::new(context)?;

        // Find the hash key in fallbacks
        if let Some(hash_key) = key.fallbacks.iter().find(|k| k.starts_with("hash:")) {
            return self.storage.get_by_key(hash_key);
        }

        Ok(None)
    }

    /// Get resolution candidates for debugging
    pub fn get_resolution_candidates(
        &mut self,
        context: &FunctionContext,
    ) -> Result<Vec<(OverrideEntry, ResolutionStrategy, f32)>> {
        let mut candidates = Vec::new();

        // Check each strategy
        for strategy in [
            ResolutionStrategy::Exact,
            ResolutionStrategy::Fallback,
            ResolutionStrategy::FuzzyFunction,
            ResolutionStrategy::LineProximity,
            ResolutionStrategy::HashFallback,
        ] {
            if let Some(entry) = self.try_resolve_with_strategy(context, strategy)? {
                let confidence = match strategy {
                    ResolutionStrategy::Exact => 1.0,
                    ResolutionStrategy::Fallback => 0.9,
                    ResolutionStrategy::FuzzyFunction => 0.7,
                    ResolutionStrategy::LineProximity => 0.6,
                    ResolutionStrategy::HashFallback => 0.5,
                };

                candidates.push((entry, strategy, confidence));
            }
        }

        Ok(candidates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OverrideMetadata;
    use raz_config::CommandOverride;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_exact_resolution() {
        let temp_dir = TempDir::new().unwrap();
        let storage = OverrideStorage::new(temp_dir.path()).unwrap();
        let mut resolver = OverrideResolver::new(storage).unwrap();

        // Create and save an override
        let context = FunctionContext {
            file_path: PathBuf::from("src/main.rs"),
            function_name: Some("test_func".to_string()),
            line_number: 10,
            context: None,
        };

        let key = OverrideKey::new(&context).unwrap();
        let override_config = CommandOverride::new("test".to_string());
        let entry = OverrideEntry {
            key: key.clone(),
            override_config,
            metadata: OverrideMetadata {
                created_at: chrono::Utc::now(),
                modified_at: chrono::Utc::now(),
                file_path: context.file_path.clone(),
                function_name: context.function_name.clone(),
                original_line: Some(context.line_number),
                notes: None,
                validation_status: crate::ValidationStatus::Pending,
                failure_count: 0,
                last_execution_success: None,
                last_execution_time: None,
            },
        };

        resolver.storage.save(&entry).unwrap();

        // Resolve should find it with exact strategy
        let result = resolver.resolve(&context).unwrap();
        assert!(result.is_some());

        let (resolved_entry, strategy) = result.unwrap();
        assert_eq!(strategy, ResolutionStrategy::Exact);
        assert_eq!(resolved_entry.key.primary, key.primary);
    }
}
