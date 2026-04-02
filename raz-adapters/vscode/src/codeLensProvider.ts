import * as vscode from 'vscode';
import { detectBuildSystem, deriveBazelTarget } from './utils/buildSystem';

// ──────────────────────────────────────────────────────────────────────────────
// Regexes
// ──────────────────────────────────────────────────────────────────────────────
const FN_RE   = /^\s*(?:pub\s+)?(?:async\s+)?fn\s+([a-zA-Z_]\w*)\s*\(/;
const TEST_RE = /^\s*#\[(?:.*::)?test\]/;
const DOC_RE  = /^\s*\/\/\/.*```\s*(?:rust)?$/;   // start of a doc-test block

// ──────────────────────────────────────────────────────────────────────────────
// CodeLens provider
// ──────────────────────────────────────────────────────────────────────────────
export class RazCodeLensProvider implements vscode.CodeLensProvider {
    private _onDidChangeCodeLenses = new vscode.EventEmitter<void>();
    public readonly onDidChangeCodeLenses = this._onDidChangeCodeLenses.event;

    constructor() {
        vscode.workspace.onDidChangeConfiguration(() => this._onDidChangeCodeLenses.fire());
    }

    public provideCodeLenses(
        document: vscode.TextDocument,
        _token: vscode.CancellationToken,
    ): vscode.CodeLens[] {
        const config = vscode.workspace.getConfiguration('raz');
        if (!config.get<boolean>('enableCodeLens', true)) { return []; }

        const filePath = document.uri.fsPath;
        const buildSystem = detectBuildSystem(filePath);
        const isBazel = buildSystem === 'bazel';
        const bazelTarget = isBazel ? deriveBazelTarget(filePath) : undefined;

        const lenses: vscode.CodeLens[] = [];
        const lines = document.getText().split('\n');
        let hasTestMacro = false;
        let inDocTest = false;

        for (let i = 0; i < lines.length; i++) {
            const line = lines[i];

            // Track doc-test code fences (``` / ```rust)
            if (DOC_RE.test(line)) { inDocTest = true; continue; }
            if (inDocTest && /^\s*\/\/\/.*```\s*$/.test(line)) { inDocTest = false; continue; }

            if (TEST_RE.test(line)) { hasTestMacro = true; continue; }

            const fnMatch = FN_RE.exec(line);
            if (!fnMatch) { continue; }

            const fnName   = fnMatch[1];
            const isTest   = hasTestMacro || fnName.startsWith('test_') || fnName.endsWith('_test');
            const isMain   = fnName === 'main';
            hasTestMacro   = false;   // reset for next fn

            if (!isTest && !isMain) { continue; }

            const range = new vscode.Range(i, 0, i, 0);

            if (isBazel && isTest) {
                // ── E1: Bazel test target tooltip ────────────────────────────
                const target    = bazelTarget ?? '//…:test';
                const testLabel = `${target} --test_arg=--exact --test_arg=${fnName}`;
                const tooltip   =
                    `RAZ · Bazel test\n` +
                    `bazel test ${testLabel} --test_output=streamed\n` +
                    `Override via .cargo-runner.json → bazel.test_framework`;

                lenses.push(new vscode.CodeLens(range, {
                    title: `$(flame) Run (Bazel)`,
                    tooltip,
                    command: 'raz.runSpecificOverride',
                    arguments: [filePath, i + 1],
                }));

                lenses.push(new vscode.CodeLens(range, {
                    title: '⚙️ Override',
                    command: 'raz.editOverride',
                    arguments: [filePath, i + 1, 'args', ''],
                }));

            } else if (isBazel && isMain) {
                // ── E2: Doc-test limitation warning for Bazel ────────────────
                const target = bazelTarget ?? '//';
                const tooltip =
                    `RAZ · Bazel binary\n` +
                    `bazel run ${target} --test_output=streamed\n` +
                    `WARNING: Doc-tests are not supported by Bazel. Convert doc-tests to #[test] functions instead.`;

                lenses.push(new vscode.CodeLens(range, {
                    title: `$(flame) Run (Bazel)`,
                    tooltip,
                    command: 'raz.runSpecificOverride',
                    arguments: [filePath, i + 1],
                }));

            } else {
                // ── Standard Cargo CodeLens ──────────────────────────────────
                lenses.push(new vscode.CodeLens(range, {
                    title: '🚀 Run with RAZ',
                    command: 'raz.runSpecificOverride',
                    arguments: [filePath, i + 1],
                }));

                lenses.push(new vscode.CodeLens(range, {
                    title: '⚙️ Override',
                    command: 'raz.editOverride',
                    arguments: [filePath, i + 1, 'args', ''],
                }));
            }
        }

        return lenses;
    }

    public resolveCodeLens(lens: vscode.CodeLens): vscode.CodeLens {
        return lens;
    }
}
