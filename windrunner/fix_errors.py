import os
import re

def fix_doctest():
    path = "crates/core/src/command/builder/bazel/doctest_builder.rs"
    with open(path, "r") as f:
        t = f.read()
    # add missing imports
    t = t.replace("use crate::bazel::BazelTargetKind;", "use crate::RunnableKind;\nuse crate::bazel::BazelTargetFinder;")
    # remove doc comment
    t = t.replace("    /// Build a command from a framework configuration\n}\n", "}\n")
    with open(path, "w") as f:
        f.write(t)

def fix_bazel_builder():
    path = "crates/core/src/command/builder/bazel/bazel_builder.rs"
    with open(path, "r") as f:
        t = f.read()
    t = t.replace("    pub(crate) fn build(", "    fn build(")
    with open(path, "w") as f:
        f.write(t)

def fix_test_builder():
    path = "crates/core/src/command/builder/bazel/test_builder.rs"
    with open(path, "r") as f:
        t = f.read()
    t = t.replace("use crate::bazel::BazelTargetKind;", "use crate::RunnableKind;")
    with open(path, "w") as f:
        f.write(t)

def fix_other_builders():
    for name in ["binary_builder.rs", "benchmark_builder.rs"]:
        path = f"crates/core/src/command/builder/bazel/{name}"
        with open(path, "r") as f:
            t = f.read()
        t = t.replace("use crate::bazel::BazelTargetKind;\n", "")
        with open(path, "w") as f:
            f.write(t)

fix_doctest()
fix_bazel_builder()
fix_test_builder()
fix_other_builders()
