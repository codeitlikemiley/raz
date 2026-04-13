path_bd = "crates/core/src/command/builder/mod.rs"
with open(path_bd, "r") as f:
    t = f.read()

t = t.replace("                let result = BinaryCommandBuilder::build(\n                    self.runnable,\n                    self.package_name.as_deref(),\n                    &config,\n                    file_type,\n                );\n\n                result", "                BinaryCommandBuilder::build(\n                    self.runnable,\n                    self.package_name.as_deref(),\n                    &config,\n                    file_type,\n                )")

with open(path_bd, "w") as f:
    f.write(t)
