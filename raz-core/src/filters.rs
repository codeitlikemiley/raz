//! Command filtering and prioritization engine

use crate::{Command, ProjectContext, RazResult};
use chrono::Timelike;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Engine for filtering and prioritizing commands
pub struct FilterEngine {
    filters: Vec<Box<dyn CommandFilter>>,
    rules: Vec<FilterRule>,
}

impl Default for FilterEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl FilterEngine {
    pub fn new() -> Self {
        Self {
            filters: vec![
                Box::new(CategoryFilter::new()),
                Box::new(PriorityFilter::new()),
                Box::new(ConditionFilter::new()),
                Box::new(DuplicateFilter::new()),
                Box::new(TagFilter::new()),
            ],
            rules: Vec::new(),
        }
    }

    /// Apply all filters to the command list
    pub fn apply_filters(
        &self,
        commands: &[Command],
        context: &ProjectContext,
    ) -> RazResult<Vec<Command>> {
        let mut filtered_commands = commands.to_vec();

        // Apply each filter in sequence
        for filter in &self.filters {
            filtered_commands = filter.filter(filtered_commands, context)?;
        }

        // Apply custom rules
        for rule in &self.rules {
            filtered_commands = rule.apply(filtered_commands, context)?;
        }

        // Final sort by priority (descending) and then by label
        filtered_commands.sort_by(|a, b| b.priority.cmp(&a.priority).then(a.label.cmp(&b.label)));

        Ok(filtered_commands)
    }

    /// Add a custom filter
    pub fn add_filter(&mut self, filter: Box<dyn CommandFilter>) {
        self.filters.push(filter);
    }

    /// Add a custom rule
    pub fn add_rule(&mut self, rule: FilterRule) {
        self.rules.push(rule);
    }
}

/// Trait for implementing command filters
pub trait CommandFilter: Send + Sync {
    fn filter(&self, commands: Vec<Command>, context: &ProjectContext) -> RazResult<Vec<Command>>;
}

/// Filter commands based on their conditions
pub struct ConditionFilter;

impl Default for ConditionFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl ConditionFilter {
    pub fn new() -> Self {
        Self
    }
}

impl CommandFilter for ConditionFilter {
    fn filter(&self, commands: Vec<Command>, context: &ProjectContext) -> RazResult<Vec<Command>> {
        Ok(commands
            .into_iter()
            .filter(|command| command.is_available(context))
            .collect())
    }
}

/// Filter commands by category
pub struct CategoryFilter {
    excluded_categories: HashSet<String>,
}

impl Default for CategoryFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl CategoryFilter {
    pub fn new() -> Self {
        Self {
            excluded_categories: HashSet::new(),
        }
    }

    pub fn exclude_category(&mut self, category: &str) {
        self.excluded_categories.insert(category.to_string());
    }
}

impl CommandFilter for CategoryFilter {
    fn filter(&self, commands: Vec<Command>, _context: &ProjectContext) -> RazResult<Vec<Command>> {
        Ok(commands
            .into_iter()
            .filter(|command| !self.excluded_categories.contains(command.category.as_str()))
            .collect())
    }
}

/// Filter commands by minimum priority
pub struct PriorityFilter {
    min_priority: u8,
}

impl Default for PriorityFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl PriorityFilter {
    pub fn new() -> Self {
        Self { min_priority: 0 }
    }

    pub fn with_min_priority(min_priority: u8) -> Self {
        Self { min_priority }
    }
}

impl CommandFilter for PriorityFilter {
    fn filter(&self, commands: Vec<Command>, _context: &ProjectContext) -> RazResult<Vec<Command>> {
        Ok(commands
            .into_iter()
            .filter(|command| command.priority >= self.min_priority)
            .collect())
    }
}

/// Remove duplicate commands (based on command line)
pub struct DuplicateFilter;

impl Default for DuplicateFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl DuplicateFilter {
    pub fn new() -> Self {
        Self
    }
}

impl CommandFilter for DuplicateFilter {
    fn filter(&self, commands: Vec<Command>, _context: &ProjectContext) -> RazResult<Vec<Command>> {
        let mut seen = HashSet::new();
        let mut unique_commands = Vec::new();

        for command in commands {
            let command_line = command.command_line();
            if seen.insert(command_line) {
                unique_commands.push(command);
            }
        }

        Ok(unique_commands)
    }
}

/// Filter commands by tags
pub struct TagFilter {
    required_tags: HashSet<String>,
    excluded_tags: HashSet<String>,
}

impl Default for TagFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl TagFilter {
    pub fn new() -> Self {
        Self {
            required_tags: HashSet::new(),
            excluded_tags: HashSet::new(),
        }
    }

    pub fn require_tag(&mut self, tag: &str) {
        self.required_tags.insert(tag.to_string());
    }

    pub fn exclude_tag(&mut self, tag: &str) {
        self.excluded_tags.insert(tag.to_string());
    }
}

impl CommandFilter for TagFilter {
    fn filter(&self, commands: Vec<Command>, _context: &ProjectContext) -> RazResult<Vec<Command>> {
        Ok(commands
            .into_iter()
            .filter(|command| {
                // Check required tags
                if !self.required_tags.is_empty() {
                    let has_required = self
                        .required_tags
                        .iter()
                        .any(|tag| command.tags.contains(tag));
                    if !has_required {
                        return false;
                    }
                }

                // Check excluded tags
                let has_excluded = self
                    .excluded_tags
                    .iter()
                    .any(|tag| command.tags.contains(tag));

                !has_excluded
            })
            .collect())
    }
}

/// Limit the number of commands returned
pub struct LimitFilter {
    max_commands: usize,
}

impl LimitFilter {
    pub fn new(max_commands: usize) -> Self {
        Self { max_commands }
    }
}

impl CommandFilter for LimitFilter {
    fn filter(
        &self,
        mut commands: Vec<Command>,
        _context: &ProjectContext,
    ) -> RazResult<Vec<Command>> {
        commands.truncate(self.max_commands);
        Ok(commands)
    }
}

/// Custom filter rule with conditions and actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterRule {
    pub name: String,
    pub conditions: Vec<RuleCondition>,
    pub actions: Vec<RuleAction>,
    pub enabled: bool,
}

impl FilterRule {
    pub fn apply(
        &self,
        mut commands: Vec<Command>,
        context: &ProjectContext,
    ) -> RazResult<Vec<Command>> {
        if !self.enabled {
            return Ok(commands);
        }

        // Check if all conditions are met
        let conditions_met = self
            .conditions
            .iter()
            .all(|condition| condition.is_met(context));

        if !conditions_met {
            return Ok(commands);
        }

        // Apply actions
        for action in &self.actions {
            commands = action.apply(commands, context)?;
        }

        Ok(commands)
    }
}

/// Conditions for filter rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleCondition {
    /// Project has specific type
    ProjectType(String),

    /// File extension matches
    FileExtension(String),

    /// Cursor is in specific context
    CursorContext(String),

    /// Project has dependency
    HasDependency(String),

    /// Time of day condition
    TimeOfDay { start: u8, end: u8 },

    /// Custom expression
    Expression(String),
}

impl RuleCondition {
    pub fn is_met(&self, context: &ProjectContext) -> bool {
        match self {
            RuleCondition::ProjectType(project_type) => {
                format!("{:?}", context.project_type).to_lowercase() == project_type.to_lowercase()
            }
            RuleCondition::FileExtension(ext) => {
                if let Some(file_context) = &context.current_file {
                    if let Some(path_ext) = file_context.path.extension() {
                        return path_ext == ext.as_str();
                    }
                }
                false
            }
            RuleCondition::CursorContext(context_type) => {
                if let Some(file_context) = &context.current_file {
                    if let Some(symbol) = &file_context.cursor_symbol {
                        return format!("{:?}", symbol.kind).to_lowercase()
                            == context_type.to_lowercase();
                    }
                }
                false
            }
            RuleCondition::HasDependency(dep_name) => {
                context.dependencies.iter().any(|dep| dep.name == *dep_name)
            }
            RuleCondition::TimeOfDay { start, end } => {
                let now = chrono::Local::now();
                let hour = now.hour() as u8;
                hour >= *start && hour <= *end
            }
            RuleCondition::Expression(_expr) => {
                // TODO: Implement expression evaluation
                false
            }
        }
    }
}

/// Actions for filter rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleAction {
    /// Boost priority of commands matching pattern
    BoostPriority { pattern: String, boost: u8 },

    /// Filter by category
    FilterCategory(String),

    /// Limit number of commands
    Limit(usize),

    /// Add tag to matching commands
    AddTag { pattern: String, tag: String },

    /// Remove commands matching pattern
    Remove(String),

    /// Reorder by custom criteria
    Reorder(String),
}

impl RuleAction {
    pub fn apply(
        &self,
        mut commands: Vec<Command>,
        _context: &ProjectContext,
    ) -> RazResult<Vec<Command>> {
        match self {
            RuleAction::BoostPriority { pattern, boost } => {
                for command in &mut commands {
                    if command.id.contains(pattern) || command.label.contains(pattern) {
                        command.priority = command.priority.saturating_add(*boost);
                    }
                }
            }
            RuleAction::FilterCategory(category) => {
                commands.retain(|command| command.category.as_str() == category);
            }
            RuleAction::Limit(max) => {
                commands.truncate(*max);
            }
            RuleAction::AddTag { pattern, tag } => {
                for command in &mut commands {
                    if (command.id.contains(pattern) || command.label.contains(pattern))
                        && !command.tags.contains(tag)
                    {
                        command.tags.push(tag.clone());
                    }
                }
            }
            RuleAction::Remove(pattern) => {
                commands.retain(|command| {
                    !command.id.contains(pattern) && !command.label.contains(pattern)
                });
            }
            RuleAction::Reorder(_criteria) => {
                // TODO: Implement custom reordering
            }
        }

        Ok(commands)
    }
}

/// Smart filter that adapts based on context
pub struct SmartFilter {
    context_weights: std::collections::HashMap<String, f32>,
}

impl Default for SmartFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl SmartFilter {
    pub fn new() -> Self {
        let mut context_weights = std::collections::HashMap::new();

        // Default weights for different contexts
        context_weights.insert("test".to_string(), 1.5);
        context_weights.insert("debug".to_string(), 1.2);
        context_weights.insert("build".to_string(), 1.0);
        context_weights.insert("run".to_string(), 1.3);

        Self { context_weights }
    }

    fn calculate_relevance_score(&self, command: &Command, context: &ProjectContext) -> f32 {
        let mut score = command.priority as f32;

        // Boost based on current file context
        if let Some(file_context) = &context.current_file {
            if let Some(cursor_symbol) = &file_context.cursor_symbol {
                match cursor_symbol.kind {
                    crate::SymbolKind::Test => {
                        if command.category == crate::CommandCategory::Test {
                            score *= self.context_weights.get("test").unwrap_or(&1.0);
                        }
                    }
                    crate::SymbolKind::Function => {
                        if command.category == crate::CommandCategory::Run {
                            score *= self.context_weights.get("run").unwrap_or(&1.0);
                        }
                    }
                    _ => {}
                }
            }
        }

        // Boost based on project type
        match context.project_type {
            crate::ProjectType::Leptos | crate::ProjectType::Dioxus => {
                if command.tags.contains(&"serve".to_string())
                    || command.tags.contains(&"dev".to_string())
                {
                    score *= 1.4;
                }
            }
            crate::ProjectType::Bevy => {
                if command.tags.contains(&"run".to_string()) {
                    score *= 1.3;
                }
            }
            _ => {}
        }

        score
    }
}

impl CommandFilter for SmartFilter {
    fn filter(
        &self,
        mut commands: Vec<Command>,
        context: &ProjectContext,
    ) -> RazResult<Vec<Command>> {
        // Calculate relevance scores and sort
        commands.sort_by(|a, b| {
            let score_a = self.calculate_relevance_score(a, context);
            let score_b = self.calculate_relevance_score(b, context);
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(commands)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BuildTarget, Command, CommandBuilder, CommandCategory, ProjectType, TargetType,
        WorkspaceMember,
    };
    use std::path::PathBuf;

    fn create_test_context() -> ProjectContext {
        ProjectContext {
            workspace_root: PathBuf::from("/test"),
            current_file: None,
            cursor_position: None,
            project_type: ProjectType::Binary,
            dependencies: Vec::new(),
            workspace_members: vec![WorkspaceMember {
                name: "test".to_string(),
                path: PathBuf::from("/test"),
                package_type: ProjectType::Binary,
            }],
            build_targets: vec![BuildTarget {
                name: "main".to_string(),
                target_type: TargetType::Binary,
                path: PathBuf::from("/test/src/main.rs"),
            }],
            active_features: Vec::new(),
            env_vars: std::collections::HashMap::new(),
        }
    }

    fn create_test_commands() -> Vec<Command> {
        vec![
            CommandBuilder::new("build", "cargo")
                .label("Build")
                .arg("build")
                .category(CommandCategory::Build)
                .priority(80)
                .build(),
            CommandBuilder::new("test", "cargo")
                .label("Test")
                .arg("test")
                .category(CommandCategory::Test)
                .priority(90)
                .build(),
            CommandBuilder::new("run", "cargo")
                .label("Run")
                .arg("run")
                .category(CommandCategory::Run)
                .priority(85)
                .build(),
        ]
    }

    #[test]
    fn test_priority_filter() {
        let filter = PriorityFilter::with_min_priority(85);
        let commands = create_test_commands();
        let context = create_test_context();

        let filtered = filter.filter(commands, &context).unwrap();

        assert_eq!(filtered.len(), 2); // Only test (90) and run (85) should pass
        assert!(filtered.iter().all(|c| c.priority >= 85));
    }

    #[test]
    fn test_category_filter() {
        let mut filter = CategoryFilter::new();
        filter.exclude_category("test");

        let commands = create_test_commands();
        let context = create_test_context();

        let filtered = filter.filter(commands, &context).unwrap();

        assert_eq!(filtered.len(), 2); // Build and run should remain
        assert!(!filtered.iter().any(|c| c.category == CommandCategory::Test));
    }

    #[test]
    fn test_duplicate_filter() {
        let filter = DuplicateFilter::new();
        let mut commands = create_test_commands();

        // Add a duplicate - same command and args as the build command
        commands.push(
            CommandBuilder::new("build2", "cargo")
                .label("Build Duplicate")
                .arg("build") // This makes it "cargo build", same as the original build command
                .build(),
        );

        let context = create_test_context();
        let filtered = filter.filter(commands, &context).unwrap();

        assert_eq!(filtered.len(), 3); // Should remove one duplicate (4 -> 3)
    }

    #[test]
    fn test_limit_filter() {
        let filter = LimitFilter::new(2);
        let commands = create_test_commands();
        let context = create_test_context();

        let filtered = filter.filter(commands, &context).unwrap();

        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_engine() {
        let engine = FilterEngine::new();
        let commands = create_test_commands();
        let context = create_test_context();

        let filtered = engine.apply_filters(&commands, &context).unwrap();

        // All commands should pass through (no conditions set in test commands)
        assert_eq!(filtered.len(), 3);

        // Should be sorted by priority (descending)
        assert_eq!(filtered[0].priority, 90); // test
        assert_eq!(filtered[1].priority, 85); // run
        assert_eq!(filtered[2].priority, 80); // build
    }

    #[test]
    fn test_smart_filter() {
        let filter = SmartFilter::new();
        let commands = create_test_commands();
        let context = create_test_context();

        let filtered = filter.filter(commands, &context).unwrap();

        // Commands should be reordered based on relevance
        assert_eq!(filtered.len(), 3);
    }

    #[test]
    fn test_condition_filter() {
        let filter = ConditionFilter::new();
        let commands = create_test_commands();
        let context = create_test_context();

        let filtered = filter.filter(commands, &context).unwrap();

        // All commands should pass through since they have no conditions
        assert_eq!(filtered.len(), 3);
    }

    #[test]
    fn test_create_test_commands() {
        let commands = create_test_commands();
        assert_eq!(commands.len(), 3);

        // Check that all commands have empty conditions
        for command in &commands {
            assert_eq!(
                command.conditions.len(),
                0,
                "Command {} has conditions",
                command.id
            );
        }
    }

    #[test]
    fn test_step_by_step_filters() {
        let commands = create_test_commands();
        let context = create_test_context();

        println!("Initial commands: {}", commands.len());
        for cmd in &commands {
            println!("  Command: {} -> {}", cmd.id, cmd.command_line());
        }

        // Test each filter individually
        let category_filter = CategoryFilter::new();
        let after_category = category_filter.filter(commands.clone(), &context).unwrap();
        println!("After CategoryFilter: {}", after_category.len());

        let priority_filter = PriorityFilter::new();
        let after_priority = priority_filter.filter(after_category, &context).unwrap();
        println!("After PriorityFilter: {}", after_priority.len());

        let condition_filter = ConditionFilter::new();
        let after_condition = condition_filter.filter(after_priority, &context).unwrap();
        println!("After ConditionFilter: {}", after_condition.len());

        let duplicate_filter = DuplicateFilter::new();
        let after_duplicate = duplicate_filter.filter(after_condition, &context).unwrap();
        println!("After DuplicateFilter: {}", after_duplicate.len());
        for cmd in &after_duplicate {
            println!("  Remaining: {} -> {}", cmd.id, cmd.command_line());
        }

        let tag_filter = TagFilter::new();
        let after_tag = tag_filter.filter(after_duplicate, &context).unwrap();
        println!("After TagFilter: {}", after_tag.len());

        assert_eq!(after_tag.len(), 3);
    }
}
