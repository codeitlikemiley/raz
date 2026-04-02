/*---------------------------------------------------------------------------------------------
 *  RAZ VS Code Extension — Bazel transparent-proxy commands
 *
 *  Provides IDE-level wrappers for the four CLI sub-commands added in Phase 1:
 *    cargo runner sync       → raz.bazelSync
 *    cargo runner add        → raz.bazelAdd
 *    cargo runner build-sync → raz.buildSync
 *    cargo runner init --bazel → raz.initBazel
 *
 *  Also exports `registerCargoTomlWatcher` which watches Cargo.toml changes inside
 *  Bazel workspaces and prompts the user to run `cargo runner sync`.
 *--------------------------------------------------------------------------------------------*/

import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { exec } from 'node:child_process';
import { promisify } from 'node:util';
import { detectBuildSystem } from './utils/buildSystem';

const execAsync = promisify(exec);

// ── helpers ──────────────────────────────────────────────────────────────────

/** Return the first workspace root, or undefined. */
function wsRoot(): string | undefined {
    return vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
}

/**
 * Find the cargo-runner binary on PATH.
 * Uses the user's "raz.cargoRunnerPath" setting if set, otherwise resolves
 * from PATH via `which cargo-runner`.
 */
async function resolveCargoRunner(): Promise<string> {
    const config = vscode.workspace.getConfiguration('raz');
    const customPath = config.get<string>('cargoRunnerPath');
    if (customPath && fs.existsSync(customPath)) {
        return customPath;
    }
    // Fallback: assume it is on PATH as installed by `cargo install cargo-runner`
    return 'cargo-runner';
}

/** Run a cargo-runner sub-command in the given cwd, stream output to the RAZ channel. */
async function runCargoRunnerCommand(
    channel: vscode.OutputChannel,
    args: string[],
    cwd: string,
    title: string,
): Promise<void> {
    const bin = await resolveCargoRunner();
    const fullCmd = [bin, ...args].join(' ');

    channel.show(true);
    channel.appendLine('');
    channel.appendLine(`▶ ${fullCmd}  (cwd: ${cwd})`);
    channel.appendLine('─'.repeat(72));

    await vscode.window.withProgress(
        {
            location: vscode.ProgressLocation.Notification,
            title: `RAZ · ${title}`,
            cancellable: false,
        },
        async (progress) => {
            progress.report({ message: 'Running…' });

            try {
                const { stdout, stderr } = await execAsync(fullCmd, {
                    cwd,
                    // Inherit the login shell PATH so cargo / bazel are findable
                    shell: process.env.SHELL ?? '/bin/sh',
                    env: { ...process.env },
                });

                if (stdout) { channel.appendLine(stdout); }
                if (stderr) { channel.appendLine(stderr); }

                channel.appendLine(`✅ Done`);
            } catch (err: unknown) {
                const error = err as { stdout?: string; stderr?: string; message?: string };
                if (error.stdout) { channel.appendLine(error.stdout); }
                if (error.stderr) { channel.appendLine(error.stderr); }
                channel.appendLine(`❌ Failed: ${error.message ?? err}`);
                throw err;
            }
        },
    );
}

// ── exported command registrations ───────────────────────────────────────────

/**
 * raz.bazelSync — cargo runner sync [--crate <name>] [--skip-ide]
 *
 * Runs: cargo update → bazel sync → gen_rust_project for the whole workspace,
 * or scoped to a specific crate if the user provides one.
 */
export function registerBazelSync(
    context: vscode.ExtensionContext,
    channel: vscode.OutputChannel,
): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('raz.bazelSync', async () => {
            const root = wsRoot();
            if (!root) {
                vscode.window.showErrorMessage('RAZ: No workspace folder open.');
                return;
            }

            // Optional: narrow to a specific crate
            const crateName = await vscode.window.showInputBox({
                title: 'RAZ · Bazel Sync',
                prompt: 'Crate name to sync (leave empty to sync entire workspace)',
                placeHolder: 'e.g., server  (empty = all)',
            });
            if (crateName === undefined) { return; } // user pressed Esc

            const args = ['sync'];
            if (crateName.trim() !== '') {
                args.push('--crate', crateName.trim());
            }

            try {
                await runCargoRunnerCommand(channel, args, root, 'Bazel Sync');
                vscode.window.showInformationMessage('RAZ: Bazel sync complete ✅');
            } catch {
                vscode.window.showErrorMessage('RAZ: Bazel sync failed — see RAZ output channel for details.');
            }
        }),
    );
    channel.appendLine('✅ raz.bazelSync registered');
}

/**
 * raz.bazelAdd — cargo runner add <crate> [--features f1,f2] [--dev] [--crate-dir <dir>]
 *
 * Prompts for crate name + features, runs `cargo runner add`, then shows
 * a summary of what was installed.
 */
export function registerBazelAdd(
    context: vscode.ExtensionContext,
    channel: vscode.OutputChannel,
): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('raz.bazelAdd', async () => {
            const root = wsRoot();
            if (!root) {
                vscode.window.showErrorMessage('RAZ: No workspace folder open.');
                return;
            }

            const crateName = await vscode.window.showInputBox({
                title: 'RAZ · Add Crate (Bazel)',
                prompt: 'Crate name to add (e.g. tokio)',
                placeHolder: 'tokio',
                validateInput: (v) => v.trim() === '' ? 'Crate name is required' : undefined,
            });
            if (!crateName || crateName.trim() === '') { return; }

            const features = await vscode.window.showInputBox({
                title: 'RAZ · Add Crate — Features',
                prompt: 'Features to enable (comma-separated, leave empty for none)',
                placeHolder: 'full,rt-multi-thread',
            });
            if (features === undefined) { return; }

            const devChoice = await vscode.window.showQuickPick(
                [
                    { label: 'Normal dependency', description: '[dependencies]', value: false },
                    { label: 'Dev dependency',    description: '[dev-dependencies]', value: true },
                ],
                { title: 'RAZ · Add Crate — Dependency type' },
            );
            if (!devChoice) { return; }

            // Optionally scope to a specific crate directory
            const crateDir = await vscode.window.showInputBox({
                title: 'RAZ · Add Crate — Target crate directory',
                prompt: 'Relative path to the crate (leave empty = workspace root)',
                placeHolder: 'crates/server',
            });
            if (crateDir === undefined) { return; }

            const args = ['add', crateName.trim()];
            if (features.trim() !== '') { args.push('--features', features.trim()); }
            if (devChoice.value)         { args.push('--dev'); }
            if (crateDir.trim() !== '')  { args.push('--crate-dir', crateDir.trim()); }

            try {
                await runCargoRunnerCommand(channel, args, root, `Add ${crateName}`);
                vscode.window.showInformationMessage(`RAZ: Added \`${crateName}\` and synced Bazel ✅`);
            } catch {
                vscode.window.showErrorMessage(`RAZ: Failed to add \`${crateName}\` — see RAZ output channel.`);
            }
        }),
    );
    channel.appendLine('✅ raz.bazelAdd registered');
}

/**
 * raz.buildSync — cargo runner build-sync [--crate <name>] [--dry-run]
 *
 * Scans the crate's src layout and scaffolds / updates BUILD.bazel targets
 * inside the managed block.  A dry-run preview is shown first.
 */
export function registerBuildSync(
    context: vscode.ExtensionContext,
    channel: vscode.OutputChannel,
): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('raz.buildSync', async () => {
            const root = wsRoot();
            if (!root) {
                vscode.window.showErrorMessage('RAZ: No workspace folder open.');
                return;
            }

            const crateName = await vscode.window.showInputBox({
                title: 'RAZ · Build Sync',
                prompt: 'Crate name to scaffold (leave empty for current directory)',
                placeHolder: 'e.g., server',
            });
            if (crateName === undefined) { return; }

            const previewChoice = await vscode.window.showQuickPick(
                [
                    { label: '$(search) Preview (--dry-run)',  description: 'Show what would change without writing', dryRun: true  },
                    { label: '$(file-add) Apply changes',      description: 'Write updated BUILD.bazel targets',      dryRun: false },
                ],
                { title: 'RAZ · Build Sync — Mode' },
            );
            if (!previewChoice) { return; }

            const args = ['build-sync'];
            if (crateName.trim() !== '')  { args.push('--crate', crateName.trim()); }
            if (previewChoice.dryRun)     { args.push('--dry-run'); }

            const label = previewChoice.dryRun ? 'Build Sync (dry-run)' : 'Build Sync';
            try {
                await runCargoRunnerCommand(channel, args, root, label);
                const suffix = previewChoice.dryRun ? '(dry-run — no files written)' : '✅';
                vscode.window.showInformationMessage(`RAZ: BUILD.bazel scaffold complete ${suffix}`);
            } catch {
                vscode.window.showErrorMessage('RAZ: build-sync failed — see RAZ output channel.');
            }
        }),
    );
    channel.appendLine('✅ raz.buildSync registered');
}

/**
 * raz.initBazel — cargo runner init --bazel [--workspace-name <name>]
 *
 * Generates a `.cargo-runner.json` with Bazel framework defaults pre-populated.
 */
export function registerInitBazel(
    context: vscode.ExtensionContext,
    channel: vscode.OutputChannel,
): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('raz.initBazel', async () => {
            const root = wsRoot();
            if (!root) {
                vscode.window.showErrorMessage('RAZ: No workspace folder open.');
                return;
            }

            const configPath = path.join(root, '.cargo-runner.json');
            if (fs.existsSync(configPath)) {
                const overwrite = await vscode.window.showWarningMessage(
                    'RAZ: .cargo-runner.json already exists. Overwrite with Bazel config?',
                    { modal: true },
                    'Overwrite',
                    'Cancel',
                );
                if (overwrite !== 'Overwrite') { return; }
            }

            const wsName = await vscode.window.showInputBox({
                title: 'RAZ · Init Bazel Config',
                prompt: 'Bazel workspace name (leave empty to use directory name)',
                placeHolder: path.basename(root),
            });
            if (wsName === undefined) { return; }

            const args = ['init', '--bazel', '--force'];
            if (wsName.trim() !== '') { args.push('--workspace-name', wsName.trim()); }

            try {
                await runCargoRunnerCommand(channel, args, root, 'Init Bazel Config');
                vscode.window.showInformationMessage('RAZ: .cargo-runner.json created with Bazel defaults ✅');
            } catch {
                vscode.window.showErrorMessage('RAZ: init --bazel failed — see RAZ output channel.');
            }
        }),
    );
    channel.appendLine('✅ raz.initBazel registered');
}

/**
 * registerCargoTomlWatcher
 *
 * Watches every Cargo.toml in the workspace.  When one is saved inside a
 * directory that also has a BUILD.bazel (i.e. a Bazel-managed crate), the
 * user is prompted to run `cargo runner sync` immediately.
 *
 * The prompt is debounced: if multiple files are saved in quick succession
 * only one notification fires.
 */
export function registerCargoTomlWatcher(
    context: vscode.ExtensionContext,
    channel: vscode.OutputChannel,
): void {
    let debounceTimer: ReturnType<typeof setTimeout> | undefined;

    // FileSystemWatcher fires for create/change/delete events
    const cargoWatcher = vscode.workspace.createFileSystemWatcher('**/Cargo.toml');

    const onCargoChange = (uri: vscode.Uri) => {
        const fileDir = path.dirname(uri.fsPath);
        const buildSystem = detectBuildSystem(uri.fsPath);

        if (buildSystem !== 'bazel') {
            // Not in a Bazel workspace — do nothing
            return;
        }

        // Respect the user setting
        const config = vscode.workspace.getConfiguration('raz');
        if (!config.get<boolean>('bazelAutoSync', true)) {
            channel.appendLine(`[watcher] Cargo.toml changed (bazelAutoSync disabled): ${uri.fsPath}`);
            return;
        }

        channel.appendLine(`[watcher] Cargo.toml changed: ${uri.fsPath}`);

        // Debounce: only fire ≥1 second after the last save
        if (debounceTimer) { clearTimeout(debounceTimer); }
        debounceTimer = setTimeout(async () => {
            const root = wsRoot() ?? fileDir;
            const relativePath = path.relative(root, uri.fsPath);

            const choice = await vscode.window.showInformationMessage(
                `RAZ: \`${relativePath}\` changed in a Bazel workspace. Sync Bazel crate-universe?`,
                'Sync Now',
                'Later',
            );

            if (choice === 'Sync Now') {
                try {
                    await runCargoRunnerCommand(channel, ['sync'], root, 'Bazel Sync');
                    vscode.window.showInformationMessage('RAZ: Bazel sync complete ✅');
                } catch {
                    vscode.window.showErrorMessage('RAZ: Sync failed — see RAZ output channel.');
                }
            }
        }, 1_200);
    };

    cargoWatcher.onDidChange(onCargoChange);
    cargoWatcher.onDidCreate(onCargoChange);
    context.subscriptions.push(cargoWatcher);

    channel.appendLine('✅ Cargo.toml Bazel watcher registered');
}

/**
 * Extend the existing `showBuildSystemInfo` quick-pick for Bazel files to
 * include action items for the new commands.
 *
 * Call this instead of (or before) the original showBuildSystemInfo so the
 * extra actions appear at the top.
 */
export async function showBazelActions(filePath: string): Promise<void> {
    const items: (vscode.QuickPickItem & { action?: string })[] = [
        {
            label: '$(sync) Sync dependencies',
            description: 'cargo runner sync',
            detail: 'Run after `cargo add` or editing Cargo.toml — updates Bazel crate-universe',
            action: 'sync',
        },
        {
            label: '$(package) Add crate',
            description: 'cargo runner add',
            detail: 'Add an external crate and auto-sync Bazel in one step',
            action: 'add',
        },
        {
            label: '$(file-add) Scaffold BUILD.bazel',
            description: 'cargo runner build-sync',
            detail: 'Detect new source files and generate missing Bazel targets',
            action: 'buildSync',
        },
        {
            label: '$(gear) Init Bazel config',
            description: 'cargo runner init --bazel',
            detail: 'Generate .cargo-runner.json with Bazel framework defaults',
            action: 'initBazel',
        },
        { label: '', kind: vscode.QuickPickItemKind.Separator },
        {
            label: '$(flame) Bazel project info',
            description: filePath,
            detail: 'Build system: Bazel',
            action: 'info',
        },
        {
            label: '$(warning) Doc-test limitation',
            description: '',
            detail: 'Bazel does not support `cargo test --doc`. Use #[test] functions inside the module instead.',
            action: undefined,
        },
    ];

    const picked = await vscode.window.showQuickPick(items, {
        title: 'RAZ — Bazel Workspace Actions',
        placeHolder: 'Choose an action or view build info',
        matchOnDescription: true,
        matchOnDetail: true,
    });

    if (!picked?.action) { return; }

    switch (picked.action) {
        case 'sync':      await vscode.commands.executeCommand('raz.bazelSync');   break;
        case 'add':       await vscode.commands.executeCommand('raz.bazelAdd');    break;
        case 'buildSync': await vscode.commands.executeCommand('raz.buildSync');   break;
        case 'initBazel': await vscode.commands.executeCommand('raz.initBazel');   break;
        case 'info':      /* informational only */                                 break;
    }
}
