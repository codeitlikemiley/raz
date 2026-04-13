import re

filepath = "crates/core/src/command/builder/bazel/bazel_builder.rs"
with open(filepath, "r") as f:
    text = f.read()

pattern = r"(pub\(crate\) struct LegacyExpandArgs\S* \{\n)(.*?)(^\})"
match = re.search(pattern, text, re.MULTILINE | re.DOTALL)
if match:
    fields = match.group(2)
    fields = re.sub(r"^[ ]+([a-zA-Z_]+):", r"    pub(crate) \1:", fields, flags=re.MULTILINE)
    text = text[:match.start(2)] + fields + text[match.end(2):]
    with open(filepath, "w") as f:
        f.write(text)
