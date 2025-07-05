# RAZ Issue Templates Summary

This document summarizes the comprehensive issue reporting system we've created for the RAZ project.

## 📋 Issue Templates Created

### 1. **Bug Report** (`bug_report.md`)
- **Purpose:** General bugs and unexpected behavior
- **Key sections:** Steps to reproduce, terminal output, environment details
- **Emphasis:** Complete terminal output with the exact command being executed

### 2. **Override Issue** (`override_issue.md`) 
- **Purpose:** Problems with saving, loading, or applying overrides
- **Key sections:** Override saving/loading flows, expected vs actual commands
- **Special focus:** Command position (before/after `--` separator)

### 3. **Command Detection Issue** (`command_detection.md`)
- **Purpose:** Wrong project type detection or missing commands
- **Key sections:** Project structure, framework detection, debug output
- **Focus:** Understanding why wrong commands are generated

### 4. **VS Code Extension Issue** (`vscode_extension.md`)
- **Purpose:** Extension-specific problems
- **Key sections:** Keybindings, task runner, output channels, binary management
- **Special focus:** Extension-specific debugging steps

### 5. **Performance Issue** (`performance.md`)
- **Purpose:** Slow performance or resource usage problems
- **Key sections:** Timing data, system resources, project size metrics
- **Focus:** Quantifiable performance data

### 6. **Feature Request** (`feature_request.md`)
- **Purpose:** New features or enhancements
- **Key sections:** Problem description, proposed solution, example usage
- **Focus:** Clear use cases and implementation ideas

## 📚 Supporting Documentation

### 1. **Contributing Guide** (`CONTRIBUTING.md`)
- Comprehensive guide for contributors
- References all issue templates
- Development setup instructions
- Pull request guidelines
- Code style requirements

### 2. **Troubleshooting Guide** (`TROUBLESHOOTING.md`)
- Common issues and solutions
- Debug information collection
- Step-by-step diagnostic procedures
- Quick fixes and workarounds

### 3. **Pull Request Template** (`pull_request_template.md`)
- Structured PR format
- Testing requirements
- Documentation checklist
- Breaking changes handling

### 4. **Template Configuration** (`config.yml`)
- Disables blank issues
- Provides links to discussions and documentation
- Encourages using appropriate templates

## 🔧 Integration Points

### README Updates
- Added comprehensive "Issues and Support" section to main README
- Table of all issue templates with direct links
- Quick troubleshooting steps
- Essential information requirements

### VS Code Extension README
- Added extension-specific issue reporting section
- Quick debugging steps for extension problems
- Direct link to VS Code extension template

## 📊 Template Features

### Common Elements Across All Templates
1. **Clear categorization** - Checkboxes to identify issue type
2. **Environment information** - OS, versions, project context
3. **Reproduction steps** - Exact steps to reproduce the issue
4. **Terminal output** - Complete command execution logs
5. **Additional context** - Project files, configuration, debug info

### Special Template Features

#### Bug Report & Override Issues
- **Terminal output format** with examples
- **Before/after command comparisons**
- **Override inspection commands**

#### Command Detection Issues
- **Project structure details**
- **Framework detection information**
- **Debug logging instructions**

#### VS Code Extension Issues
- **Extension host logs**
- **Output channel inspection**
- **Binary management debugging**

#### Performance Issues
- **Timing measurements**
- **Resource usage metrics**
- **Project size considerations**

## 💡 User Experience

### For Users Reporting Issues
1. **Clear guidance** - Each template explains when to use it
2. **Complete information** - Templates ask for all needed debugging info
3. **Easy access** - Direct links from README and documentation
4. **Self-service** - Troubleshooting guide helps solve common issues

### For Maintainers
1. **Structured information** - All necessary details in consistent format
2. **Efficient triage** - Clear categorization helps route issues
3. **Reduced back-and-forth** - Templates request complete information upfront
4. **Quality control** - Links to discussions and docs reduce low-quality issues

## 🚀 Implementation Benefits

### Improved Issue Quality
- **Complete reproduction steps** - Easier to debug issues
- **Environmental context** - Better understanding of user setups  
- **Consistent format** - Standardized information collection

### Better User Support
- **Self-service debugging** - Troubleshooting guide helps users solve issues themselves
- **Targeted assistance** - Specialized templates for different issue types
- **Clear expectations** - Users know what information to provide

### Efficient Maintenance
- **Faster issue resolution** - Complete information reduces debugging time
- **Better prioritization** - Clear categorization helps triage
- **Documentation integration** - Links to relevant docs and guides

## 📈 Success Metrics

To measure the effectiveness of these templates, monitor:

1. **Issue quality** - Percentage of issues with complete information
2. **Resolution time** - Average time from issue to resolution
3. **User satisfaction** - Feedback on issue reporting process
4. **Self-service rate** - Issues resolved through troubleshooting guide
5. **Template usage** - Which templates are most/least used

## 🔄 Maintenance Notes

### Regular Updates Needed
- **Version compatibility** - Update version requirements as tools evolve
- **New frameworks** - Add framework-specific guidance as support expands
- **Common issues** - Update troubleshooting guide with new solutions
- **Template refinement** - Improve templates based on user feedback

### GitHub Configuration
- Templates are located in `.github/ISSUE_TEMPLATE/`
- Configuration file controls blank issues and contact links
- Template links in documentation should be updated if repository moves

---

This comprehensive issue reporting system provides users with clear guidance while giving maintainers the information they need to efficiently resolve issues. The structured approach reduces friction for both reporters and maintainers, leading to better overall project support.