import re

# 1. Clean up unified_runner.rs
path_ur = "crates/core/src/runners/unified_runner.rs"
with open(path_ur, "r") as f:
    t = f.read()

# remove double import
t = t.replace("use super::{bazel_runner::BazelRunner, cargo_runner::CargoRunner, traits::CommandRunner};", "use super::traits::CommandRunner;")

# remove duplicate init_runners
while t.count("fn init_runners() -> Result<HashMap") > 1:
    old = t
    t = re.sub(r"fn init_runners\(\) -> Result<HashMap<.*?Ok\(runners\)\n}\n", "", t, count=1, flags=re.DOTALL)
    if old == t:
        break

with open(path_ur, "w") as f:
    f.write(t)


# 2. Fix build mod.rs
path_bd = "crates/core/src/command/builder/mod.rs"
with open(path_bd, "r") as f:
    t = f.read()

# change match (file_type, ...) to allow placeholder
t = t.replace("        match (file_type, &self.runnable.kind) {", "        match (file_type, &self.runnable.kind) {\n            (crate::types::FileType::Standalone, _) => unreachable!(),")

with open(path_bd, "w") as f:
    f.write(t)
