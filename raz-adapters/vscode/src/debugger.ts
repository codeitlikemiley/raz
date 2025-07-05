/*---------------------------------------------------------------------------------------------
 *  RAZ VS Code Extension - Simple debugging integration via rust-analyzer codelens
 *--------------------------------------------------------------------------------------------*/
import * as vscode from "vscode";

/**
 * Get breakpoints in the current symbol's range
 */
function getBreakpoints(symbol: vscode.DocumentSymbol, document: vscode.TextDocument): vscode.Breakpoint[] {
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
 * Find relevant symbol at cursor position
 */
async function findSymbol(document: vscode.TextDocument, position: vscode.Position): Promise<vscode.DocumentSymbol | null> {
	const symbols = await vscode.commands.executeCommand<vscode.DocumentSymbol[]>(
		"vscode.executeDocumentSymbolProvider",
		document.uri,
	);

	if (!symbols || symbols.length === 0) {
		return null;
	}

	const isPositionInSymbol = (pos: vscode.Position, symbol: vscode.DocumentSymbol) =>
		pos.isAfterOrEqual(symbol.range.start) && pos.isBeforeOrEqual(symbol.range.end);

	// Prioritize function/method symbols over variables
	const findInSymbols = (symbols: vscode.DocumentSymbol[]): vscode.DocumentSymbol | null => {
		let foundSymbol: vscode.DocumentSymbol | null = null;
		
		for (const symbol of symbols) {
			if (isPositionInSymbol(position, symbol)) {
				// If it's a function/method, prefer it
				if (symbol.kind === vscode.SymbolKind.Function || 
					symbol.kind === vscode.SymbolKind.Method) {
					return symbol;
				}
				
				// Otherwise, keep looking for children
				const childSymbol = findInSymbols(symbol.children);
				if (childSymbol && 
					(childSymbol.kind === vscode.SymbolKind.Function || 
					 childSymbol.kind === vscode.SymbolKind.Method)) {
					return childSymbol;
				}
				
				// Fall back to any symbol if no function found
				foundSymbol = childSymbol || symbol;
			}
		}
		return foundSymbol;
	};

	return findInSymbols(symbols);
}

/**
 * Get codelens for the current document
 */
async function getCodeLens(document: vscode.TextDocument): Promise<vscode.CodeLens[]> {
	const codelenses = await vscode.commands.executeCommand<vscode.CodeLens[]>(
		"vscode.executeCodeLensProvider",
		document.uri,
	);
	return codelenses || [];
}

/**
 * Main debugging integration - check for breakpoints and use Debug codelens if found
 */
export async function executeWithDebuggingSupport(
	document: vscode.TextDocument,
	position: vscode.Position,
	fallbackToRaz: () => Promise<void>,
	outputChannel: vscode.OutputChannel,
): Promise<void> {
	outputChannel.appendLine(`[RAZ Debug] executeWithDebuggingSupport called`);
	
	const config = vscode.workspace.getConfiguration("raz");
	const enableBreakpointDetection = config.get<boolean>("enableBreakpointDetection", true);
	
	outputChannel.appendLine(`[RAZ Debug] Breakpoint detection enabled: ${enableBreakpointDetection}`);
	
	if (!enableBreakpointDetection) {
		outputChannel.appendLine(`[RAZ Debug] Breakpoint detection disabled, using RAZ`);
		await fallbackToRaz();
		return;
	}

	try {
		// Find symbol at cursor
		const symbol = await findSymbol(document, position);
		outputChannel.appendLine(`[RAZ Debug] Symbol found: ${symbol ? `${symbol.name} (${vscode.SymbolKind[symbol.kind]})` : 'none'}`);
		
		// No symbol or not a function/method - use RAZ
		if (!symbol || 
			(symbol.kind !== vscode.SymbolKind.Function && 
			 symbol.kind !== vscode.SymbolKind.Method)) {
			outputChannel.appendLine(`[RAZ Debug] No function/method found at cursor, using RAZ`);
			await fallbackToRaz();
			return;
		}

		// Check for breakpoints in this symbol
		outputChannel.appendLine(`[RAZ Debug] Symbol "${symbol.name}" range: ${symbol.range.start.line}:${symbol.range.start.character} - ${symbol.range.end.line}:${symbol.range.end.character}`);
		
		// Get ALL breakpoints in the document first
		const allBreakpoints = vscode.debug.breakpoints.filter(bp => 
			bp instanceof vscode.SourceBreakpoint && 
			bp.location.uri.toString() === document.uri.toString()
		);
		outputChannel.appendLine(`[RAZ Debug] Total breakpoints in document: ${allBreakpoints.length}`);
		allBreakpoints.forEach((bp, i) => {
			if (bp instanceof vscode.SourceBreakpoint) {
				outputChannel.appendLine(`[RAZ Debug] Breakpoint ${i}: line ${bp.location.range.start.line}:${bp.location.range.start.character}`);
			}
		});
		
		const breakpoints = getBreakpoints(symbol, document);
		outputChannel.appendLine(`[RAZ Debug] Breakpoints in ${symbol.name}: ${breakpoints.length}`);
		if (breakpoints.length === 0) {
			outputChannel.appendLine(`[RAZ Debug] No breakpoints found in symbol range, using RAZ`);
			await fallbackToRaz();
			return;
		}

		// Get codelens and find Debug command
		const codelenses = await getCodeLens(document);
		outputChannel.appendLine(`[RAZ Debug] Total codelenses: ${codelenses.length}`);
		
		// DUMP ALL CODELENS FOR DEBUGGING
		outputChannel.appendLine(`[RAZ Debug] ALL CODELENS:`);
		codelenses.forEach((lens, index) => {
			outputChannel.appendLine(`[RAZ Debug] Codelens ${index}: ${lens.command?.title} | ${lens.command?.command} | line ${lens.range.start.line} | symbolRange: ${symbol.range.start.line} - ${symbol.range.end.line}`);
		});
		
		// Find Debug codelens for this symbol - same logic as cargo-runner
		const debugCodeLens = codelenses.find(lens => 
			lens.command?.title?.includes("Debug") &&
			lens.range.start.line >= symbol.range.start.line - 2 &&
			lens.range.start.line <= symbol.range.end.line
		);
		
		outputChannel.appendLine(`[RAZ Debug] Debug codelens found: ${debugCodeLens ? 'yes' : 'no'}`);
		if (debugCodeLens) {
			outputChannel.appendLine(`[RAZ Debug] Debug command: ${debugCodeLens.command?.command}`);
		}

		if (debugCodeLens?.command) {
			// Execute the Debug codelens command directly
			outputChannel.appendLine(`[RAZ Debug] Executing debug command for ${symbol.name}`);
			await vscode.commands.executeCommand(
				debugCodeLens.command.command,
				...(debugCodeLens.command.arguments || [])
			);
			
			vscode.window.showInformationMessage(
				`🐛 Debug mode: Found ${breakpoints.length} breakpoint(s) in ${symbol.name}`
			);
		} else {
			// No Debug codelens found, use RAZ
			outputChannel.appendLine(`[RAZ Debug] No debug codelens found, using RAZ`);
			await fallbackToRaz();
		}
	} catch (error) {
		outputChannel.appendLine(`[RAZ Debug] Error: ${error}`);
		await fallbackToRaz();
	}
}

/**
 * Register minimal debug commands
 */
export function registerDebugCommands(context: vscode.ExtensionContext): void {
	context.subscriptions.push(
		vscode.commands.registerCommand("raz.toggleBreakpointDetection", async () => {
			const config = vscode.workspace.getConfiguration("raz");
			const current = config.get<boolean>("enableBreakpointDetection", true);
			await config.update("enableBreakpointDetection", !current, vscode.ConfigurationTarget.Global);
			
			vscode.window.showInformationMessage(
				`RAZ breakpoint detection ${!current ? "enabled" : "disabled"}`
			);
		})
	);

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
				const symbol = await findSymbol(document, position);
				if (!symbol) {
					vscode.window.showInformationMessage("No symbol found at cursor position");
					return;
				}

				const breakpoints = getBreakpoints(symbol, document);
				const codelenses = await getCodeLens(document);
				const debugCodeLens = codelenses.find(lens => lens.command?.title === "Debug");

				const info = [
					`Symbol: ${symbol.name} (${vscode.SymbolKind[symbol.kind]})`,
					`Breakpoints: ${breakpoints.length}`,
					`Debug codelens available: ${debugCodeLens ? "Yes" : "No"}`,
					`Would use: ${breakpoints.length > 0 && debugCodeLens ? "Debug mode" : "RAZ execution"}`,
				].join("\n");

				vscode.window.showInformationMessage(info, { modal: true });
			} catch (error) {
				vscode.window.showErrorMessage(`Debug info error: ${error}`);
			}
		})
	);
}