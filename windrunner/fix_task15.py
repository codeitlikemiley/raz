import re
import os

# 1. Update RunnableDetector to cache Source & Scopes.
path = "crates/core/src/patterns/detector.rs"
with open(path, "r") as f:
    text = f.read()

text = text.replace(
    "struct CacheEntry {\n    mtime: SystemTime,\n    runnables: Vec<Runnable>,\n}",
    "struct CacheEntry {\n    mtime: SystemTime,\n    runnables: Vec<Runnable>,\n    source: String,\n    scopes: Vec<Scope>,\n}"
)

# And in detect_runnables, update cache logic
replacement1 = """        let cached = PARSE_CACHE.with(|c| {
            if let Some(entry) = c.borrow().get(file_path) {
                if entry.mtime == mtime {
                    return Some((entry.runnables.clone(), entry.source.clone(), entry.scopes.clone()));
                }
            }
            None
        });

        let (runnables, source, scopes) = if let Some(r) = cached {
            r
        } else {"""

text = text.replace("""        let cached = PARSE_CACHE.with(|c| {
            if let Some(entry) = c.borrow().get(file_path) {
                if entry.mtime == mtime {
                    return Some(entry.runnables.clone());
                }
            }
            None
        });

        let runnables = if let Some(r) = cached {
            r
        } else {""", replacement1)


replacement2 = """        PARSE_CACHE.with(|c| {
            c.borrow_mut().insert(
                file_path.to_path_buf(),
                CacheEntry {
                    mtime,
                    runnables: runnables.clone(),
                    source: source.clone(),
                    scopes: extended_scopes.iter().map(|e| e.scope.clone()).collect(),
                },
            );
        });

        (runnables, source, extended_scopes.into_iter().map(|e| e.scope).collect())
    };"""

text = text.replace("""        PARSE_CACHE.with(|c| {
            c.borrow_mut().insert(
                file_path.to_path_buf(),
                CacheEntry {
                    mtime,
                    runnables: runnables.clone(),
                },
            );
        });

        runnables
    };""", replacement2)

# Expose get_cached_scopes
text = text.replace("    pub fn get_best_runnable_at_line(", "    pub fn get_cached_scopes(&mut self, file_path: &Path) -> Result<Vec<Scope>> {\n        self.detect_runnables(file_path, None)?;\n        \n        let mtime = std::fs::metadata(file_path).and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH);\n        let cached = PARSE_CACHE.with(|c| {\n            if let Some(entry) = c.borrow().get(file_path) {\n                if entry.mtime == mtime {\n                    return Some(entry.scopes.clone());\n                }\n            }\n            None\n        });\n        \n        Ok(cached.unwrap_or_default())\n    }\n\n    pub fn get_best_runnable_at_line(")
with open(path, "w") as f:
    f.write(text)


# 2. Update common.rs
path_common = "crates/core/src/runners/common.rs"
with open(path_common, "r") as f:
    t = f.read()

t = t.replace(
    "pub fn resolve_module_paths(\n    runnables: &mut [Runnable],\n    file_path: &Path,\n    package_name: Option<&str>,\n) -> Result<()> {",
    "pub fn resolve_module_paths(\n    runnables: &mut [Runnable],\n    file_path: &Path,\n    package_name: Option<&str>,\n    detector: &mut crate::patterns::RunnableDetector,\n) -> Result<()> {"
)

t = re.sub(
    r"    // Parse the file to get all scopes for module resolution\n    let source = std::fs::read_to_string.*?\n    let mut parser = RustParser::new\(\)\?;\n    let scopes = parser.get_scopes\(&source, file_path\)\?;",
    "    // Get cached scopes from detector\n    let scopes = detector.get_cached_scopes(file_path)?;",
    t,
    flags=re.DOTALL
)

t = t.replace(
    "pub fn resolve_module_path_single(\n    runnable: &mut Runnable,\n    file_path: &Path,\n    package_name: Option<&str>,\n) -> Result<()> {\n    resolve_module_paths(std::slice::from_mut(runnable), file_path, package_name)\n}",
    "pub fn resolve_module_path_single(\n    runnable: &mut Runnable,\n    file_path: &Path,\n    package_name: Option<&str>,\n    detector: &mut crate::patterns::RunnableDetector,\n) -> Result<()> {\n    resolve_module_paths(std::slice::from_mut(runnable), file_path, package_name, detector)\n}"
)
with open(path_common, "w") as f:
    f.write(t)


# 3. Update cargo_runner.rs
path_cr = "crates/core/src/runners/cargo_runner.rs"
with open(path_cr, "r") as f:
    t = f.read()

t = t.replace("resolve_module_paths(&mut runnables, file_path, package_name.as_deref())?;", "resolve_module_paths(&mut runnables, file_path, package_name.as_deref(), &mut detector)?;")
t = t.replace("resolve_module_path_single(&mut runnable, file_path, package_name.as_deref())?;", "resolve_module_path_single(&mut runnable, file_path, package_name.as_deref(), &mut detector)?;")

with open(path_cr, "w") as f:
    f.write(t)

# 4. Update bazel_runner.rs
path_br = "crates/core/src/runners/bazel_runner.rs"
with open(path_br, "r") as f:
    t = f.read()

t = t.replace("resolve_module_paths(&mut runnables, file_path, None)?;", "resolve_module_paths(&mut runnables, file_path, None, &mut detector)?;")
t = t.replace("resolve_module_path_single(&mut runnable, file_path, None)?;", "resolve_module_path_single(&mut runnable, file_path, None, &mut detector)?;")

with open(path_br, "w") as f:
    f.write(t)
