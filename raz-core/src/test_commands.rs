//! Enhanced test command generation with precise targeting

use crate::{
    Command, CommandBuilder, CommandCategory, ProjectContext, RazResult, TestContext,
    TestTargetType,
};

/// Enhanced test command generator
pub struct TestCommandGenerator;

impl TestCommandGenerator {
    /// Generate enhanced test commands from test context
    pub fn generate_commands(
        _context: &ProjectContext,
        test_context: Option<&TestContext>,
    ) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        if let Some(test_ctx) = test_context {
            // Generate precise test command
            if let Some(precise_cmd) = Self::build_precise_cargo_test(_context, test_ctx)? {
                commands.push(precise_cmd);
            }

            // Generate nextest variant if available
            if let Some(nextest_cmd) = Self::build_precise_nextest(_context, test_ctx)? {
                commands.push(nextest_cmd);
            }
        }

        Ok(commands)
    }

    /// Build precise cargo test command targeting specific test function
    fn build_precise_cargo_test(
        _context: &ProjectContext,
        test_ctx: &TestContext,
    ) -> RazResult<Option<Command>> {
        let Some(test_name) = &test_ctx.test_name else {
            return Ok(None);
        };

        let mut args = vec!["test".to_string()];

        // Add package specification
        if let Some(ref package) = test_ctx.package_name {
            args.push("--package".to_string());
            args.push(package.clone());
        }

        // Add target specification
        match &test_ctx.target_type {
            TestTargetType::Lib => args.push("--lib".to_string()),
            TestTargetType::Bin(name) => {
                args.push("--bin".to_string());
                args.push(name.clone());
            }
            TestTargetType::Test(name) => {
                args.push("--test".to_string());
                args.push(name.clone());
            }
            TestTargetType::Bench(name) => {
                args.push("--bench".to_string());
                args.push(name.clone());
            }
            TestTargetType::Example(name) => {
                args.push("--example".to_string());
                args.push(name.clone());
            }
        }

        // Add features
        if !test_ctx.features.is_empty() {
            args.push("--features".to_string());
            args.push(test_ctx.features.join(","));
        }

        // Add test filter separator
        args.push("--".to_string());

        // Add full test path
        if let Some(full_path) = test_ctx.full_test_path() {
            args.push(full_path);
        } else {
            args.push(test_name.clone());
        }

        // Add exact match and output flags as defaults
        // These are placed after -- where they belong
        args.push("--exact".to_string());
        args.push("--show-output".to_string());

        let label = format!("Run Test: {test_name}");
        let description = format!("Run the '{test_name}' test function with precise targeting");

        let mut builder = CommandBuilder::new("cargo-test-precise", "cargo")
            .label(label)
            .description(description)
            .args(args)
            .category(CommandCategory::Test)
            .priority(100)
            .tag("test")
            .tag("precise")
            .tag("current");

        // Set working directory
        if let Some(ref working_dir) = test_ctx.working_dir {
            builder = builder.cwd(working_dir.clone());
        }

        Ok(Some(builder.build()))
    }

    /// Build precise nextest command targeting specific test function
    fn build_precise_nextest(
        _context: &ProjectContext,
        test_ctx: &TestContext,
    ) -> RazResult<Option<Command>> {
        let Some(test_name) = &test_ctx.test_name else {
            return Ok(None);
        };

        let mut args = vec!["nextest".to_string(), "run".to_string()];

        // Add package specification
        if let Some(ref package) = test_ctx.package_name {
            args.push("-p".to_string());
            args.push(package.clone());
        }

        // Add target specification
        match &test_ctx.target_type {
            TestTargetType::Lib => args.push("--lib".to_string()),
            TestTargetType::Bin(name) => {
                args.push("--bin".to_string());
                args.push(name.clone());
            }
            TestTargetType::Test(name) => {
                args.push("--test".to_string());
                args.push(name.clone());
            }
            TestTargetType::Bench(name) => {
                args.push("--bench".to_string());
                args.push(name.clone());
            }
            TestTargetType::Example(name) => {
                args.push("--example".to_string());
                args.push(name.clone());
            }
        }

        // Add features
        if !test_ctx.features.is_empty() {
            args.push("--features".to_string());
            args.push(test_ctx.features.join(","));
        }

        // Use filterset expression for precise targeting
        if let Some(full_path) = test_ctx.full_test_path() {
            args.push("-E".to_string());
            args.push(format!("test({full_path})"));
        } else {
            args.push("-E".to_string());
            args.push(format!("test({test_name})"));
        }

        let label = format!("Run Test (nextest): {test_name}");
        let description =
            format!("Run the '{test_name}' test function with nextest and precise targeting");

        let builder = CommandBuilder::new("nextest-test-precise", "cargo")
            .label(label)
            .description(description)
            .args(args)
            .category(CommandCategory::Test)
            .priority(95)
            .tag("test")
            .tag("nextest")
            .tag("precise")
            .tag("current");

        Ok(Some(builder.build()))
    }
}
