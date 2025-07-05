## Description
<!-- Provide a clear and concise description of your changes -->

## Type of Change
<!-- Check the type of change your PR introduces -->

- [ ] 🐛 Bug fix (non-breaking change which fixes an issue)
- [ ] ✨ New feature (non-breaking change which adds functionality)  
- [ ] 💥 Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] 📚 Documentation update
- [ ] 🔧 Refactoring (no functional changes)
- [ ] ⚡ Performance improvement
- [ ] 🧪 Test improvement
- [ ] 🏗️ Build/CI changes

## Related Issues
<!-- Link any related issues -->

Fixes #(issue number)
Relates to #(issue number)

## Changes Made
<!-- List the specific changes made -->

- [ ] Change 1
- [ ] Change 2
- [ ] Change 3

## Testing
<!-- Describe how you tested your changes -->

### Test Environment
- OS: [e.g., macOS 14.5]
- Rust Version: [e.g., 1.75.0]
- VS Code Version: [e.g., 1.85.0] (if applicable)

### Test Cases
<!-- Describe the test cases you ran -->

**Manual Testing:**
- [ ] Tested with single Rust file
- [ ] Tested with Cargo package
- [ ] Tested with Cargo workspace
- [ ] Tested VS Code extension (if applicable)
- [ ] Tested override functionality (if applicable)

**Automated Testing:**
- [ ] All existing tests pass (`cargo test`)
- [ ] Added new tests for the changes
- [ ] Integration tests pass

### Test Output
<!-- If applicable, show before/after behavior -->

<details>
<summary>Before (if fixing a bug)</summary>

```bash
# Command that failed before
raz "/path/to/file.rs:line:column"
# Error output
```
</details>

<details>
<summary>After</summary>

```bash
# Same command now working
raz "/path/to/file.rs:line:column"
# Successful output
```
</details>

## Breaking Changes
<!-- If this is a breaking change, describe what breaks and migration path -->

- [ ] This PR introduces breaking changes
- [ ] Migration guide provided
- [ ] Changelog updated

**Breaking changes:**
- Change 1: [describe what breaks and how to migrate]
- Change 2: [describe what breaks and how to migrate]

## Documentation
<!-- Check if documentation needs to be updated -->

- [ ] Updated README.md
- [ ] Updated docs/ files
- [ ] Updated VS Code extension README
- [ ] Updated inline code documentation
- [ ] No documentation changes needed

## Checklist
<!-- Check off completed items -->

- [ ] Code follows the project's style guidelines
- [ ] Self-review of the code completed
- [ ] Code is commented, particularly in hard-to-understand areas
- [ ] Tests added for new functionality
- [ ] All tests pass locally
- [ ] No new warnings from `cargo clippy`
- [ ] Code formatted with `cargo fmt`
- [ ] Rebased on latest main branch

## Screenshots/Demo
<!-- If applicable, add screenshots or demo videos -->

## Additional Notes
<!-- Any additional information for reviewers -->

## Reviewer Checklist
<!-- For maintainers -->

- [ ] Code quality and style
- [ ] Test coverage adequate  
- [ ] Documentation complete
- [ ] Breaking changes justified
- [ ] Performance impact considered
- [ ] Security implications reviewed