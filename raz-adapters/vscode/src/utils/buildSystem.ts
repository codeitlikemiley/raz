import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

/** Detect the build system for the given Rust file by walking ancestors. */
export function detectBuildSystem(filePath: string): 'bazel' | 'cargo' | 'unknown' {
    for (const ancestor of ancestors(filePath)) {
        if (fs.existsSync(path.join(ancestor, 'BUILD.bazel')) ||
            fs.existsSync(path.join(ancestor, 'BUILD'))) {
            return 'bazel';
        }
        if (fs.existsSync(path.join(ancestor, 'Cargo.toml'))) {
            return 'cargo';
        }
    }
    return 'unknown';
}

/** Derive the Bazel target label for a file, e.g. //server:server */
export function deriveBazelTarget(filePath: string): string | undefined {
    for (const ancestor of ancestors(filePath)) {
        const buildFile = [
            path.join(ancestor, 'BUILD.bazel'),
            path.join(ancestor, 'BUILD'),
        ].find(f => fs.existsSync(f));

        if (buildFile) {
            // Find the workspace root (contains MODULE.bazel or WORKSPACE)
            const wsRoot = findWorkspaceRoot(ancestor);
            if (!wsRoot) { return undefined; }

            const relDir = path.relative(wsRoot, ancestor).replace(/\\/g, '/');
            const pkgLabel = relDir === '' ? '//' : `//${relDir}`;

            // Try to infer a target name from the nearest Cargo.toml package name
            const cargoToml = path.join(ancestor, 'Cargo.toml');
            const targetName = readPackageName(cargoToml) ?? path.basename(ancestor);
            return `${pkgLabel}:${targetName}`;
        }
    }
    return undefined;
}

function findWorkspaceRoot(startDir: string): string | undefined {
    for (const ancestor of ancestors(startDir)) {
        if (fs.existsSync(path.join(ancestor, 'MODULE.bazel')) ||
            fs.existsSync(path.join(ancestor, 'WORKSPACE')) ||
            fs.existsSync(path.join(ancestor, 'WORKSPACE.bazel'))) {
            return ancestor;
        }
    }
    return undefined;
}

function readPackageName(cargoTomlPath: string): string | undefined {
    try {
        if (!fs.existsSync(cargoTomlPath)) { return undefined; }
        const content = fs.readFileSync(cargoTomlPath, 'utf8');
        // Scope to [package] section only
        const pkgSection = content.split(/^\s*\[/m).find(s => s.startsWith('package]'));
        if (!pkgSection) { return undefined; }
        const match = pkgSection.match(/name\s*=\s*"([^"]+)"/);
        return match?.[1];
    } catch {
        return undefined;
    }
}

function* ancestors(filePath: string): Generator<string> {
    let current = path.dirname(filePath);
    while (true) {
        yield current;
        const parent = path.dirname(current);
        if (parent === current) { break; }
        current = parent;
    }
}
