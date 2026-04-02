import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import * as os from 'os';
import * as toml from 'toml';
import { getAllRazDirectories } from './utils/workspace';
import { detectBuildSystem, deriveBazelTarget } from './utils/buildSystem';

interface OverrideEntry {
    key: {
        primary: string;
        display: string;
    };
    override_config: {
        key: string;
        cargo_options?: string[];
        rustc_options?: string[];
        args?: string[];
        mode?: string;
        env?: Record<string, string>;
    };
    metadata: {
        file_path: string;
        function_name?: string;
        original_line?: number;
    };
}

export class RazOverrideTreeProvider implements vscode.TreeDataProvider<OverrideItem> {
    private _onDidChangeTreeData: vscode.EventEmitter<OverrideItem | undefined | null | void> = new vscode.EventEmitter<OverrideItem | undefined | null | void>();
    readonly onDidChangeTreeData: vscode.Event<OverrideItem | undefined | null | void> = this._onDidChangeTreeData.event;

    constructor(private context: vscode.ExtensionContext) {}

    refresh(): void {
        this._onDidChangeTreeData.fire();
    }

    getTreeItem(element: OverrideItem): vscode.TreeItem {
        return element;
    }

    getChildren(element?: OverrideItem): Thenable<OverrideItem[]> {
        if (!element) {
            // Root level - show all projects with .raz directories
            return Promise.resolve(this.getRazProjects());
        } else if (element.contextValue === 'razProject' || element.contextValue === 'razGlobalProject') {
            // For projects, show overrides
            return Promise.resolve(this.getProjectChildren(element.filePath!, element.contextValue === 'razGlobalProject'));
        } else if (element.contextValue === 'razOverride') {
            // Show override details as children
            return Promise.resolve(this.getOverrideDetails(element));
        }
        // Workspace headers don't expand
        return Promise.resolve([]);
    }

    private getRazProjects(): OverrideItem[] {
        const razDirs = getAllRazDirectories();
        const processedPaths = new Set<string>();
        const items: OverrideItem[] = [];
        const workspaceRoots = new Set<string>();
        
        // Add global overrides section first
        const globalRazPath = path.join(os.homedir(), '.raz');
        const globalOverridePath = path.join(globalRazPath, 'overrides.toml');
        
        if (fs.existsSync(globalOverridePath)) {
            // For global overrides, the overrides.toml is directly in .raz folder
            let globalOverrideCount = 0;
            try {
                const content = fs.readFileSync(globalOverridePath, 'utf8');
                const data = toml.parse(content);
                
                if (data.overrides) {
                    // Filter global overrides to only those relevant to current workspace
                    const workspaceFolders = vscode.workspace.workspaceFolders;
                    const workspacePaths = workspaceFolders ? workspaceFolders.map(folder => folder.uri.fsPath) : [];
                    
                    let relevantCount = 0;
                    // eslint-disable-next-line @typescript-eslint/no-explicit-any
                    for (const [, override] of Object.entries(data.overrides as Record<string, any>)) {
                        if (override && override.metadata && override.metadata.file_path) {
                            const filePath = override.metadata.file_path;
                            // Check if this file is within any workspace folder
                            const isRelevant = workspacePaths.some(workspacePath => 
                                filePath.startsWith(workspacePath)
                            );
                            if (isRelevant) {
                                relevantCount++;
                            }
                        }
                    }
                    globalOverrideCount = relevantCount;
                }
            } catch (error) {
                console.error('Error counting global overrides:', error);
            }
            
            if (globalOverrideCount > 0) {
                const globalItem = new OverrideItem(
                    `Standalone (${globalOverrideCount} override${globalOverrideCount !== 1 ? 's' : ''})`,
                    vscode.TreeItemCollapsibleState.Collapsed,
                    'razGlobalProject',
                    globalRazPath
                );
                globalItem.iconPath = new vscode.ThemeIcon('globe');
                globalItem.tooltip = `${globalRazPath}\n${globalOverrideCount} standalone override${globalOverrideCount !== 1 ? 's' : ''}`;
                items.push(globalItem);
            }
        }
        
        // First pass: identify all workspace roots
        for (const dir of razDirs) {
            const cargoTomlPath = path.join(dir, 'Cargo.toml');
            if (fs.existsSync(cargoTomlPath)) {
                try {
                    const content = fs.readFileSync(cargoTomlPath, 'utf8');
                    if (content.includes('[workspace]')) {
                        workspaceRoots.add(dir);
                    }
                } catch {
                    // Ignore errors reading Cargo.toml
                }
            }
        }
        
        // Second pass: process directories
        for (const dir of razDirs) {
            if (processedPaths.has(dir)) {
                continue;
            }
            
            // Skip the global .raz directory (home directory) since we handle it separately as "Standalone"
            if (dir === os.homedir()) {
                continue;
            }
            
            // Skip if this directory is inside a workspace (will be handled by workspace)
            let isInsideWorkspace = false;
            for (const wsRoot of workspaceRoots) {
                if (dir !== wsRoot && dir.startsWith(wsRoot + path.sep)) {
                    isInsideWorkspace = true;
                    break;
                }
            }
            if (isInsideWorkspace) {
                continue;
            }
            
            const cargoTomlPath = path.join(dir, 'Cargo.toml');
            const isWorkspace = workspaceRoots.has(dir);
            
            const overrideCount = this.countOverrides(dir);
            const projectName = path.basename(dir) || path.basename(path.dirname(dir));
            
            if (isWorkspace) {
                // For workspaces, show just the name with total count
                const item = new OverrideItem(
                    `${projectName} (${overrideCount} override${overrideCount !== 1 ? 's' : ''})`,
                    vscode.TreeItemCollapsibleState.None,
                    'razWorkspaceHeader',
                    dir
                );
                
                item.iconPath = new vscode.ThemeIcon('folder-library');
                item.tooltip = `${dir}\n${overrideCount} total override${overrideCount !== 1 ? 's' : ''} in workspace`;
                
                items.push(item);
                
                // Add member crates with indentation
                if (fs.existsSync(cargoTomlPath)) {
                    try {
                        const content = fs.readFileSync(cargoTomlPath, 'utf8');
                        const membersMatch = content.match(/members\s*=\s*\[([\s\S]*?)\]/);
                        if (membersMatch) {
                            const members = membersMatch[1]
                                .split(',')
                                .map(m => m.trim().replace(/['"]/g, ''))
                                .filter(m => m.length > 0);
                            
                            for (const member of members) {
                                const memberPath = path.join(dir, member);
                                const memberOverrideCount = this.countOverrides(memberPath);
                                
                                if (memberOverrideCount > 0) {
                                    const memberName = path.basename(memberPath);
                                    const memberItem = new OverrideItem(
                                        `  ${memberName} (${memberOverrideCount} override${memberOverrideCount !== 1 ? 's' : ''})`,
                                        vscode.TreeItemCollapsibleState.Collapsed,
                                        'razProject',
                                        memberPath
                                    );
                                    memberItem.iconPath = new vscode.ThemeIcon('package');
                                    items.push(memberItem);
                                }
                            }
                        }
                    } catch {
                    // Ignore errors reading Cargo.toml
                }
                }
                
                processedPaths.add(dir);
            } else if (overrideCount > 0) {
                // Only show standalone projects that have overrides
                const item = new OverrideItem(
                    `${projectName} (${overrideCount} override${overrideCount !== 1 ? 's' : ''})`,
                    vscode.TreeItemCollapsibleState.Collapsed,
                    'razProject',
                    dir
                );
                
                item.iconPath = new vscode.ThemeIcon('folder');
                item.tooltip = `${dir}\n${overrideCount} override${overrideCount !== 1 ? 's' : ''}`;
                
                items.push(item);
                processedPaths.add(dir);
            }
        }
        
        return items;
    }

    private getProjectChildren(projectPath: string, isGlobal = false): OverrideItem[] {
        const items: OverrideItem[] = [];

        // E3 — Bazel config section
        if (!isGlobal) {
            // Pick a representative file to detect the build system
            const probe = path.join(projectPath, 'src', 'lib.rs');
            const buildSystem = detectBuildSystem(probe);

            if (buildSystem === 'bazel') {
                const target = deriveBazelTarget(probe);
                const configJson = path.join(projectPath, '.cargo-runner.json');

                // Read bazel.test_framework if present
                let testFramework = 'default';
                if (fs.existsSync(configJson)) {
                    try {
                        // eslint-disable-next-line @typescript-eslint/no-explicit-any
                        const cfg: any = JSON.parse(fs.readFileSync(configJson, 'utf8'));
                        const tf = cfg?.bazel?.test_framework;
                        if (tf?.command && tf?.subcommand) {
                            testFramework = `${tf.command} ${tf.subcommand}`;
                        }
                    } catch { /* ignore */ }
                }

                const bazelHeader = new OverrideItem(
                    `Bazel Config`,
                    vscode.TreeItemCollapsibleState.Expanded,
                    'razBazelHeader',
                    projectPath,
                );
                bazelHeader.iconPath = new vscode.ThemeIcon('flame');
                bazelHeader.tooltip = 'Bazel build system settings for this package';
                bazelHeader.description = target ?? '(target not resolved)';
                items.push(bazelHeader);

                // Target item
                const targetItem = new OverrideItem(
                    target ? `Target: ${target}` : 'Target: (not resolved)',
                    vscode.TreeItemCollapsibleState.None,
                    'razBazelTarget',
                    projectPath,
                );
                targetItem.iconPath = new vscode.ThemeIcon('symbol-constant');
                targetItem.tooltip = target
                    ? `Inferred Bazel target label.\nFull command: bazel test ${target} --test_output=streamed`
                    : 'Could not resolve BUILD label. Ensure a BUILD.bazel file exists in the package.';
                items.push(targetItem);

                // Test framework item  
                const tfItem = new OverrideItem(
                    `Test runner: ${testFramework}`,
                    vscode.TreeItemCollapsibleState.None,
                    'razBazelTestRunner',
                    path.join(projectPath, '.cargo-runner.json'),
                );
                tfItem.iconPath = new vscode.ThemeIcon('beaker');
                tfItem.tooltip = 'Configure via bazel.test_framework in .cargo-runner.json';
                tfItem.command = fs.existsSync(configJson) ? {
                    command: 'vscode.open',
                    title: 'Open config',
                    arguments: [vscode.Uri.file(configJson)],
                } : undefined;
                items.push(tfItem);

                // Doc-test limitation notice
                const docItem = new OverrideItem(
                    `⚠️  Doc-tests not supported`,
                    vscode.TreeItemCollapsibleState.None,
                    'razBazelDocTestWarning',
                );
                docItem.iconPath = new vscode.ThemeIcon('warning');
                docItem.tooltip =
                    'Bazel does not support `cargo test --doc`.\n' +
                    'Convert doc-tests to #[test] unit tests within the same module.';
                items.push(docItem);
            }
        }

        // Standard overrides
        items.push(...this.getProjectOverrides(projectPath, isGlobal));
        return items;
    }
    
    private getProjectOverrides(projectPath: string, isGlobal = false): OverrideItem[] {
        // For global overrides, the path is different
        const overridePath = isGlobal 
            ? path.join(projectPath, 'overrides.toml')
            : path.join(projectPath, '.raz', 'overrides.toml');
            
        if (!fs.existsSync(overridePath)) {
            return [];
        }
        
        try {
            const content = fs.readFileSync(overridePath, 'utf8');
            const data = toml.parse(content);
            const items: OverrideItem[] = [];
            
            if (data.overrides) {
                // Get workspace paths for filtering
                const workspaceFolders = vscode.workspace.workspaceFolders;
                const workspacePaths = workspaceFolders ? workspaceFolders.map(folder => folder.uri.fsPath) : [];
                
                // For global overrides, each key is the override ID and contains the full override object
                for (const [overrideKey, override] of Object.entries(data.overrides as Record<string, OverrideEntry>)) {
                    if (!override || typeof override !== 'object') {
                        continue;
                    }
                    
                    // For global overrides, only show those relevant to current workspace
                    if (isGlobal && override.metadata && override.metadata.file_path) {
                        const filePath = override.metadata.file_path;
                        const isRelevant = workspacePaths.some(workspacePath => 
                            filePath.startsWith(workspacePath)
                        );
                        if (!isRelevant) {
                            continue;
                        }
                    }
                    
                    const metadata = override.metadata;
                    
                    // Try to extract a better function name
                    let functionName = metadata.function_name;
                    
                    if (!functionName || functionName === 'unknown') {
                        // Try to extract from the override key
                        const keyParts = overrideKey.split(':');
                        if (keyParts.length >= 2) {
                            const lastPart = keyParts[keyParts.length - 1];
                            // Check if it looks like a function name (not just a line number)
                            if (lastPart && !lastPart.startsWith('L') && !/^\d+$/.test(lastPart)) {
                                functionName = lastPart;
                            } else if (keyParts.length >= 3) {
                                // Maybe it's file:line:function format
                                const potentialFunction = keyParts[keyParts.length - 2];
                                if (potentialFunction && !potentialFunction.startsWith('L') && !/^\d+$/.test(potentialFunction)) {
                                    functionName = potentialFunction;
                                }
                            }
                        }
                        
                        // If still no good name, check if it's a doctest or test
                        if (!functionName || functionName === 'unknown') {
                            const overrideConfigKey = override.override_config?.key || '';
                            const isTestRelated = overrideKey.includes('doctest') || 
                                                  overrideConfigKey === 'test' || 
                                                  overrideConfigKey.includes('test') ||
                                                  overrideKey.includes(':L'); // Line-based overrides are often doctests
                                                  
                            if (isTestRelated) {
                                // Try to extract from file path for doctests
                                const filePath = metadata.file_path || '';
                                const fileName = filePath.split('/').pop()?.replace('.rs', '') || 'doctest';
                                functionName = `${fileName}_doctest`;
                            } else {
                                functionName = 'unknown';
                            }
                        }
                    }
                    
                    const line = metadata.original_line || 0;
                    
                    // Create main override item with just function name
                    const label = functionName;
                    
                    const item = new OverrideItem(
                        label,
                        vscode.TreeItemCollapsibleState.Collapsed,
                        'razOverride',
                        metadata.file_path,
                        line,
                        undefined,
                        override // Store the full override data
                    );
                    
                    // Set icon based on override type
                    const overrideType = override.override_config.key;
                    item.iconPath = new vscode.ThemeIcon(
                        overrideType === 'cargo' ? 'package' : 
                        overrideType === 'test' ? 'beaker' : 
                        'settings-gear'
                    );
                    
                    // Build tooltip with detailed info and instructions
                    const tooltipLines = [
                        `Function: ${functionName}`,
                        `File: ${metadata.file_path}`,
                        `Line: ${line}`,
                        `Type: ${overrideType}`,
                        '',
                        '💡 Click to navigate to code',
                        '💡 Use action buttons to Run or Edit'
                    ];
                    
                    if (override.override_config.cargo_options?.length) {
                        tooltipLines.splice(4, 0, `Cargo options: ${override.override_config.cargo_options.join(' ')}`);
                    }
                    if (override.override_config.args?.length) {
                        tooltipLines.splice(4, 0, `Args: ${override.override_config.args.join(' ')}`);
                    }
                    if (override.override_config.mode) {
                        tooltipLines.splice(4, 0, `Mode: ${override.override_config.mode}`);
                    }
                    
                    item.tooltip = tooltipLines.join('\n');
                    
                    // Add command to open the file at the specific line
                    item.command = {
                        command: 'vscode.open',
                        title: 'Open',
                        arguments: [
                            vscode.Uri.file(metadata.file_path),
                            {
                                selection: new vscode.Range(
                                    new vscode.Position(line - 1, 0),
                                    new vscode.Position(line - 1, 0)
                                )
                            }
                        ]
                    };
                    
                    items.push(item);
                }
            }
            
            console.log(`Returning ${items.length} items for ${isGlobal ? 'global' : 'project'} overrides`);
            return items.sort((a, b) => a.label.localeCompare(b.label));
        } catch (error) {
            console.error('Failed to parse overrides:', error);
            return [];
        }
    }

    private getOverrideDetails(parent: OverrideItem): OverrideItem[] {
        const items: OverrideItem[] = [];
        
        if (!parent.overrideData) {
            return items;
        }
        
        const override = parent.overrideData;
        
        // Build the full override string for display
        let overrideString = '';
        
        // Add environment variables
        if (override.override_config.env && Object.keys(override.override_config.env).length > 0) {
            const envVars = Object.entries(override.override_config.env)
                .map(([key, value]) => `${key}=${value}`)
                .join(' ');
            overrideString += envVars + ' ';
        }
        
        // Add cargo options
        if (override.override_config.cargo_options?.length) {
            overrideString += override.override_config.cargo_options.join(' ') + ' ';
        }
        
        // Add args (with -- prefix)
        if (override.override_config.args?.length) {
            overrideString += '-- ' + override.override_config.args.join(' ');
        }
        
        overrideString = overrideString.trim() || '-- --test-threads=1';
        
        // Create a single item with play icon and args
        const overrideItem = new OverrideItem(
            `▶  ${overrideString}`,
            vscode.TreeItemCollapsibleState.None,
            'razOverrideRunnable',
            parent.filePath,
            parent.line,
            undefined,
            override
        );
        
        // No icon since we have ▶ in the label
        
        // Primary action is run (when clicking the item)
        overrideItem.command = {
            command: 'raz.runSpecificOverride',
            title: 'Run',
            arguments: [parent.filePath, parent.line]
        };
        
        // Tooltip explains the actions
        overrideItem.tooltip = new vscode.MarkdownString(
            `**Override:** \`${overrideString}\`\n\n` +
            `• **Click** to run with this override\n` +
            `• **Right-click** → Edit to modify`
        );
        
        items.push(overrideItem);
        
        return items;
    }
    
    private countOverrides(projectPath: string): number {
        let totalCount = 0;
        
        // First, check if this directory itself has overrides
        const overridePath = path.join(projectPath, '.raz', 'overrides.toml');
        if (fs.existsSync(overridePath)) {
            try {
                const content = fs.readFileSync(overridePath, 'utf8');
                const data = toml.parse(content);
                
                if (data.overrides) {
                    // Count unique overrides (each override has multiple sub-sections like .key, .override_config, etc.)
                    const overrideKeys = new Set<string>();
                    for (const key of Object.keys(data.overrides)) {
                        // Extract the base key (before .key, .override_config, etc.)
                        const baseKey = key.split('.')[0];
                        overrideKeys.add(baseKey);
                    }
                    totalCount += overrideKeys.size;
                }
            } catch (error) {
                console.error('Error counting overrides:', error);
            }
        }
        
        // Check if this is a workspace by looking for Cargo.toml with [workspace]
        const cargoTomlPath = path.join(projectPath, 'Cargo.toml');
        if (fs.existsSync(cargoTomlPath)) {
            try {
                const cargoContent = fs.readFileSync(cargoTomlPath, 'utf8');
                if (cargoContent.includes('[workspace]')) {
                    // This is a workspace, count overrides in all member crates
                    const membersMatch = cargoContent.match(/members\s*=\s*\[([\s\S]*?)\]/);
                    if (membersMatch) {
                        const members = membersMatch[1]
                            .split(',')
                            .map(m => m.trim().replace(/['"]/g, ''))
                            .filter(m => m.length > 0);
                        
                        // Recursively count overrides in each member
                        for (const member of members) {
                            const memberPath = path.join(projectPath, member);
                            if (fs.existsSync(memberPath)) {
                                totalCount += this.countOverrides(memberPath);
                            }
                        }
                    }
                }
            } catch (error) {
                console.error('Error reading Cargo.toml:', error);
            }
        }
        
        return totalCount;
    }
}

class OverrideItem extends vscode.TreeItem {
    constructor(
        public readonly label: string,
        public readonly collapsibleState: vscode.TreeItemCollapsibleState,
        public readonly contextValue: string,
        public readonly filePath?: string,
        public readonly line?: number,
        description?: string,
        public readonly overrideData?: OverrideEntry
    ) {
        super(label, collapsibleState);
        if (filePath) {
            this.resourceUri = vscode.Uri.file(filePath);
        }
        if (description) {
            this.description = description;
        }
    }
}