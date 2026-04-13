import re
import os

filepath = "crates/core/src/command/builder/bazel/bazel_builder.rs"
with open(filepath, "r") as f:
    text = f.read()

# find all `    fn ` inside `impl BazelCommandBuilder {` and replace them with `    pub(crate) fn `
text = text.replace("    fn ", "    pub(crate) fn ")
with open(filepath, "w") as f:
    f.write(text)

# Also fix the extracted files
for name in ["binary_builder.rs", "test_builder.rs", "benchmark_builder.rs", "doctest_builder.rs"]:
    path = f"crates/core/src/command/builder/bazel/{name}"
    with open(path, "r") as f:
        t = f.read()
    t = t.replace("    fn ", "    pub(crate) fn ")
    with open(path, "w") as f:
        f.write(t)
