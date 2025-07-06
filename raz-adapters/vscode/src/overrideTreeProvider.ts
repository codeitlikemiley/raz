import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
// @ts-expect-error toml module doesn't have types
import * as toml from 'toml';
import { getAllRazDirectories } from './utils/workspace';

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
        } else if (element.contextValue === 'razProject') {
            // For projects, show overrides
            return Promise.resolve(this.getProjectChildren(element.filePath!));
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

    private getProjectChildren(projectPath: string): OverrideItem[] {
        // Simply get the overrides for this project
        return this.getProjectOverrides(projectPath);
    }
    
    private getProjectOverrides(projectPath: string): OverrideItem[] {
        const overridePath = path.join(projectPath, '.raz', 'overrides.toml');
        if (!fs.existsSync(overridePath)) {
            return [];
        }
        
        try {
            const content = fs.readFileSync(overridePath, 'utf8');
            const data = toml.parse(content);
            const items: OverrideItem[] = [];
            
            if (data.overrides) {
                for (const [, override] of Object.entries(data.overrides as Record<string, OverrideEntry>)) {
                    const metadata = override.metadata;
                    const functionName = metadata.function_name || 'unknown';
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