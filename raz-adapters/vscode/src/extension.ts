/*---------------------------------------------------------------------------------------------
 *  RAZ VS Code Extension - Universal command generator for Rust projects
 *--------------------------------------------------------------------------------------------*/
import * as vscode from "vscode";
import * as path from "node:path";
import * as fs from "node:fs";
import * as https from "node:https";
import * as zlib from "node:zlib";
import * as tar from "tar";
import { exec } from "node:child_process";
import { registerTaskProvider, executeRazAsTask } from "./taskProvider";
import { executeWithDebuggingSupport, registerDebugCommands } from "./debugger";
import { findNearestRazDirectory, getAllRazDirectories } from "./utils/workspace";
import { RazOverrideTreeProvider } from "./overrideTreeProvider";

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

	private getVersionFilePath(): string {
		return path.join(
			this.context.globalStorageUri.fsPath,
			this.BINARY_DIR,
			'version.txt'
		);
	}

	private async getCurrentBinaryVersion(): Promise<string | null> {
		const versionFile = this.getVersionFilePath();
		try {
			if (fs.existsSync(versionFile)) {
				return fs.readFileSync(versionFile, 'utf8').trim();
			}
		} catch (error) {
			this.outputChannel.appendLine(`Failed to read version file: ${error}`);
		}
		return null;
	}

	private async saveBinaryVersion(version: string): Promise<void> {
		const versionFile = this.getVersionFilePath();
		const dir = path.dirname(versionFile);
		await fs.promises.mkdir(dir, { recursive: true });
		await fs.promises.writeFile(versionFile, version, 'utf8');
	}

	private async getInstalledBinaryVersion(): Promise<string | null> {
		const binaryPath = this.getBinaryPath();
		if (!fs.existsSync(binaryPath)) {
			return null;
		}

		return new Promise((resolve) => {
			exec(`"${binaryPath}" --version`, (error: Error | null, stdout: string, _stderr: string) => {
				if (error) {
					this.outputChannel.appendLine(`Failed to get binary version: ${error}`);
					resolve(null);
					return;
				}
				// Parse version from output like "raz 0.2.0"
				const match = stdout.match(/raz\s+(\d+\.\d+\.\d+)/);
				if (match) {
					resolve(`v${match[1]}`); // Add 'v' prefix to match GitHub tags
				} else {
					this.outputChannel.appendLine(`Failed to parse version from: ${stdout}`);
					resolve(null);
				}
			});
		});
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
								.on("finish", async () => {
									// Make the binary executable
									fs.chmodSync(binaryPath, 0o755);
									// Save the version
									await this.saveBinaryVersion(version);
									this.outputChannel.appendLine(
										`RAZ binary ${version} downloaded and extracted successfully`,
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

	async getLatestVersion(): Promise<string> {
		return new Promise((resolve, reject) => {
			https
				.get(
					`https://api.github.com/repos/${this.GITHUB_REPO}/releases/latest`,
					{
						headers: { "User-Agent": "raz-vscode-extension" },
					},
					(response) => {
						if (response.statusCode !== 200) {
							reject(new Error(`GitHub API returned ${response.statusCode}`));
							return;
						}
						
						let data = "";
						response.on("data", (chunk) => {
							data += chunk;
						});
						response.on("end", () => {
							try {
								const release = JSON.parse(data);
								if (!release.tag_name) {
									reject(new Error("No tag_name in release"));
									return;
								}
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
		
		// If binary exists, use it
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
					this.outputChannel.appendLine(`Failed to get latest version: ${error}`);
					// Fallback to extension's version using proper VS Code API
					const extension = vscode.extensions.getExtension('masterustacean.raz-vscode');
					const fallbackVersion = `v${extension?.packageJSON.version || '0.2.1'}`;
					this.outputChannel.appendLine(`Using extension version as fallback: ${fallbackVersion}`);
					try {
						await this.downloadBinary(fallbackVersion);
						return binaryPath;
					} catch (downloadError) {
						throw new Error(`Failed to download RAZ binary: ${downloadError}`);
					}
				}
			},
		);
	}
}

/// Handle migration from old binary management
async function handleBinaryMigration(context: vscode.ExtensionContext): Promise<void> {
	const outputChannel = ensureOutputChannel();
	
	// Check if we need to migrate from old version
	const migrationKey = 'raz.migrated.v0.2.1';
	const hasMigrated = context.globalState.get<boolean>(migrationKey, false);
	
	if (hasMigrated) {
		// Already migrated, just check for updates
		await checkForUpdates(context);
		return;
	}
	
	// First time running v0.2.1+ - clean up old binaries and notify
	outputChannel.appendLine("Performing migration to new RAZ binary management...");
	
	// Clean up old version files and binaries
	const binaryDir = path.join(context.globalStorageUri.fsPath, 'bin');
	const versionFile = path.join(binaryDir, 'version.txt');
	
	if (fs.existsSync(versionFile)) {
		fs.unlinkSync(versionFile);
		outputChannel.appendLine("Removed old version tracking file");
	}
	
	// Remove old binaries (keep only latest)
	if (fs.existsSync(binaryDir)) {
		const files = fs.readdirSync(binaryDir);
		for (const file of files) {
			if (file.startsWith('raz') && file !== 'raz' && file !== 'raz.exe') {
				const filePath = path.join(binaryDir, file);
				fs.unlinkSync(filePath);
				outputChannel.appendLine(`Removed old binary: ${file}`);
			}
		}
	}
	
	// Mark as migrated
	context.globalState.update(migrationKey, true);
	
	// Show notification about new update method
	const choice = await vscode.window.showInformationMessage(
		"RAZ has been updated! We've simplified binary management. You can now use 'raz self-update' to update the CLI.",
		"Learn More",
		"Dismiss"
	);
	
	if (choice === "Learn More") {
		vscode.env.openExternal(vscode.Uri.parse("https://github.com/codeitlikemiley/raz#updating"));
	}
	
	// Check for updates after migration
	await checkForUpdates(context);
}

/// Check for updates and notify user
async function checkForUpdates(context: vscode.ExtensionContext): Promise<void> {
	const outputChannel = ensureOutputChannel();
	
	try {
		// Check if user has cargo-installed version
		const razVersion = await getRazVersion();
		if (!razVersion) {
			outputChannel.appendLine("No RAZ installation found in PATH");
			return;
		}
		
		// Get latest version from GitHub
		const binaryManager = new RazBinaryManager(context, outputChannel);
		const latestVersion = await binaryManager.getLatestVersion();
		
		// Compare versions
		const currentVersion = razVersion.replace('v', '');
		const latest = latestVersion.replace('v', '');
		
		if (currentVersion !== latest) {
			outputChannel.appendLine(`Update available: v${currentVersion} -> ${latestVersion}`);
			
			// Don't show update notification too frequently
			const lastNotification = context.globalState.get<number>('raz.lastUpdateNotification', 0);
			const now = Date.now();
			const daysSince = (now - lastNotification) / (1000 * 60 * 60 * 24);
			
			if (daysSince >= 7) { // Show update notification once per week
				// Check if the current version supports self-update (>=0.2.1)
				const supportsSelfUpdate = isVersionAtLeast(currentVersion, '0.2.1');
				
				let choice;
				if (supportsSelfUpdate) {
					choice = await vscode.window.showInformationMessage(
						`RAZ update available: v${currentVersion} → ${latestVersion}`,
						"Update with CLI",
						"Manual Update",
						"Later"
					);
				} else {
					// Older version without self-update command
					choice = await vscode.window.showInformationMessage(
						`RAZ update available: v${currentVersion} → ${latestVersion}\n(Your version doesn't support self-update)`,
						"Manual Update",
						"Later"
					);
				}
				
				if (choice === "Update with CLI") {
					// Open terminal and run raz self-update
					const terminal = vscode.window.createTerminal("RAZ Update");
					terminal.show();
					terminal.sendText("raz self-update");
				} else if (choice === "Manual Update") {
					// Show manual update instructions
					const updateChoice = await vscode.window.showInformationMessage(
						"How would you like to update RAZ?",
						"cargo install raz-cli",
						"View GitHub Release"
					);
					
					if (updateChoice === "cargo install raz-cli") {
						await vscode.env.clipboard.writeText("cargo install raz-cli --force");
						vscode.window.showInformationMessage("Update command copied to clipboard!");
						const terminal = vscode.window.createTerminal("RAZ Update");
						terminal.show();
						terminal.sendText("# Paste the update command from clipboard");
					} else if (updateChoice === "View GitHub Release") {
						vscode.env.openExternal(vscode.Uri.parse(`https://github.com/codeitlikemiley/raz/releases/tag/${latestVersion}`));
					}
				}
				
				context.globalState.update('raz.lastUpdateNotification', now);
			}
		} else {
			outputChannel.appendLine(`RAZ is up to date: v${currentVersion}`);
		}
	} catch (error) {
		outputChannel.appendLine(`Update check failed: ${error}`);
	}
}

/// Get RAZ version from system PATH
async function getRazVersion(): Promise<string | null> {
	return new Promise((resolve) => {
		exec('raz --version', (error: Error | null, stdout: string) => {
			if (error) {
				resolve(null);
				return;
			}
			const match = stdout.match(/raz\s+(\d+\.\d+\.\d+)/);
			if (match) {
				resolve(`v${match[1]}`);
			} else {
				resolve(null);
			}
		});
	});
}

/// Compare version strings (semver-like)
function isVersionAtLeast(current: string, required: string): boolean {
	const parseVersion = (v: string) => v.split('.').map(n => parseInt(n, 10));
	const currentParts = parseVersion(current);
	const requiredParts = parseVersion(required);
	
	for (let i = 0; i < Math.max(currentParts.length, requiredParts.length); i++) {
		const currentPart = currentParts[i] || 0;
		const requiredPart = requiredParts[i] || 0;
		
		if (currentPart > requiredPart) {return true;}
		if (currentPart < requiredPart) {return false;}
	}
	
	return true; // Equal versions
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

	// Handle migration from old binary management and check for updates
	handleBinaryMigration(context).catch(error => {
		outputChannel.appendLine(`Binary migration failed: ${error}`);
	});

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

	context.subscriptions.push(
		vscode.commands.registerCommand("raz.updateBinary", async () => {
			outputChannel.appendLine("📝 raz.updateBinary executed");
			
			// Check current version and available update methods
			const razVersion = await getRazVersion();
			if (!razVersion) {
				vscode.window.showErrorMessage("RAZ not found in system PATH");
				return;
			}
			
			const currentVersion = razVersion.replace('v', '');
			const supportsSelfUpdate = isVersionAtLeast(currentVersion, '0.2.1');
			
			let choice;
			if (supportsSelfUpdate) {
				choice = await vscode.window.showQuickPick([
					{ label: "Update with CLI", description: "Use 'raz self-update' command" },
					{ label: "Manual Update", description: "Show manual update instructions" }
				], {
					placeHolder: "Choose update method"
				});
			} else {
				choice = await vscode.window.showQuickPick([
					{ label: "Manual Update", description: "Show manual update instructions" },
					{ label: "Manual Update", description: "Show manual update instructions" }
				], {
					placeHolder: "Your version doesn't support self-update"
				});
			}
			
			if (!choice) {return;}
			
			if (choice.label === "Update with CLI") {
				const terminal = vscode.window.createTerminal("RAZ Update");
				terminal.show();
				terminal.sendText("raz self-update");
			} else if (choice.label === "Manual Update") {
				const updateChoice = await vscode.window.showInformationMessage(
					"How would you like to update RAZ?",
					"cargo install raz-cli",
					"View GitHub Release"
				);
				
				if (updateChoice === "cargo install raz-cli") {
					await vscode.env.clipboard.writeText("cargo install raz-cli --force");
					vscode.window.showInformationMessage("Update command copied to clipboard!");
					const terminal = vscode.window.createTerminal("RAZ Update");
					terminal.show();
					terminal.sendText("# Paste the update command from clipboard");
				} else if (updateChoice === "View GitHub Release") {
					try {
						const binaryManager = new RazBinaryManager(context, outputChannel);
						const latestVersion = await binaryManager.getLatestVersion();
						vscode.env.openExternal(vscode.Uri.parse(`https://github.com/codeitlikemiley/raz/releases/tag/${latestVersion}`));
					} catch {
						vscode.env.openExternal(vscode.Uri.parse("https://github.com/codeitlikemiley/raz/releases/latest"));
					}
				}
			}
		}),
	);

	// Register reset binary command
	context.subscriptions.push(
		vscode.commands.registerCommand("raz.resetBinary", async () => {
			outputChannel.appendLine("📝 raz.resetBinary executed");
			
			const result = await vscode.window.showWarningMessage(
				"This will remove any old RAZ binaries and reset the extension. Continue?",
				{ modal: true, detail: "This is useful if you're having issues with old binary management." },
				"Reset",
				"Cancel"
			);
			
			if (result === "Reset") {
				try {
					// Clear any old binary paths
					const config = vscode.workspace.getConfiguration("raz");
					await config.update("path", "", vscode.ConfigurationTarget.Global);
					
					// Clear old binary manager storage if it exists
					// Note: cleanupOldBinaries method may not exist in newer versions
					try {
						const binaryManager = new RazBinaryManager(context, outputChannel);
						if (typeof (binaryManager as unknown as {cleanupOldBinaries?: () => Promise<void>}).cleanupOldBinaries === 'function') {
							await (binaryManager as unknown as {cleanupOldBinaries: () => Promise<void>}).cleanupOldBinaries();
						}
					} catch {
						// Ignore if method doesn't exist
					}
					
					// Clear migration flags to force re-check
					await context.globalState.update('raz.migrated.v0.2.1', undefined);
					await context.globalState.update('raz.lastUpdateNotification', undefined);
					
					// Show instructions for clean install
					const terminal = vscode.window.createTerminal("RAZ Reset");
					terminal.show();
					terminal.sendText("# RAZ has been reset. Installing fresh version...");
					terminal.sendText("cargo install raz-cli --force");
					
					vscode.window.showInformationMessage(
						"RAZ reset complete! Installing fresh version via cargo...",
						"OK"
					);
				} catch (error) {
					vscode.window.showErrorMessage(`Failed to reset RAZ: ${error}`);
					outputChannel.appendLine(`Reset error: ${error}`);
				}
			}
		}),
	);

	// Register init command
	context.subscriptions.push(
		vscode.commands.registerCommand("raz.init", async () => {
			outputChannel.appendLine("📝 raz.init executed");
			await initRazConfig(context);
		}),
	);

	// Register list overrides for current file command
	context.subscriptions.push(
		vscode.commands.registerCommand("raz.listOverridesForFile", async () => {
			outputChannel.appendLine("📝 raz.listOverridesForFile executed");
			await listOverridesForFile(context);
		}),
	);

	// Register show stats for current file command
	context.subscriptions.push(
		vscode.commands.registerCommand("raz.showStatsForFile", async () => {
			outputChannel.appendLine("📝 raz.showStatsForFile executed");
			await showStatsForFile(context);
		}),
	);

	// Register clear overrides for current file command
	context.subscriptions.push(
		vscode.commands.registerCommand("raz.clearOverridesForFile", async () => {
			outputChannel.appendLine("📝 raz.clearOverridesForFile executed");
			await clearOverridesForFile(context);
		}),
	);

	// Register debugging-related commands
	registerDebugCommands(context);
	outputChannel.appendLine("✅ Debug commands registered");

	// Register the override tree view
	const overrideTreeProvider = new RazOverrideTreeProvider(context);
	context.subscriptions.push(
		vscode.window.registerTreeDataProvider('razOverrides', overrideTreeProvider)
	);
	
	// Register refresh command for the tree
	context.subscriptions.push(
		vscode.commands.registerCommand('raz.refreshOverrides', () => {
			overrideTreeProvider.refresh();
		})
	);
	
	// Register run specific override command
	context.subscriptions.push(
		vscode.commands.registerCommand('raz.runSpecificOverride', async (filePath: string, line: number) => {
			// Open the file first
			const doc = await vscode.workspace.openTextDocument(filePath);
			const editor = await vscode.window.showTextDocument(doc);
			
			// Move cursor to the line
			const position = new vscode.Position(line - 1, 0);
			editor.selection = new vscode.Selection(position, position);
			
			// Run the command
			await vscode.commands.executeCommand('raz.runCommand');
		})
	);
	
	// Register edit override command
	context.subscriptions.push(
		vscode.commands.registerCommand('raz.editOverride', async (filePath: string, line: number, field: string, currentValue?: string) => {
			// Open the file first
			const doc = await vscode.workspace.openTextDocument(filePath);
			const editor = await vscode.window.showTextDocument(doc);
			
			// Move cursor to the line
			const position = new vscode.Position(line - 1, 0);
			editor.selection = new vscode.Selection(position, position);
			
			// Prompt for new value
			const fieldLabel = field === 'cargo_options' ? 'Cargo Options' : 
			               field === 'args' ? 'Arguments' : 
			               field === 'env' ? 'Environment Variables' : field;
			const placeHolder = field === 'cargo_options' ? 'e.g., --release --features foo' : 
			                 field === 'args' ? 'e.g., --nocapture --exact' :
			                 field === 'env' ? 'e.g., RUST_BACKTRACE=1 RUST_LOG=debug' : '';
			
			const newValue = await vscode.window.showInputBox({
				prompt: `Edit ${fieldLabel}`,
				value: currentValue || '',
				placeHolder: placeHolder
			});
			
			if (newValue !== undefined) {
				// Prepare the override string based on the field
				let overrideString = '';
				if (field === 'env') {
					overrideString = newValue;
				} else if (field === 'cargo_options') {
					overrideString = newValue;
				} else if (field === 'args') {
					overrideString = `-- ${newValue}`;
				}
				
				// Set the override value in a way that runCommandWithOverride can use
				await vscode.commands.executeCommand('setContext', 'raz.pendingOverride', overrideString);
				// Run with the new override
				await vscode.commands.executeCommand('raz.runCommandWithOverride');
			}
		})
	);
	
	// Register edit all override options command
	context.subscriptions.push(
		vscode.commands.registerCommand('raz.editAllOverrideOptions', async (item: unknown, ...restArgs: unknown[]) => {
			// Handle both direct calls and calls from tree view
			let filePath: string;
			let line: number;
			let overrideData: unknown;
			
			if (typeof item === 'string') {
				// Called with separate arguments
				filePath = item;
				line = restArgs[0] as number;
				overrideData = restArgs[1];
			} else if (item && item.filePath) {
				// Called from tree view context menu
				filePath = item.filePath;
				line = item.line || 1;
				overrideData = item.overrideData;
			} else {
				vscode.window.showErrorMessage('Invalid arguments for edit override command');
				return;
			}
			
			// Open the file first
			const doc = await vscode.workspace.openTextDocument(filePath);
			const editor = await vscode.window.showTextDocument(doc);
			
			// Move cursor to the line
			const position = new vscode.Position(line - 1, 0);
			editor.selection = new vscode.Selection(position, position);
			
			// Build current override string
			let currentOverride = '';
			if (overrideData && overrideData.override_config) {
				if (overrideData.override_config.env && Object.keys(overrideData.override_config.env).length > 0) {
					const envVars = Object.entries(overrideData.override_config.env)
						.map(([key, value]) => `${key}=${value}`)
						.join(' ');
					currentOverride += envVars + ' ';
				}
				if (overrideData.override_config.cargo_options?.length) {
					currentOverride += overrideData.override_config.cargo_options.join(' ') + ' ';
				}
				if (overrideData.override_config.args?.length) {
					currentOverride += '-- ' + overrideData.override_config.args.join(' ');
				}
			}
			
			const newValue = await vscode.window.showInputBox({
				prompt: `Edit override options`,
				value: currentOverride.trim(),
				placeHolder: 'e.g., RUST_BACKTRACE=1 --release -- --nocapture'
			});
			
			if (newValue !== undefined) {
				// Show the override dialog with the new value pre-filled
				await runRazCommandWithOverride(context, newValue);
			}
		})
	);
	
	// Watch for changes to .raz/overrides.toml files
	const watcher = vscode.workspace.createFileSystemWatcher('**/.raz/overrides.toml');
	watcher.onDidChange(() => overrideTreeProvider.refresh());
	watcher.onDidCreate(() => overrideTreeProvider.refresh());
	watcher.onDidDelete(() => overrideTreeProvider.refresh());
	context.subscriptions.push(watcher);
	
	outputChannel.appendLine("✅ RAZ Override tree view registered");
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

// Old update functions removed - now using raz self-update

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
		ensureOutputChannel().appendLine("🔍 Checking for debugging integration...");
		// Check for breakpoints and use debugging integration when available
		try {
			await executeWithDebuggingSupport(
				document, 
				selection.active,
				async () => {
					ensureOutputChannel().appendLine("⚡ Using RAZ execution (no debug mode)");
					// Fallback to normal RAZ execution
					await executeRazCommand(context, filePath, cursorLine, cursorColumn);
				},
				ensureOutputChannel()
			);
		} catch (debugError) {
			ensureOutputChannel().appendLine(`🚨 DEBUG INTEGRATION ERROR: ${debugError}`);
			ensureOutputChannel().appendLine("⚡ Falling back to RAZ execution due to debug error");
			await executeRazCommand(context, filePath, cursorLine, cursorColumn);
		}
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
	preFilledValue?: string,
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
		value: preFilledValue || "",
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
	try {
		const razPath = await ensureRazExecutable(context);
		if (!razPath) {
			return;
		}
		
		// Find all directories with .raz configurations
		const razDirectories = getAllRazDirectories();
		
		if (razDirectories.length === 0) {
			vscode.window.showInformationMessage("No RAZ configurations found in any workspace folders");
			return;
		}
		
		let selectedDir: string;
		
		if (razDirectories.length === 1) {
			// Single directory
			selectedDir = razDirectories[0];
		} else {
			// Multiple directories, let user choose
			const choice = await vscode.window.showQuickPick(
				razDirectories.map(dir => ({
					label: `$(folder) ${path.basename(dir) || path.basename(path.dirname(dir))}`,
					description: dir,
					value: dir
				})), {
					placeHolder: "Select which project's overrides to clear",
					title: "RAZ: Select Project to Clear Overrides"
				}
			);
			
			if (!choice) {
				return;
			}
			
			selectedDir = choice.value;
		}
		
		const result = await vscode.window.showWarningMessage(
			"Are you sure you want to clear all RAZ overrides?",
			{ modal: true, detail: `This will clear all overrides in: ${selectedDir}` },
			"Clear All",
			"Cancel",
		);

		if (result === "Clear All") {
			const args = ["override", "clear", "--force"];

			// Execute the clear command
			await executeRazAsTask(context, razPath, args, {
				cwd: selectedDir,
				label: `RAZ: Clear Overrides (${path.basename(selectedDir)})`,
			});

			vscode.window.showInformationMessage(
				"All RAZ overrides cleared successfully",
			);
		}
	} catch (error) {
		vscode.window.showErrorMessage(`Failed to clear overrides: ${error}`);
	}
}

async function listOverrides(context: vscode.ExtensionContext): Promise<void> {
	try {
		const razPath = await ensureRazExecutable(context);
		if (!razPath) {
			return;
		}
		
		// Find all directories with .raz configurations
		const razDirectories = getAllRazDirectories();
		
		if (razDirectories.length === 0) {
			vscode.window.showInformationMessage("No RAZ configurations found in any workspace folders");
			return;
		}
		
		if (razDirectories.length === 1) {
			// Single directory, just list it
			const args = ["override", "list"];
			await executeRazAsTask(context, razPath, args, {
				cwd: razDirectories[0],
				label: `RAZ: List Overrides (${path.basename(razDirectories[0])})`,
			});
		} else {
			// Multiple directories, let user choose
			const choice = await vscode.window.showQuickPick([
				{
					label: "$(list-flat) Show All",
					description: "List overrides from all locations",
					value: "all"
				},
				...razDirectories.map(dir => ({
					label: `$(folder) ${path.basename(dir) || path.basename(path.dirname(dir))}`,
					description: dir,
					value: dir
				}))
			], {
				placeHolder: "Select which project's overrides to show",
				title: "RAZ: Select Override Location"
			});
			
			if (!choice) {
				return;
			}
			
			if (choice.value === "all") {
				// Show overrides from all directories
				for (const dir of razDirectories) {
					const args = ["override", "list"];
					await executeRazAsTask(context, razPath, args, {
						cwd: dir,
						label: `RAZ: List Overrides (${path.basename(dir) || path.basename(path.dirname(dir))})`,
					});
				}
			} else {
				// Show overrides from selected directory
				const args = ["override", "list"];
				await executeRazAsTask(context, razPath, args, {
					cwd: choice.value,
					label: `RAZ: List Overrides (${path.basename(choice.value)})`,
				});
			}
		}
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
	try {
		const razPath = await ensureRazExecutable(context);
		if (!razPath) {
			return;
		}
		
		// Find all directories with .raz configurations
		const razDirectories = getAllRazDirectories();
		
		if (razDirectories.length === 0) {
			vscode.window.showInformationMessage("No RAZ configurations found in any workspace folders");
			return;
		}
		
		if (razDirectories.length === 1) {
			// Single directory, just show stats
			const args = ["override", "stats"];
			await executeRazAsTask(context, razPath, args, {
				cwd: razDirectories[0],
				label: `RAZ: Override Statistics (${path.basename(razDirectories[0])})`,
			});
		} else {
			// Multiple directories, let user choose
			const choice = await vscode.window.showQuickPick(
				razDirectories.map(dir => ({
					label: `$(folder) ${path.basename(dir) || path.basename(path.dirname(dir))}`,
					description: dir,
					value: dir
				})), {
					placeHolder: "Select which project's override statistics to show",
					title: "RAZ: Select Project"
				}
			);
			
			if (!choice) {
				return;
			}
			
			const args = ["override", "stats"];
			await executeRazAsTask(context, razPath, args, {
				cwd: choice.value,
				label: `RAZ: Override Statistics (${path.basename(choice.value)})`,
			});
		}
	} catch (error) {
		vscode.window.showErrorMessage(`Failed to show override stats: ${error}`);
	}
}

async function initRazConfig(context: vscode.ExtensionContext): Promise<void> {
	const outputChannel = ensureOutputChannel();
	
	// Check if we have a workspace
	const workspaceFolders = vscode.workspace.workspaceFolders;
	if (!workspaceFolders || workspaceFolders.length === 0) {
		vscode.window.showErrorMessage("RAZ: No workspace folder found");
		return;
	}

	// Ask which level to initialize
	const levelChoice = await vscode.window.showQuickPick([
		{
			label: "$(file-directory) Project Level",
			description: "Initialize in the current project/crate",
			value: "project",
		},
		{
			label: "$(folder) Workspace Level",
			description: "Initialize for the entire workspace",
			value: "workspace",
		},
		{
			label: "$(home) Global Level",
			description: "Initialize in ~/.raz for all projects",
			value: "global",
		},
	], {
		placeHolder: "Select where to initialize RAZ configuration",
		title: "RAZ Configuration Level",
	});

	if (!levelChoice) {
		return;
	}

	// Determine template
	const templateChoice = await vscode.window.showQuickPick([
		{
			label: "$(file) Default",
			description: "Basic configuration for general Rust projects",
			value: "default",
		},
		{
			label: "$(globe) Web",
			description: "Configuration for web frameworks (Leptos, Dioxus, etc.)",
			value: "web",
		},
		{
			label: "$(device-desktop) Desktop",
			description: "Configuration for desktop applications (Tauri, egui, etc.)",
			value: "desktop",
		},
		{
			label: "$(game) Game",
			description: "Configuration for game development (Bevy, etc.)",
			value: "game",
		},
		{
			label: "$(library) Library",
			description: "Configuration for library crates",
			value: "library",
		},
	], {
		placeHolder: "Select a configuration template",
		title: "RAZ Configuration Template",
	});

	if (!templateChoice) {
		return;
	}

	try {
		const razPath = await ensureRazExecutable(context);
		if (!razPath) {
			return;
		}

		// Build init command args based on level
		const args = ["init", "--template", templateChoice.value];
		
		// Determine the working directory based on level
		let cwd = workspaceFolders[0].uri.fsPath;
		
		if (levelChoice.value === "project") {
			// For project level, try to find the nearest Cargo.toml
			const activeEditor = vscode.window.activeTextEditor;
			if (activeEditor && activeEditor.document.languageId === "rust") {
				cwd = path.dirname(activeEditor.document.uri.fsPath);
			}
		} else if (levelChoice.value === "global") {
			// For global level, use home directory
			const homeDir = process.env.HOME || process.env.USERPROFILE;
			if (homeDir) {
				cwd = homeDir;
			}
		}

		// Add --force flag if user wants to overwrite
		const existingConfig = await vscode.workspace.fs.stat(
			vscode.Uri.file(path.join(cwd, ".raz", "config.toml"))
		).then(() => true, () => false);

		if (existingConfig) {
			const overwrite = await vscode.window.showWarningMessage(
				"RAZ configuration already exists. Overwrite?",
				{ modal: true },
				"Overwrite",
				"Cancel"
			);
			if (overwrite !== "Overwrite") {
				return;
			}
			args.push("--force");
		}

		// Execute init command
		await executeRazAsTask(context, razPath, args, {
			cwd,
			label: `RAZ: Initialize ${levelChoice.label} Configuration`,
		});

		outputChannel.appendLine(`✅ RAZ configuration initialized at ${levelChoice.label} level with ${templateChoice.label} template`);
		
		// Show success message
		vscode.window.showInformationMessage(
			`RAZ configuration initialized successfully at ${levelChoice.label} level!`,
			"Open Config"
		).then(selection => {
			if (selection === "Open Config") {
				const configPath = path.join(cwd, ".raz", "config.toml");
				vscode.workspace.openTextDocument(configPath).then(doc => {
					vscode.window.showTextDocument(doc);
				});
			}
		});

	} catch (error) {
		vscode.window.showErrorMessage(`Failed to initialize RAZ config: ${error}`);
		outputChannel.appendLine(`Init error: ${error}`);
	}
}




async function listOverridesForFile(context: vscode.ExtensionContext): Promise<void> {
	try {
		// Check if we have an active editor
		const editor = vscode.window.activeTextEditor;
		if (!editor || editor.document.languageId !== "rust") {
			vscode.window.showErrorMessage("RAZ: Please open a Rust file to list its overrides");
			return;
		}

		const razPath = await ensureRazExecutable(context);
		if (!razPath) {
			return;
		}
		
		// Find the project root for the current file
		const filePath = editor.document.uri.fsPath;
		const projectDir = findNearestRazDirectory();
		
		if (!projectDir) {
			vscode.window.showInformationMessage("No RAZ configuration found for the current file's project");
			return;
		}
		
		// Use 'override list --file' to list overrides for specific file
		const args = ["override", "list", "--file", filePath];
		await executeRazAsTask(context, razPath, args, {
			cwd: projectDir,
			label: `RAZ: List Overrides for ${path.basename(filePath)}`,
		});
	} catch (error) {
		vscode.window.showErrorMessage(`Failed to list overrides for file: ${error}`);
	}
}

async function showStatsForFile(context: vscode.ExtensionContext): Promise<void> {
	try {
		// Check if we have an active editor
		const editor = vscode.window.activeTextEditor;
		if (!editor || editor.document.languageId !== "rust") {
			vscode.window.showErrorMessage("RAZ: Please open a Rust file to show its override statistics");
			return;
		}

		const razPath = await ensureRazExecutable(context);
		if (!razPath) {
			return;
		}
		
		// For now, we'll show stats for the whole project but filtered by file
		// Since raz CLI doesn't have a stats --file option yet
		const filePath = editor.document.uri.fsPath;
		const projectDir = findNearestRazDirectory();
		
		if (!projectDir) {
			vscode.window.showInformationMessage("No RAZ configuration found for the current file's project");
			return;
		}
		
		// First list overrides for the file, then show general stats
		const listArgs = ["override", "list", "--file", filePath];
		await executeRazAsTask(context, razPath, listArgs, {
			cwd: projectDir,
			label: `RAZ: Overrides for ${path.basename(filePath)}`,
		});
		
		// Also show general stats
		const statsArgs = ["override", "stats"];
		await executeRazAsTask(context, razPath, statsArgs, {
			cwd: projectDir,
			label: `RAZ: Override Statistics (${path.basename(projectDir)})`,
		});
	} catch (error) {
		vscode.window.showErrorMessage(`Failed to show stats for file: ${error}`);
	}
}

async function clearOverridesForFile(context: vscode.ExtensionContext): Promise<void> {
	try {
		// Check if we have an active editor
		const editor = vscode.window.activeTextEditor;
		if (!editor || editor.document.languageId !== "rust") {
			vscode.window.showErrorMessage("RAZ: Please open a Rust file to clear its overrides");
			return;
		}

		const filePath = editor.document.uri.fsPath;
		const projectDir = findNearestRazDirectory();
		
		if (!projectDir) {
			vscode.window.showInformationMessage("No RAZ configuration found for the current file's project");
			return;
		}

		// First, list overrides for the file to see what will be cleared
		const razPath = await ensureRazExecutable(context);
		if (!razPath) {
			return;
		}

		const result = await vscode.window.showWarningMessage(
			`Are you sure you want to clear all overrides for ${path.basename(filePath)}?`,
			{ modal: true, detail: "This action cannot be undone." },
			"Clear",
			"Cancel"
		);

		if (result === "Clear") {
			// Since raz CLI doesn't have a clear --file option, we need to:
			// 1. List all overrides
			// 2. Find ones for this file
			// 3. Delete them individually
			vscode.window.showInformationMessage(
				"Note: File-specific clearing is not yet implemented in RAZ CLI. Please use 'Clear All Overrides' and re-add the ones you want to keep.",
				"OK"
			);
		}
	} catch (error) {
		vscode.window.showErrorMessage(`Failed to clear overrides for file: ${error}`);
	}
}

export function deactivate(): void {
	if (terminal) {
		terminal.dispose();
	}
}
