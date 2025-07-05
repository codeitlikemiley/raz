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
			const { exec } = require('child_process');
			exec(`"${binaryPath}" --version`, (error: any, stdout: string, stderr: string) => {
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
	const binaryManager = new RazBinaryManager(context, outputChannel);
	
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
				const supportseSelfUpdate = isVersionAtLeast(currentVersion, '0.2.1');
				
				let choice;
				if (supportseSelfUpdate) {
					choice = await vscode.window.showInformationMessage(
						`RAZ update available: v${currentVersion} → ${latestVersion}`,
						"Update with CLI",
						"Download Binary",
						"Later"
					);
				} else {
					// Older version without self-update command
					choice = await vscode.window.showInformationMessage(
						`RAZ update available: v${currentVersion} → ${latestVersion}\n(Your version doesn't support self-update)`,
						"Download Binary",
						"Manual Update",
						"Later"
					);
				}
				
				if (choice === "Update with CLI") {
					// Open terminal and run raz self-update
					const terminal = vscode.window.createTerminal("RAZ Update");
					terminal.show();
					terminal.sendText("raz self-update");
				} else if (choice === "Download Binary") {
					// Use VSCode to download and replace the binary
					await downloadAndReplaceSystemBinary(context, latestVersion);
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
		const { exec } = require('child_process');
		exec('raz --version', (error: any, stdout: string) => {
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
		
		if (currentPart > requiredPart) return true;
		if (currentPart < requiredPart) return false;
	}
	
	return true; // Equal versions
}

/// Download and replace system RAZ binary
async function downloadAndReplaceSystemBinary(context: vscode.ExtensionContext, version: string): Promise<void> {
	const outputChannel = ensureOutputChannel();
	
	try {
		// First, try to find where the current raz binary is located
		const razPath = await findRazBinaryPath();
		if (!razPath) {
			throw new Error("Could not locate RAZ binary");
		}
		
		outputChannel.appendLine(`Found RAZ binary at: ${razPath}`);
		
		// Download the new binary to a temporary location
		const binaryManager = new RazBinaryManager(context, outputChannel);
		const tempBinaryPath = await binaryManager.ensureBinary(); // Downloads latest
		
		// Show progress
		await vscode.window.withProgress(
			{
				location: vscode.ProgressLocation.Notification,
				title: "Updating RAZ",
				cancellable: false,
			},
			async (progress) => {
				progress.report({ message: "Replacing binary..." });
				
				// Copy the new binary over the old one
				const fs = require('fs');
				
				// Make backup of original
				const backupPath = `${razPath}.backup`;
				if (fs.existsSync(razPath)) {
					fs.copyFileSync(razPath, backupPath);
					outputChannel.appendLine(`Created backup: ${backupPath}`);
				}
				
				try {
					// Replace the binary
					fs.copyFileSync(tempBinaryPath, razPath);
					// Make it executable on Unix systems
					if (process.platform !== 'win32') {
						fs.chmodSync(razPath, 0o755);
					}
					
					outputChannel.appendLine(`Successfully updated RAZ binary to ${version}`);
					vscode.window.showInformationMessage(`RAZ updated to ${version}! Run 'raz --version' to verify.`);
					
					// Clean up backup if successful
					if (fs.existsSync(backupPath)) {
						fs.unlinkSync(backupPath);
					}
				} catch (replaceError) {
					// Restore backup if replacement failed
					if (fs.existsSync(backupPath)) {
						fs.copyFileSync(backupPath, razPath);
						fs.unlinkSync(backupPath);
						outputChannel.appendLine("Restored backup after failed update");
					}
					throw replaceError;
				}
			},
		);
	} catch (error) {
		outputChannel.appendLine(`Binary update failed: ${error}`);
		vscode.window.showErrorMessage(`Failed to update RAZ binary: ${error}`);
	}
}

/// Find the path to the RAZ binary in system PATH
async function findRazBinaryPath(): Promise<string | null> {
	return new Promise((resolve) => {
		const { exec } = require('child_process');
		const command = process.platform === 'win32' ? 'where raz' : 'which raz';
		
		exec(command, (error: any, stdout: string) => {
			if (error) {
				resolve(null);
				return;
			}
			
			const paths = stdout.trim().split('\n');
			// Return the first valid path
			for (const path of paths) {
				const trimmedPath = path.trim();
				if (trimmedPath && require('fs').existsSync(trimmedPath)) {
					resolve(trimmedPath);
					return;
				}
			}
			
			resolve(null);
		});
	});
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
			const supportseSelfUpdate = isVersionAtLeast(currentVersion, '0.2.1');
			
			let choice;
			if (supportseSelfUpdate) {
				choice = await vscode.window.showQuickPick([
					{ label: "Update with CLI", description: "Use 'raz self-update' command" },
					{ label: "Download Binary", description: "VSCode downloads and replaces binary" }
				], {
					placeHolder: "Choose update method"
				});
			} else {
				choice = await vscode.window.showQuickPick([
					{ label: "Download Binary", description: "VSCode downloads and replaces binary" },
					{ label: "Manual Update", description: "Show manual update instructions" }
				], {
					placeHolder: "Your version doesn't support self-update"
				});
			}
			
			if (!choice) return;
			
			if (choice.label === "Update with CLI") {
				const terminal = vscode.window.createTerminal("RAZ Update");
				terminal.show();
				terminal.sendText("raz self-update");
			} else if (choice.label === "Download Binary") {
				try {
					const binaryManager = new RazBinaryManager(context, outputChannel);
					const latestVersion = await binaryManager.getLatestVersion();
					await downloadAndReplaceSystemBinary(context, latestVersion);
				} catch (error) {
					vscode.window.showErrorMessage(`Update failed: ${error}`);
				}
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
					} catch (error) {
						vscode.env.openExternal(vscode.Uri.parse("https://github.com/codeitlikemiley/raz/releases/latest"));
					}
				}
			}
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
