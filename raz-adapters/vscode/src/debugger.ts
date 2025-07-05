/*---------------------------------------------------------------------------------------------
 *  RAZ VS Code Extension - Debugging Integration with rust-analyzer codelens
 *  Based on cargo-runner debugging architecture patterns
 *--------------------------------------------------------------------------------------------*/
import * as vscode from "vscode";

/**
 * Configuration for debugging integration
 */
interface DebugConfig {
	prioritySymbolKinds: vscode.SymbolKind[];
	logLevel: "debug" | "info" | "error";
	enableBreakpointDetection: boolean;
	useRustAnalyzerCodeLens: boolean;
}

/**
 * Metadata about available codelens actions for debugging
 */
interface CodelensMetadata {
	testLens: string;
	benchLens: string;
	isModule: boolean;
	runner: vscode.CodeLens | undefined;
	hasBreakpoints: boolean;
}

/**
 * Get the debug configuration from VS Code settings
 */
function getDebugConfig(): DebugConfig {
	const config = vscode.workspace.getConfiguration("raz");
	const symbolKindMap: Record<string, vscode.SymbolKind> = {
		Module: vscode.SymbolKind.Module,
		Object: vscode.SymbolKind.Object,
		Struct: vscode.SymbolKind.Struct,
		Enum: vscode.SymbolKind.Enum,
		Function: vscode.SymbolKind.Function,
	};

	return {
		prioritySymbolKinds: config
			.get<string[]>("prioritySymbolKinds", [
				"Module",
				"Object", 
				"Struct",
				"Enum",
				"Function",
			])
			.map((kind) => symbolKindMap[kind]),
		logLevel: config.get("logLevel", "error"),
		enableBreakpointDetection: config.get("enableBreakpointDetection", true),
		useRustAnalyzerCodeLens: config.get("useRustAnalyzerCodeLens", true),
	};
}

/**
 * Get relevant breakpoints within a symbol's range
 * Adapted from cargo-runner's getBreakpoints function
 */
export function getBreakpoints(
	symbol: vscode.DocumentSymbol,
	document: vscode.TextDocument,
): vscode.Breakpoint[] {
	const config = getDebugConfig();
	
	if (!config.enableBreakpointDetection) {
		return [];
	}

	return vscode.debug.breakpoints.filter((breakpoint) => {
		if (breakpoint instanceof vscode.SourceBreakpoint) {
			const { location } = breakpoint;
			const { start, end } = symbol.range;

			return (
				location.uri.toString() === document.uri.toString() &&
				location.range.start.isAfterOrEqual(start) &&
				location.range.end.isBeforeOrEqual(end)
			);
		}
		return false;
	});
}

/**
 * Find the most relevant symbol at the cursor position
 * Adapted from cargo-runner's find_symbol function
 */
export async function findRelevantSymbol(
	document: vscode.TextDocument,
	position: vscode.Position,
): Promise<vscode.DocumentSymbol | null> {
	const config = getDebugConfig();
	
	// Get document symbols from rust-analyzer
	const symbols = await vscode.commands.executeCommand<vscode.DocumentSymbol[]>(
		"vscode.executeDocumentSymbolProvider",
		document.uri,
	);

	if (!symbols || symbols.length === 0) {
		return null;
	}

	const isPositionWithinRange = (pos: vscode.Position, range: vscode.Range) =>
		pos.isAfterOrEqual(range.start) && pos.isBeforeOrEqual(range.end);

	const isPositionInSymbol = (
		pos: vscode.Position,
		symbol: vscode.DocumentSymbol,
	) =>
		isPositionWithinRange(pos, symbol.range) ||
		isPositionWithinRange(pos, symbol.selectionRange);

	const filterSymbol = (
		symbols: vscode.DocumentSymbol[],
	): vscode.DocumentSymbol | null => {
		for (const symbol of symbols) {
			if (
				config.prioritySymbolKinds.includes(symbol.kind) &&
				isPositionInSymbol(position, symbol)
			) {
				const childSymbol = filterSymbol(symbol.children);
				return childSymbol || symbol;
			}
		}
		// Fallback to main function if no priority symbol found
		return symbols.find((symbol) => symbol.name === "main") ?? null;
	};

	return filterSymbol(symbols);
}

/**
 * Get codelens actions for a specific symbol
 * Adapted from cargo-runner's getLenses function
 */
export async function getSymbolCodeLens(
	document: vscode.TextDocument,
	symbol: vscode.DocumentSymbol,
): Promise<vscode.CodeLens[]> {
	const config = getDebugConfig();
	
	if (!config.useRustAnalyzerCodeLens) {
		return [];
	}

	// Get all codelens from rust-analyzer
	const codelenses = await vscode.commands.executeCommand<vscode.CodeLens[]>(
		"vscode.executeCodeLensProvider",
		document.uri,
	);

	if (!codelenses) {
		return [];
	}

	// Filter codelens to those related to our symbol
	const symbolRelatedCodeLenses = codelenses.filter((lens) => {
		const lensStart = lens.range.start.line;
		const symbolStart = symbol.range.start.line;
		const symbolEnd = symbol.range.end.line;

		// Allow 2-line tolerance for symbol boundaries
		return lensStart >= symbolStart - 2 && lensStart <= symbolEnd;
	});

	return symbolRelatedCodeLenses;
}

/**
 * Analyze codelens and determine the best runner based on breakpoints
 * Adapted from cargo-runner's codelensMetadata function
 */
export function analyzeCodelensForDebugging(
	codeLenses: vscode.CodeLens[],
	nearestSymbol: vscode.DocumentSymbol,
	document: vscode.TextDocument,
): CodelensMetadata {
	const isModule =
		nearestSymbol?.kind === vscode.SymbolKind.Module ||
		nearestSymbol?.kind === vscode.SymbolKind.Struct;

	// Define codelens title patterns based on cargo-runner
	const testLens = isModule ? "▶︎ Run Tests" : "▶︎ Run Test";
	const benchLens = "▶︎ Run Bench";
	const runLens = "▶︎ Run";
	const docLens = "▶︎ Run Doctest";
	const debugLens = "Debug";

	// Find available runners
	const run: vscode.CodeLens | undefined = codeLenses.find(
		(lens) => lens.command?.title?.startsWith(runLens),
	);
	const bench: vscode.CodeLens | undefined = codeLenses.find(
		(lens) => lens.command?.title === benchLens,
	);
	const test: vscode.CodeLens | undefined = codeLenses.find(
		(lens) => lens.command?.title === testLens,
	);
	const doc: vscode.CodeLens | undefined = codeLenses.find(
		(lens) => lens.command?.title === docLens,
	);
	const debuggable: vscode.CodeLens | undefined = codeLenses.find(
		(lens) => lens.command?.title === debugLens,
	);

	// Get relevant breakpoints for this symbol
	const relevantBreakpoints = getBreakpoints(nearestSymbol, document);
	const hasBreakpoints = relevantBreakpoints.length > 0;

	// Core business logic: If breakpoints exist, prefer debug runner
	const runner = hasBreakpoints 
		? debuggable 
		: run || test || doc || bench;

	return {
		testLens,
		benchLens,
		isModule,
		runner,
		hasBreakpoints,
	};
}

/**
 * Check if the current context should use debugging
 */
export async function shouldUseDebugMode(
	document: vscode.TextDocument,
	position: vscode.Position,
): Promise<{ useDebug: boolean; runner?: vscode.CodeLens; symbol?: vscode.DocumentSymbol }> {
	const config = getDebugConfig();
	
	if (!config.enableBreakpointDetection || !config.useRustAnalyzerCodeLens) {
		return { useDebug: false };
	}

	// Find the relevant symbol at cursor position
	const symbol = await findRelevantSymbol(document, position);
	if (!symbol) {
		return { useDebug: false };
	}

	// Get codelens for this symbol
	const codeLenses = await getSymbolCodeLens(document, symbol);
	if (codeLenses.length === 0) {
		return { useDebug: false };
	}

	// Analyze codelens and check for breakpoints
	const metadata = analyzeCodelensForDebugging(codeLenses, symbol, document);
	
	return {
		useDebug: metadata.hasBreakpoints,
		runner: metadata.runner,
		symbol,
	};
}

/**
 * Execute a rust-analyzer command with debugging support
 */
export async function executeRustAnalyzerCommand(
	command: vscode.CodeLens,
	document: vscode.TextDocument,
	workspaceRoot?: string,
): Promise<void> {
	if (!command.command) {
		throw new Error("No command found in codelens");
	}

	// Clone the command arguments to avoid mutation
	const args = JSON.parse(JSON.stringify(command.command.arguments || []));
	
	// Set workspace root if provided
	if (workspaceRoot && args[0]?.args) {
		args[0].args.workspaceRoot = workspaceRoot;
	}

	// Execute the rust-analyzer command
	await vscode.commands.executeCommand(
		command.command.command,
		...args,
	);
}

/**
 * Main debugging integration function
 * This replaces RAZ execution when breakpoints are detected
 */
export async function executeWithDebuggingSupport(
	document: vscode.TextDocument,
	position: vscode.Position,
	fallbackToRaz: () => Promise<void>,
): Promise<void> {
	try {
		const debugInfo = await shouldUseDebugMode(document, position);
		
		if (debugInfo.useDebug && debugInfo.runner) {
			// Use rust-analyzer debugging instead of RAZ
			const workspaceFolder = vscode.workspace.getWorkspaceFolder(document.uri);
			const workspaceRoot = workspaceFolder?.uri.fsPath;
			
			await executeRustAnalyzerCommand(debugInfo.runner, document, workspaceRoot);
			
			vscode.window.showInformationMessage(
				`🐛 Debug mode: Found ${getBreakpoints(debugInfo.symbol!, document).length} breakpoint(s) in ${debugInfo.symbol?.name}`,
			);
		} else {
			// No breakpoints found, use standard RAZ execution
			await fallbackToRaz();
		}
	} catch (error) {
		console.error("Debug integration error:", error);
		// Fallback to RAZ on any error
		await fallbackToRaz();
	}
}

/**
 * Register debugging-related commands
 */
export function registerDebugCommands(context: vscode.ExtensionContext): void {
	// Command to toggle breakpoint detection
	context.subscriptions.push(
		vscode.commands.registerCommand("raz.toggleBreakpointDetection", async () => {
			const config = vscode.workspace.getConfiguration("raz");
			const current = config.get<boolean>("enableBreakpointDetection", true);
			await config.update("enableBreakpointDetection", !current, vscode.ConfigurationTarget.Global);
			
			vscode.window.showInformationMessage(
				`RAZ breakpoint detection ${!current ? "enabled" : "disabled"}`,
			);
		}),
	);

	// Command to show debug info for current position
	context.subscriptions.push(
		vscode.commands.registerCommand("raz.showDebugInfo", async () => {
			const editor = vscode.window.activeTextEditor;
			if (!editor || editor.document.languageId !== "rust") {
				vscode.window.showErrorMessage("RAZ: No Rust file is currently open");
				return;
			}

			const document = editor.document;
			const position = editor.selection.active;
			
			try {
				const symbol = await findRelevantSymbol(document, position);
				if (!symbol) {
					vscode.window.showInformationMessage("No relevant symbol found at cursor position");
					return;
				}

				const codeLenses = await getSymbolCodeLens(document, symbol);
				const metadata = analyzeCodelensForDebugging(codeLenses, symbol, document);
				const breakpoints = getBreakpoints(symbol, document);

				const info = [
					`Symbol: ${symbol.name} (${vscode.SymbolKind[symbol.kind]})`,
					`Breakpoints: ${breakpoints.length}`,
					`Available codelens: ${codeLenses.length}`,
					`Selected runner: ${metadata.runner?.command?.title || "None"}`,
					`Debug mode: ${metadata.hasBreakpoints ? "Yes" : "No"}`,
				].join("\n");

				vscode.window.showInformationMessage(info, { modal: true });
			} catch (error) {
				vscode.window.showErrorMessage(`Debug info error: ${error}`);
			}
		}),
	);
}