import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

/**
 * Find the nearest directory containing .raz/overrides.toml
 * starting from the active file or workspace
 */
export function findNearestRazDirectory(): string | undefined {
    // First, try from the active editor's file
    const activeEditor = vscode.window.activeTextEditor;
    if (activeEditor && activeEditor.document.languageId === 'rust') {
        const filePath = activeEditor.document.uri.fsPath;
        const razDir = findRazDirectoryFrom(path.dirname(filePath));
        if (razDir) {
            return razDir;
        }
    }

    // Fall back to workspace folders
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (workspaceFolders) {
        for (const folder of workspaceFolders) {
            const razDir = findRazDirectoryFrom(folder.uri.fsPath);
            if (razDir) {
                return razDir;
            }
        }
    }

    return undefined;
}

/**
 * Find the directory containing .raz/overrides.toml starting from a given path
 */
function findRazDirectoryFrom(startPath: string): string | undefined {
    let current = startPath;
    const root = path.parse(current).root;

    while (current !== root) {
        // Check for .raz/overrides.toml
        const razOverridesPath = path.join(current, '.raz', 'overrides.toml');
        if (fs.existsSync(razOverridesPath)) {
            return current;
        }

        // Check for Cargo.toml (potential project root)
        const cargoTomlPath = path.join(current, 'Cargo.toml');
        if (fs.existsSync(cargoTomlPath)) {
            // Even if no .raz here, this could be a valid project root
            const razPath = path.join(current, '.raz');
            if (fs.existsSync(razPath)) {
                return current;
            }
        }

        // Move up one directory
        const parent = path.dirname(current);
        if (parent === current) {
            break; // Reached root
        }
        current = parent;
    }

    return undefined;
}

/**
 * Get all directories that might contain .raz configurations
 * Returns them in order of precedence (most specific first)
 */
export function getAllRazDirectories(): string[] {
    const directories: string[] = [];
    const seen = new Set<string>();

    // 1. Active file's project (if any)
    const activeEditor = vscode.window.activeTextEditor;
    if (activeEditor && activeEditor.document.languageId === 'rust') {
        const filePath = activeEditor.document.uri.fsPath;
        const razDir = findRazDirectoryFrom(path.dirname(filePath));
        if (razDir && !seen.has(razDir)) {
            directories.push(razDir);
            seen.add(razDir);
        }
    }

    // 2. Recursively scan all workspace folders for .raz directories
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (workspaceFolders) {
        for (const folder of workspaceFolders) {
            // Recursively find all .raz directories
            const razDirs = findAllRazDirectoriesRecursive(folder.uri.fsPath);
            for (const dir of razDirs) {
                if (!seen.has(dir)) {
                    directories.push(dir);
                    seen.add(dir);
                }
            }
        }
    }

    // 3. Global directory
    const homeDir = process.env.HOME || process.env.USERPROFILE;
    if (homeDir) {
        const globalRazPath = path.join(homeDir, '.raz');
        if (fs.existsSync(globalRazPath) && !seen.has(homeDir)) {
            directories.push(homeDir);
        }
    }

    return directories;
}

/**
 * Recursively find all directories containing .raz subdirectory
 */
function findAllRazDirectoriesRecursive(startPath: string, maxDepth = 5): string[] {
    const results: string[] = [];
    
    function searchDir(dir: string, depth: number) {
        if (depth > maxDepth) {
            return;
        }
        
        try {
            // Check if this directory has .raz
            const razPath = path.join(dir, '.raz');
            if (fs.existsSync(razPath) && fs.statSync(razPath).isDirectory()) {
                results.push(dir);
            }
            
            // Don't recurse into .raz directories themselves
            if (path.basename(dir) === '.raz') {
                return;
            }
            
            // Skip common directories we don't need to search
            const skipDirs = ['node_modules', 'target', '.git', 'dist', 'build', 'out'];
            if (skipDirs.includes(path.basename(dir))) {
                return;
            }
            
            // Recurse into subdirectories
            const entries = fs.readdirSync(dir, { withFileTypes: true });
            for (const entry of entries) {
                if (entry.isDirectory()) {
                    searchDir(path.join(dir, entry.name), depth + 1);
                }
            }
        } catch {
            // Ignore permission errors and other issues
        }
    }
    
    searchDir(startPath, 0);
    return results;
}

/**
 * Get the best working directory for override commands
 * Prioritizes directories with existing overrides
 */
export function getBestWorkingDirectory(): string {
    // First try to find a directory with overrides
    const razDir = findNearestRazDirectory();
    if (razDir) {
        return razDir;
    }

    // Fall back to first workspace folder
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (workspaceFolders && workspaceFolders.length > 0) {
        return workspaceFolders[0].uri.fsPath;
    }

    // Last resort: current working directory
    return process.cwd();
}