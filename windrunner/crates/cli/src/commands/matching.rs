use cargo_runner_core::{Runnable, RunnableKind};

/// Normalize a user query or candidate text for comparison.
///
/// The comparison ignores punctuation, whitespace, and case so that
/// `foo bar`, `foo_bar`, and `FooBar` collapse to the same key.
pub fn normalize_query(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Return the symbol-like name for a runnable when it has one.
///
/// This is intentionally narrow:
/// - `DocTest` returns the struct/module name, but not method doc tests.
/// - `ModuleTests` returns the module name.
/// - `Binary` returns the binary name when available.
pub fn runnable_symbol_name(runnable: &Runnable) -> Option<String> {
    match &runnable.kind {
        RunnableKind::DocTest {
            struct_or_module_name,
            method_name,
        } if method_name.is_none() => Some(struct_or_module_name.clone()),
        RunnableKind::ModuleTests { module_name } => Some(module_name.clone()),
        RunnableKind::Binary { bin_name } => bin_name.clone(),
        _ => None,
    }
}

/// Build the most specific runnable selector string we can derive.
pub fn runnable_full_selector(runnable: &Runnable) -> Option<String> {
    let function_name = runnable.get_function_name()?;
    if runnable.module_path.is_empty() {
        Some(function_name)
    } else {
        Some(format!("{}::{}", runnable.module_path, function_name))
    }
}

/// Return the set of candidate strings used for text matching.
pub fn runnable_candidate_texts(runnable: &Runnable) -> Vec<String> {
    let mut candidates = Vec::new();

    candidates.push(runnable.label.clone());

    if !runnable.module_path.is_empty() {
        candidates.push(runnable.module_path.clone());
    }

    if let Some(function_name) = runnable.get_function_name() {
        candidates.push(function_name);
    }

    if let Some(symbol_name) = runnable_symbol_name(runnable) {
        candidates.push(symbol_name);
    }

    if let Some(full_selector) = runnable_full_selector(runnable) {
        candidates.push(full_selector);
    }

    candidates.sort();
    candidates.dedup();
    candidates
}

/// Check whether a runnable matches a user-provided text query.
pub fn runnable_matches_query(runnable: &Runnable, query: Option<&str>, exact: bool) -> bool {
    let Some(query) = query else {
        return true;
    };

    let query = normalize_query(query);
    if query.is_empty() {
        return true;
    }

    runnable_candidate_texts(runnable)
        .into_iter()
        .map(|candidate| normalize_query(&candidate))
        .any(|candidate| {
            if exact {
                candidate == query
            } else {
                candidate.contains(&query)
            }
        })
}

/// Rank how strongly a runnable matches an execution selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SelectorMatchRank {
    FullSelector = 0,
    FunctionName = 1,
    SymbolName = 2,
    ModulePath = 3,
    Label = 4,
}

/// Return the best exact match rank for a selector against a runnable.
///
/// This keeps `cargo runner run foo` deterministic:
/// - exact full selector wins first
/// - then function name
/// - then symbol name
/// - then module path
/// - then label
pub fn selector_match_rank(selector: &str, runnable: &Runnable) -> Option<SelectorMatchRank> {
    let selector = normalize_query(selector);
    if selector.is_empty() {
        return None;
    }

    let check = |candidate: Option<String>, rank: SelectorMatchRank| {
        candidate
            .map(|candidate| normalize_query(&candidate) == selector)
            .unwrap_or(false)
            .then_some(rank)
    };

    check(
        runnable_full_selector(runnable),
        SelectorMatchRank::FullSelector,
    )
    .or_else(|| {
        check(
            runnable.get_function_name(),
            SelectorMatchRank::FunctionName,
        )
    })
    .or_else(|| {
        check(
            runnable_symbol_name(runnable),
            SelectorMatchRank::SymbolName,
        )
    })
    .or_else(|| {
        check(
            (!runnable.module_path.is_empty()).then_some(runnable.module_path.clone()),
            SelectorMatchRank::ModulePath,
        )
    })
    .or_else(|| check(Some(runnable.label.clone()), SelectorMatchRank::Label))
}
