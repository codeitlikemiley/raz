import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { detectBuildSystem, deriveBazelTarget } from './utils/buildSystem';

/** Tracks the active build-system badge in the status bar. */
export class BuildSystemStatusBar {
    private item: vscode.StatusBarItem;

    constructor(context: vscode.ExtensionContext) {
        this.item = vscode.window.createStatusBarItem(
            vscode.StatusBarAlignment.Left,
            // Priority: sit just to the right of language mode selector
            90,
        );
        this.item.command = 'raz.showBuildSystemInfo';
        context.subscriptions.push(this.item);

        // Update whenever the active editor changes
        context.subscriptions.push(
            vscode.window.onDidChangeActiveTextEditor(editor => this.update(editor)),
        );

        // Initial paint
        this.update(vscode.window.activeTextEditor);
    }

    update(editor: vscode.TextEditor | undefined): void {
        if (!editor || editor.document.languageId !== 'rust') {
            this.item.hide();
            return;
        }

        const filePath = editor.document.uri.fsPath;
        const buildSystem = detectBuildSystem(filePath);

        switch (buildSystem) {
            case 'bazel': {
                const target = deriveBazelTarget(filePath);
                this.item.text = target
                    ? `$(flame) Bazel  ${target}`
                    : '$(flame) Bazel';
                this.item.tooltip = target
                    ? new vscode.MarkdownString(
                        `**Build system:** Bazel\n\n**Target:** \`${target}\`\n\nClick for details`,
                    )
                    : 'Build system: Bazel';
                this.item.backgroundColor = undefined;
                this.item.color = new vscode.ThemeColor('statusBarItem.warningForeground');
                break;
            }
            case 'cargo':
                this.item.text = '$(package) Cargo';
                this.item.tooltip = 'Build system: Cargo';
                this.item.color = undefined;
                this.item.backgroundColor = undefined;
                break;
            default:
                this.item.text = '$(question) Unknown build system';
                this.item.tooltip = 'No recognized build system found';
                this.item.color = undefined;
                break;
        }

        this.item.show();
    }

    dispose(): void {
        this.item.dispose();
    }
}

/** Show a quick-pick info panel about the current file's build system. */
export async function showBuildSystemInfo(filePath: string): Promise<void> {
    const buildSystem = detectBuildSystem(filePath);

    if (buildSystem === 'bazel') {
        const target = deriveBazelTarget(filePath) ?? '(target not resolved)';
        const items: vscode.QuickPickItem[] = [
            {
                label: '$(flame) Bazel project',
                description: target,
                detail: 'This file is part of a Bazel workspace.',
            },
            {
                label: '$(warning) Doc-test limitation',
                description: '',
                detail: 'Bazel does not support `cargo test --doc`. Use unit tests inside the same file instead.',
            },
            {
                label: '$(info) Run command',
                description: `bazel test ${target} --test_output=streamed`,
                detail: 'Override the test command via `.cargo-runner.json` in the package directory.',
            },
        ];
        await vscode.window.showQuickPick(items, {
            title: 'RAZ — Bazel Build Info',
            placeHolder: 'Build system details for this file',
        });
    } else if (buildSystem === 'cargo') {
        const cargoToml = findNearestCargoToml(filePath);
        await vscode.window.showInformationMessage(
            `RAZ: Cargo project${cargoToml ? ` (${path.basename(path.dirname(cargoToml))})` : ''}`,
        );
    } else {
        await vscode.window.showWarningMessage('RAZ: No recognized build system found for this file.');
    }
}

function findNearestCargoToml(filePath: string): string | undefined {
    let current = path.dirname(filePath);
    while (true) {
        const candidate = path.join(current, 'Cargo.toml');
        if (fs.existsSync(candidate)) { return candidate; }
        const parent = path.dirname(current);
        if (parent === current) { return undefined; }
        current = parent;
    }
}
