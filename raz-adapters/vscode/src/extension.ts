/*---------------------------------------------------------------------------------------------
 *  RAZ VS Code Extension - Universal command generator for Rust projects
 *--------------------------------------------------------------------------------------------*/
import * as vscode from "vscode";
import * as path from "node:path";
import * as fs from "node:fs";
import * as https from "node:https";
import * as zlib from "node:zlib";
import * as tar from "tar";
import { registerTaskProvider, executeRazAsTask } from "./taskProvider";
import { executeWithDebuggingSupport, registerDebugCommands } from "./debugger";

let outputChannel: vscode.OutputChannel;
let terminal: vscode.Terminal | undefined;

// Helper function to ensure outputChannel exists
function ensureOutputChannel(): vscode.OutputChannel {
	if (!outputChannel) {
		outputChannel = vscode.window.createOutputChannel("RAZ");
	}
	return outputChannel;
}

class RazBinaryManager {
	private context: vscode.ExtensionContext;
	private outputChannel: vscode.OutputChannel;
	private readonly GITHUB_REPO = "codeitlikemiley/raz"; // Update with your repo
	private readonly BINARY_DIR = "bin";

	constructor(context: vscode.ExtensionContext, outputChannel: vscode.OutputChannel) {
		this.context = context;
		this.outputChannel = outputChannel;
	}

	private getPlatformInfo(): { platform: string; arch: string; exe: string } {
		const platform = process.platform;
		const arch = process.arch;

		let platformName: string;
		let archName: string;
		let exe = "";

		switch (platform) {
			case "win32":
				platformName = "win32";
				exe = ".exe";
				break;
			case "darwin":
				platformName = "darwin";
				break;
			case "linux":
				platformName = "linux";
				break;
			default:
				throw new Error(`Unsupported platform: ${platform}`);
		}

		switch (arch) {
			case "x64":
				archName = "x64";
				break;
			case "arm64":
				archName = "arm64";
				break;
			default:
				throw new Error(`Unsupported architecture: ${arch}`);
		}

		return { platform: `${platformName}-${archName}`, arch: archName, exe };
	}

	private getBinaryPath(): string {
		const { exe } = this.getPlatformInfo();
		const binaryDir = path.join(
			this.context.globalStorageUri.fsPath,
			this.BINARY_DIR,
		);
		return path.join(binaryDir, `raz${exe}`);
	}

	private async downloadBinary(version: string): Promise<void> {
		const { platform } = this.getPlatformInfo();
		const binaryPath = this.getBinaryPath();
		const binaryDir = path.dirname(binaryPath);

		// Create directory if it doesn't exist
		await fs.promises.mkdir(binaryDir, { recursive: true });

		// Download URL for the binary
		const downloadUrl = `https://github.com/${this.GITHUB_REPO}/releases/download/${version}/raz-${platform}.tar.gz`;

		this.outputChannel.appendLine(`Downloading RAZ binary from: ${downloadUrl}`);

		return new Promise((resolve, reject) => {
			const downloadAndExtract = (url: string) => {
				https
					.get(url, (response) => {
						if (response.statusCode === 302 || response.statusCode === 301) {
							// Handle redirect
							if (response.headers.location) {
								downloadAndExtract(response.headers.location);
							} else {
								reject(new Error("Redirect without location header"));
							}
						} else if (response.statusCode === 200) {
							// Extract tar.gz directly to the binary directory
							response
								.pipe(zlib.createGunzip())
								.pipe(
									tar.extract({
										cwd: binaryDir,
										strip: 0,
									}),
								)
								.on("finish", () => {
									// Make the binary executable
									fs.chmodSync(binaryPath, 0o755);
									this.outputChannel.appendLine(
										"RAZ binary downloaded and extracted successfully",
									);
									resolve();
								})
								.on("error", reject);
						} else {
							reject(new Error(`Failed to download: ${response.statusCode}`));
						}
					})
					.on("error", reject);
			};

			downloadAndExtract(downloadUrl);
		});
	}

	private async getLatestVersion(): Promise<string> {
		return new Promise((resolve, reject) => {
			https
				.get(
					`https://api.github.com/repos/${this.GITHUB_REPO}/releases/latest`,
					{
						headers: { "User-Agent": "raz-vscode-extension" },
					},
					(response) => {
						let data = "";
						response.on("data", (chunk) => {
							data += chunk;
						});
						response.on("end", () => {
							try {
								const release = JSON.parse(data);
								resolve(release.tag_name);
							} catch (err) {
								reject(err);
							}
						});
					},
				)
				.on("error", reject);
		});
	}

	async ensureBinary(): Promise<string> {
		// Check for user-configured path first
		const config = vscode.workspace.getConfiguration("raz");
		const customPath = config.get<string>("path");

		if (customPath) {
			this.outputChannel.appendLine(`Using custom RAZ path: ${customPath}`);
			if (fs.existsSync(customPath)) {
				return customPath;
			}
			this.outputChannel.appendLine(`Custom RAZ path not found: ${customPath}`);
			throw new Error(`Custom RAZ binary not found at: ${customPath}`);
		}

		// Check if binary already exists in our managed location
		const binaryPath = this.getBinaryPath();
		if (fs.existsSync(binaryPath)) {
			this.outputChannel.appendLine(`Using managed RAZ binary: ${binaryPath}`);
			return binaryPath;
		}

		// Show progress while downloading
		return vscode.window.withProgress(
			{
				location: vscode.ProgressLocation.Notification,
				title: "RAZ",
				cancellable: false,
			},
			async (progress) => {
				progress.report({ message: "Downloading RAZ binary..." });

				try {
					const latestVersion = await this.getLatestVersion();
					await this.downloadBinary(latestVersion);
					return binaryPath;
				} catch (error) {
					throw new Error(`Failed to download RAZ binary: ${error}`);
				}
			},
		);
	}
}

export async function activate(
	context: vscode.ExtensionContext,
): Promise<void> {
	// Create output channel for RAZ - MUST be first to ensure it's available
	if (!outputChannel) {
		outputChannel = vscode.window.createOutputChannel("RAZ");
	}
	context.subscriptions.push(outputChannel);

	outputChannel.appendLine("🚀 RAZ extension activated successfully!");
	outputChannel.show(); // Show output to help with debugging

	// Register the task provider
	context.subscriptions.push(registerTaskProvider(context));
	outputChannel.appendLine("RAZ task provider registered");

	// Register the main command (Cmd+R)
	context.subscriptions.push(
		vscode.commands.registerCommand("raz.runCommand", async () => {
			outputChannel.appendLine("📝 raz.runCommand executed");
			await runRazCommand(context);
		}),
	);
	outputChannel.appendLine("✅ raz.runCommand registered");

	// Register the command picker
	context.subscriptions.push(
		vscode.commands.registerCommand("raz.selectAndRunCommand", async () => {
			outputChannel.appendLine("📝 raz.selectAndRunCommand executed");
			await selectAndRunRazCommand(context);
		}),
	);
	outputChannel.appendLine("✅ raz.selectAndRunCommand registered");

	// Register the override command (Cmd+Shift+R)
	context.subscriptions.push(
		vscode.commands.registerCommand("raz.runCommandWithOverride", async () => {
			outputChannel.appendLine("📝 raz.runCommandWithOverride executed");
			await runRazCommandWithOverride(context);
		}),
	);
	outputChannel.appendLine("✅ raz.runCommandWithOverride registered");

	// Register clear overrides command
	context.subscriptions.push(
		vscode.commands.registerCommand("raz.clearOverrides", async () => {
			outputChannel.appendLine("📝 raz.clearOverrides executed");
			try {
				await clearOverrides(context);
			} catch (error) {
				outputChannel.appendLine(`❌ Error in raz.clearOverrides: ${error}`);
				vscode.window.showErrorMessage(`RAZ: Command failed: ${error}`);
			}
		}),
	);

	// Register override debugging commands
	context.subscriptions.push(
		vscode.commands.registerCommand("raz.listOverrides", async () => {
			outputChannel.appendLine("📝 raz.listOverrides executed");
			try {
				await listOverrides(context);
			} catch (error) {
				outputChannel.appendLine(`❌ Error in raz.listOverrides: ${error}`);
				vscode.window.showErrorMessage(`RAZ: Command failed: ${error}`);
			}
		}),
	);

	context.subscriptions.push(
		vscode.commands.registerCommand("raz.debugOverride", async () => {
			outputChannel.appendLine("📝 raz.debugOverride executed");
			try {
				await debugOverride(context);
			} catch (error) {
				outputChannel.appendLine(`❌ Error in raz.debugOverride: ${error}`);
				vscode.window.showErrorMessage(`RAZ: Command failed: ${error}`);
			}
		}),
	);

	context.subscriptions.push(
		vscode.commands.registerCommand("raz.overrideStats", async () => {
			outputChannel.appendLine("📝 raz.overrideStats executed");
			try {
				await showOverrideStats(context);
			} catch (error) {
				outputChannel.appendLine(`❌ Error in raz.overrideStats: ${error}`);
				vscode.window.showErrorMessage(`RAZ: Command failed: ${error}`);
			}
		}),
	);

	// Register settings commands
	context.subscriptions.push(
		vscode.commands.registerCommand("raz.openSettings", async () => {
			outputChannel.appendLine("📝 raz.openSettings executed");
			await vscode.commands.executeCommand('workbench.action.openSettings', 'raz');
		}),
	);

	context.subscriptions.push(
		vscode.commands.registerCommand("raz.setupBinary", async () => {
			outputChannel.appendLine("📝 raz.setupBinary executed");
			await setupRazBinary(context);
		}),
	);

	// Register debugging-related commands
	registerDebugCommands(context);
	outputChannel.appendLine("✅ Debug commands registered");
}

async function setupRazBinary(_context: vscode.ExtensionContext): Promise<void> {
	const config = vscode.workspace.getConfiguration("raz");
	const currentPath = config.get<string>("path", "");

	const setupChoice = await vscode.window.showQuickPick([
		{
			label: "$(cloud-download) Auto-download binary",
			description: "Let VS Code automatically download and manage RAZ binary",
			detail: "Recommended: Works out of the box, automatically updates",
			value: "auto"
		},
		{
			label: "$(terminal) Use system RAZ from PATH",
			description: "Use 'raz' command from system PATH",
			detail: "Requires: cargo install raz-cli",
			value: "system"
		},
		{
			label: "$(folder-opened) Set custom binary path",
			description: "Specify exact path to RAZ binary",
			detail: "For advanced users with custom installations",
			value: "custom"
		},
		{
			label: "$(gear) Open RAZ settings",
			description: "Open VS Code settings for RAZ",
			detail: "Manual configuration",
			value: "settings"
		}
	], {
		placeHolder: "How do you want to setup RAZ?",
		title: "RAZ Binary Setup"
	});

	if (!setupChoice) {
		return; // User cancelled
	}

	switch (setupChoice.value) {
		case "auto": {
			// Clear the path to enable auto-download
			await config.update("path", "", vscode.ConfigurationTarget.Global);
			vscode.window.showInformationMessage(
				"✅ RAZ will now auto-download binaries. Try running a command to test!"
			);
			break;
		}

		case "system": {
			// Show installation instructions and set to 'raz'
			const installChoice = await vscode.window.showInformationMessage(
				"To use system RAZ, first install it via Cargo:",
				{ modal: true, detail: "cargo install raz-cli" },
				"Copy Install Command",
				"Open Installation Guide",
				"I Already Have RAZ"
			);

			if (installChoice === "Copy Install Command") {
				await vscode.env.clipboard.writeText("cargo install raz-cli");
				vscode.window.showInformationMessage("Install command copied to clipboard!");
			} else if (installChoice === "Open Installation Guide") {
				await vscode.env.openExternal(vscode.Uri.parse("https://crates.io/crates/raz-cli"));
			}

			if (installChoice === "I Already Have RAZ" || installChoice) {
				await config.update("path", "raz", vscode.ConfigurationTarget.Global);
				vscode.window.showInformationMessage(
					"✅ RAZ will use the system binary from PATH. Test it with Cmd+R on a Rust file!"
				);
			}
			break;
		}

		case "custom": {
			const customPath = await vscode.window.showInputBox({
				prompt: "Enter the full path to RAZ binary",
				placeHolder: "/usr/local/bin/raz or C:\\tools\\raz.exe",
				value: currentPath,
				validateInput: (value) => {
					if (!value.trim()) {
						return "Path cannot be empty";
					}
					return null;
				}
			});

			if (customPath) {
				await config.update("path", customPath, vscode.ConfigurationTarget.Global);
				vscode.window.showInformationMessage(
					`✅ RAZ binary path set to: ${customPath}`
				);
			}
			break;
		}

		case "settings": {
			await vscode.commands.executeCommand('workbench.action.openSettings', 'raz.path');
			break;
		}
	}
}

async function ensureRazExecutable(
	context: vscode.ExtensionContext,
): Promise<string | null> {
	const binaryManager = new RazBinaryManager(context, ensureOutputChannel());

	try {
		// Try to get or download the binary
		return await binaryManager.ensureBinary();
	} catch (error) {
		ensureOutputChannel().appendLine(`Binary download failed: ${error}`);

		// Fallback options
		const choice = await vscode.window.showErrorMessage(
			"Failed to download RAZ binary. Choose an alternative:",
			"Use System RAZ",
			"Install via Cargo",
			"Retry Download",
		);

		switch (choice) {
			case "Use System RAZ":
				return "raz";
			case "Install via Cargo":
				await vscode.env.openExternal(
					vscode.Uri.parse("https://crates.io/crates/raz-cli"),
				);
				return null;
			case "Retry Download":
				return await ensureRazExecutable(context);
			default:
				return null;
		}
	}
}

async function executeRazCommand(
	context: vscode.ExtensionContext,
	filePath: string,
	cursorLine?: number,
	cursorColumn?: number,
	additionalArgs?: string,
	fallbackCommand?: string,
): Promise<void> {
	const razPath = fallbackCommand || (await ensureRazExecutable(context));

	if (!razPath) {
		return;
	}

	const config = vscode.workspace.getConfiguration("raz");
	const useTaskRunner = config.get<boolean>("useTaskRunner", true);

	// Build the file path with cursor position
	let fileArg = filePath;
	if (cursorLine !== undefined && cursorColumn !== undefined) {
		// VSCode uses 0-based line/column, RAZ expects 1-based
		// The new override system will detect function names automatically
		fileArg = `${filePath}:${cursorLine + 1}:${cursorColumn + 1}`;
	}

	// Build args array
	const args = [`"${fileArg}"`];
	if (additionalArgs) {
		// Split additional args while preserving quoted strings
		const additionalArgsParsed =
			additionalArgs.match(/(?:[^\s"]+|"[^"]*")+/g) || [];
		args.push(...additionalArgsParsed);
	}

	ensureOutputChannel().appendLine(`Executing: ${razPath} ${args.join(" ")}`);

	// Use task runner for better concurrency
	if (useTaskRunner) {
		try {
			// For now, we'll execute the command directly as a task
			// In the future, we could enhance this to parse RAZ's output
			await executeRazAsTask(context, razPath, args, {
				cwd: path.dirname(filePath),
				label: `RAZ: ${path.basename(filePath)}`,
			});
			
			// Note: Execution tracking and rollback logic is handled by the CLI automatically
			// with the deferred save mechanism. No manual tracking needed in VS Code.
		} catch (error) {
			ensureOutputChannel().appendLine(
				`Task execution failed, falling back to terminal: ${error}`,
			);
			// Fallback to terminal execution
			await executeInTerminal(razPath, args, filePath);
		}
	} else {
		// Use traditional terminal execution
		await executeInTerminal(razPath, args, filePath);
	}
}

async function executeInTerminal(
	razPath: string,
	args: string[],
	filePath: string,
): Promise<void> {
	// Create or reuse terminal
	if (!terminal || terminal.exitStatus !== undefined) {
		terminal = vscode.window.createTerminal({
			name: "RAZ",
			cwd: path.dirname(filePath),
		});
	}

	// Execute RAZ command
	const command = `${razPath} ${args.join(" ")}`;
	terminal.sendText(command, true);

	const config = vscode.workspace.getConfiguration("raz");
	const showOutput = config.get<boolean>("showOutput", true);

	if (showOutput) {
		terminal.show();
	}
}

async function executeRazCommandWithSaveFlag(
	context: vscode.ExtensionContext,
	filePath: string,
	cursorLine?: number,
	cursorColumn?: number,
	additionalArgs?: string,
	saveOverride?: boolean,
): Promise<void> {
	const razPath = await ensureRazExecutable(context);
	if (!razPath) {
		return;
	}
	const config = vscode.workspace.getConfiguration("raz");
	const useTaskRunner = config.get<boolean>("useTaskRunner", true);

	// Build the file path with cursor position
	let fileArg = filePath;
	if (cursorLine !== undefined && cursorColumn !== undefined) {
		// VSCode uses 0-based line/column, RAZ expects 1-based
		// The new override system will detect function names automatically
		fileArg = `${filePath}:${cursorLine + 1}:${cursorColumn + 1}`;
	}

	// Build args array
	const args = [];

	// Add save override flag if requested
	if (saveOverride) {
		args.push("--save-override");
	}

	// Add the file argument
	args.push(`"${fileArg}"`);

	// Add additional override args if provided
	if (additionalArgs) {
		// Split additional args while preserving quoted strings
		const additionalArgsParsed =
			additionalArgs.match(/(?:[^\s"]+|"[^"]*")+/g) || [];
		args.push(...additionalArgsParsed);
	}

	ensureOutputChannel().appendLine(`Executing: ${razPath} ${args.join(" ")}`);

	// Use task runner for better concurrency
	if (useTaskRunner) {
		try {
			await executeRazAsTask(context, razPath, args, {
				cwd: path.dirname(filePath),
				label: `RAZ: ${path.basename(filePath)}`,
			});
		} catch (error) {
			ensureOutputChannel().appendLine(
				`Task execution failed, falling back to terminal: ${error}`,
			);
			// Fallback to terminal execution
			await executeInTerminal(razPath, args, filePath);
		}
	} else {
		// Use traditional terminal execution
		await executeInTerminal(razPath, args, filePath);
	}
}

async function runRazCommand(context: vscode.ExtensionContext): Promise<void> {
	const editor = vscode.window.activeTextEditor;
	if (!editor || editor.document.languageId !== "rust") {
		vscode.window.showErrorMessage("RAZ: No Rust file is currently open");
		return;
	}

	const document = editor.document;
	const selection = editor.selection;

	// Get cursor position
	const cursorLine = selection.active.line;
	const cursorColumn = selection.active.character;
	const filePath = document.uri.fsPath;

	try {
		// Check for breakpoints and use debugging integration when available
		await executeWithDebuggingSupport(
			document, 
			selection.active,
			async () => {
				// Fallback to normal RAZ execution
				await executeRazCommand(context, filePath, cursorLine, cursorColumn);
			}
		);
	} catch (error) {
		ensureOutputChannel().appendLine(`Error executing command: ${error}`);
		vscode.window.showErrorMessage(`RAZ: Failed to execute command: ${error}`);
	}
}

async function selectAndRunRazCommand(
	context: vscode.ExtensionContext,
): Promise<void> {
	const editor = vscode.window.activeTextEditor;
	if (!editor || editor.document.languageId !== "rust") {
		vscode.window.showErrorMessage("RAZ: No Rust file is currently open");
		return;
	}

	const document = editor.document;
	const selection = editor.selection;

	const cursorLine = selection.active.line;
	const cursorColumn = selection.active.character;
	const filePath = document.uri.fsPath;

	// Show information about what RAZ will do
	const fileArg =
		cursorLine !== undefined && cursorColumn !== undefined
			? `${filePath}:${cursorLine + 1}:${cursorColumn + 1}`
			: filePath;

	const action = await vscode.window.showInformationMessage(
		`RAZ will analyze: ${path.basename(fileArg)}`,
		"Run Command",
		"Show in Terminal",
	);

	if (action === "Run Command") {
		try {
			await executeRazCommand(context, filePath, cursorLine, cursorColumn);
		} catch (error) {
			ensureOutputChannel().appendLine(`Error executing command: ${error}`);
			vscode.window.showErrorMessage(
				`RAZ: Failed to execute command: ${error}`,
			);
		}
	} else if (action === "Show in Terminal") {
		// Just show what command would be run
		const razPath = await ensureRazExecutable(context);
		if (!razPath) {
			return;
		}
		if (!terminal || terminal.exitStatus !== undefined) {
			terminal = vscode.window.createTerminal({
				name: "RAZ",
				cwd: path.dirname(filePath),
			});
		}
		terminal.sendText(`echo "Would run: ${razPath} '${fileArg}'"`, true);
		terminal.show();
	}
}

async function runRazCommandWithOverride(
	context: vscode.ExtensionContext,
): Promise<void> {
	const editor = vscode.window.activeTextEditor;
	if (!editor || editor.document.languageId !== "rust") {
		vscode.window.showErrorMessage("RAZ: No Rust file is currently open");
		return;
	}

	const document = editor.document;
	const selection = editor.selection;

	// Get cursor position
	const cursorLine = selection.active.line;
	const cursorColumn = selection.active.character;
	const filePath = document.uri.fsPath;

	// Prompt for override arguments
	const overrideArgs = await vscode.window.showInputBox({
		prompt: "Enter overrides: env vars, cargo options, and test args",
		placeHolder: "Examples: --release | RUST_LOG=debug | -- --nocapture",
		title: "RAZ Command Override",
		value: "",
		validateInput: (value) => {
			// Basic validation - warn about common mistakes
			if (
				value.includes("--device") &&
				!value.match(/--device\s+(true|false)/)
			) {
				return "Note: --device expects 'true' or 'false' for Dioxus projects";
			}
			// Warn about common cargo option mistakes
			if (value.match(/-- --quiet|-- --release/)) {
				return "Warning: --quiet and --release are cargo options, not test args. Use them before --";
			}
			return null;
		},
	});

	if (overrideArgs === undefined) {
		// User cancelled
		return;
	}

	try {
		// Build args for RAZ command
		const additionalArgs = overrideArgs.trim();

		// For now, we have to save immediately because the CLI doesn't support
		// saving without executing. This is a known limitation.
		const saveOverride = additionalArgs.length > 0;

		// Execute RAZ command with override and save flag
		await executeRazCommandWithSaveFlag(
			context,
			filePath,
			cursorLine,
			cursorColumn,
			additionalArgs,
			saveOverride,
		);

		if (saveOverride) {
			vscode.window.showInformationMessage(
				`Override saved for ${path.basename(filePath)}. Note: Failed overrides will need manual cleanup.`,
			);
		}
	} catch (error) {
		ensureOutputChannel().appendLine(`Error executing command with override: ${error}`);
		vscode.window.showErrorMessage(`RAZ: Failed to execute command: ${error}`);
	}
}

async function clearOverrides(context: vscode.ExtensionContext): Promise<void> {
	const workspaceFolders = vscode.workspace.workspaceFolders;
	if (!workspaceFolders || workspaceFolders.length === 0) {
		vscode.window.showErrorMessage("RAZ: No workspace folder found");
		return;
	}

	const result = await vscode.window.showWarningMessage(
		"Are you sure you want to clear all RAZ overrides?",
		{ modal: true },
		"Clear All",
		"Cancel",
	);

	if (result === "Clear All") {
		try {
			const razPath = await ensureRazExecutable(context);
			if (!razPath) {
				return;
			}
			const args = ["override", "clear", "--force"];

			// Execute the clear command
			await executeRazAsTask(context, razPath, args, {
				cwd: workspaceFolders[0].uri.fsPath,
				label: "RAZ: Clear Overrides",
			});

			vscode.window.showInformationMessage(
				"All RAZ overrides cleared successfully",
			);
		} catch (error) {
			vscode.window.showErrorMessage(`Failed to clear overrides: ${error}`);
		}
	}
}

async function listOverrides(context: vscode.ExtensionContext): Promise<void> {
	const workspaceFolders = vscode.workspace.workspaceFolders;
	if (!workspaceFolders || workspaceFolders.length === 0) {
		vscode.window.showErrorMessage("RAZ: No workspace folder found");
		return;
	}

	try {
		const razPath = await ensureRazExecutable(context);
		if (!razPath) {
			return;
		}
		const args = ["override", "list"];

		await executeRazAsTask(context, razPath, args, {
			cwd: workspaceFolders[0].uri.fsPath,
			label: "RAZ: List Overrides",
		});
	} catch (error) {
		vscode.window.showErrorMessage(`Failed to list overrides: ${error}`);
	}
}

async function debugOverride(context: vscode.ExtensionContext): Promise<void> {
	const editor = vscode.window.activeTextEditor;
	if (!editor || editor.document.languageId !== "rust") {
		vscode.window.showErrorMessage("RAZ: No Rust file is currently open");
		return;
	}

	const workspaceFolders = vscode.workspace.workspaceFolders;
	if (!workspaceFolders || workspaceFolders.length === 0) {
		vscode.window.showErrorMessage("RAZ: No workspace folder found");
		return;
	}

	const document = editor.document;
	const selection = editor.selection;
	const cursorLine = selection.active.line;
	const cursorColumn = selection.active.character;
	const filePath = document.uri.fsPath;

	try {
		const razPath = await ensureRazExecutable(context);
		if (!razPath) {
			return;
		}
		const args = [
			"override",
			"debug",
			filePath,
			(cursorLine + 1).toString(),
			(cursorColumn + 1).toString(),
		];

		await executeRazAsTask(context, razPath, args, {
			cwd: workspaceFolders[0].uri.fsPath,
			label: `RAZ: Debug Override - ${path.basename(filePath)}`,
		});
	} catch (error) {
		vscode.window.showErrorMessage(`Failed to debug override: ${error}`);
	}
}

async function showOverrideStats(
	context: vscode.ExtensionContext,
): Promise<void> {
	const workspaceFolders = vscode.workspace.workspaceFolders;
	if (!workspaceFolders || workspaceFolders.length === 0) {
		vscode.window.showErrorMessage("RAZ: No workspace folder found");
		return;
	}

	try {
		const razPath = await ensureRazExecutable(context);
		if (!razPath) {
			return;
		}
		const args = ["override", "stats"];

		await executeRazAsTask(context, razPath, args, {
			cwd: workspaceFolders[0].uri.fsPath,
			label: "RAZ: Override Statistics",
		});
	} catch (error) {
		vscode.window.showErrorMessage(`Failed to show override stats: ${error}`);
	}
}




export function deactivate(): void {
	if (terminal) {
		terminal.dispose();
	}
}
