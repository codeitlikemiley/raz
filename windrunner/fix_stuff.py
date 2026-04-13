# 1. Update detector.rs to use `_source` and `_scopes`
path = "crates/core/src/patterns/detector.rs"
with open(path, "r") as f:
    text = f.read()

text = text.replace("let (runnables, source, scopes) = if let Some(r) = cached {", "let (runnables, _source, _scopes) = if let Some(r) = cached {")
with open(path, "w") as f:
    f.write(text)

# 2. Update common.rs
path_common = "crates/core/src/runners/common.rs"
with open(path_common, "r") as f:
    t = f.read()

t = t.replace("parser::{module_resolver::ModuleResolver, rust_parser::RustParser}", "parser::module_resolver::ModuleResolver")
with open(path_common, "w") as f:
    f.write(t)
