import re
import os

filepath = "crates/core/src/command/builder/bazel/bazel_builder.rs"
with open(filepath, "r") as f:
    text = f.read()

# We want to extract methods of `impl BazelCommandBuilder` into their own files
# but we need to put `impl BazelCommandBuilder { ... }` in each file.

binary_methods = ["build_binary_command"]
test_methods = ["build_test_command", "build_module_tests_command"]
benchmark_methods = ["build_benchmark_command"]
doctest_methods = ["build_doc_test_command"]

def extract_methods(text, method_names):
    methods_code = []
    for m in method_names:
        # Regex to find the method
        # It starts with `    fn {m}(` and ends when we see the next `    fn ` or `}` at the same indentation level.
        pattern = r"(?sm)(^[ ]{4}fn " + m + r"\(.*?(?=^[ ]{4}fn |^}$))"
        match = re.search(pattern, text)
        if match:
            methods_code.append(match.group(1))
            # replace the matched code with empty string in the original text
            text = text[:match.start()] + text[match.end():]
        else:
            print(f"Could not find method {m}")
    return text, methods_code

text, binary_code = extract_methods(text, binary_methods)
text, test_code = extract_methods(text, test_methods)
text, benchmark_code = extract_methods(text, benchmark_methods)
text, doctest_code = extract_methods(text, doctest_methods)

# Now we need to create the new files

def write_file(filename, methods_code):
    with open(filename, "w") as f:
        f.write("use super::*;\n")
        f.write("use crate::command::CargoCommand;\n")
        f.write("use crate::config::{BazelConfig, Config};\n")
        f.write("use crate::error::Result;\n")
        f.write("use crate::types::{FileType, Runnable};\n")
        f.write("use crate::bazel::BazelTargetKind;\n")
        f.write("\n")
        f.write("impl BazelCommandBuilder {\n")
        for code in methods_code:
            f.write(code)
            f.write("\n")
        f.write("}\n")

os.makedirs("crates/core/src/command/builder/bazel", exist_ok=True)
write_file("crates/core/src/command/builder/bazel/binary_builder.rs", binary_code)
write_file("crates/core/src/command/builder/bazel/test_builder.rs", test_code)
write_file("crates/core/src/command/builder/bazel/benchmark_builder.rs", benchmark_code)
write_file("crates/core/src/command/builder/bazel/doctest_builder.rs", doctest_code)

# We also need to add `mod` declarations into `bazel/mod.rs`
mod_rs = "crates/core/src/command/builder/bazel/mod.rs"
with open(mod_rs, "r") as f:
    mod_text = f.read()

if "mod binary_builder;" not in mod_text:
    mod_text = mod_text.replace("pub mod bazel_builder;", "pub mod bazel_builder;\nmod binary_builder;\nmod test_builder;\nmod benchmark_builder;\nmod doctest_builder;")
    with open(mod_rs, "w") as f:
        f.write(mod_text)

with open(filepath, "w") as f:
    f.write(text)

