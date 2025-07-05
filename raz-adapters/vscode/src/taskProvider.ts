/*---------------------------------------------------------------------------------------------
 *  RAZ Task Provider - Run commands as VS Code tasks for better concurrency
 *--------------------------------------------------------------------------------------------*/
import * as vscode from "vscode";

export interface RazTaskDefinition extends vscode.TaskDefinition {
	type: "raz";
	command: string;
	args: string[];
	env?: Record<string, string>;
	cwd?: string;
	label?: string;
}

export class RazTaskProvider implements vscode.TaskProvider {
	static razTaskType = "raz";
	private tasks: vscode.Task[] | undefined;

	constructor(private context: vscode.ExtensionContext) {}

	public provideTasks(): vscode.ProviderResult<vscode.Task[]> {
		return this.tasks;
	}

	public resolveTask(_task: vscode.Task): vscode.Task | undefined {
		const definition = _task.definition as RazTaskDefinition;
		return this.createTask(definition);
	}

	public createTask(definition: RazTaskDefinition): vscode.Task {
		// Build the complete command
		const fullCommand = [definition.command, ...definition.args].join(" ");
		const taskName = definition.label || `RAZ: ${definition.args[0] || "command"}`;

		// Create shell execution options
		const shellOptions: vscode.ShellExecutionOptions = {};
		
		if (definition.env) {
			shellOptions.env = definition.env;
		}
		
		if (definition.cwd) {
			shellOptions.cwd = definition.cwd;
		}

		// Create the task
		const task = new vscode.Task(
			definition,
			vscode.TaskScope.Workspace,
			taskName,
			"raz",
			new vscode.ShellExecution(fullCommand, shellOptions),
			"$rustc" // Use Rust problem matcher
		);

		// Configure task presentation
		task.presentationOptions = {
			reveal: vscode.TaskRevealKind.Always,
			panel: vscode.TaskPanelKind.Dedicated,
			clear: false,
			echo: true,
		};

		// Mark framework dev tasks as background tasks
		if (this.isLongRunningTask(definition)) {
			task.isBackground = true;
			// Use the rustc problem matcher for Rust compilation errors
			task.problemMatchers = ["$rustc"];
		}

		return task;
	}

	private isLongRunningTask(definition: RazTaskDefinition): boolean {
		const longRunningCommands = [
			"leptos watch",
			"trunk serve",
			"tauri dev",
			"dx serve",
			"cargo watch",
			"cargo run --watch"
		];

		const commandStr = definition.args.join(" ");
		return longRunningCommands.some(cmd => commandStr.includes(cmd));
	}
}

/**
 * Execute a RAZ command as a VS Code task
 * This allows multiple commands to run concurrently without blocking
 */
export async function executeRazAsTask(
	context: vscode.ExtensionContext,
	razPath: string,
	args: string[],
	options?: {
		env?: Record<string, string>;
		cwd?: string;
		label?: string;
	}
): Promise<vscode.TaskExecution> {
	const definition: RazTaskDefinition = {
		type: "raz",
		command: razPath,
		args: args,
		env: options?.env,
		cwd: options?.cwd,
		label: options?.label
	};

	const taskProvider = new RazTaskProvider(context);
	const task = taskProvider.createTask(definition);

	// Check if a similar task is already running
	const runningTasks = vscode.tasks.taskExecutions;
	for (const execution of runningTasks) {
		if (execution.task.name === task.name && execution.task.source === "raz") {
			const action = await vscode.window.showInformationMessage(
				`Task "${task.name}" is already running. What would you like to do?`,
				"Show Task",
				"Start New",
				"Cancel"
			);

			if (action === "Show Task") {
				// Show the terminal for the existing task
				const terminals = vscode.window.terminals;
				for (const terminal of terminals) {
					if (terminal.name.includes(task.name)) {
						terminal.show();
						break;
					}
				}
				return execution;
			} else if (action === "Cancel") {
				throw new Error("Task execution cancelled");
			}
			// If "Start New", continue to create a new task
		}
	}

	return vscode.tasks.executeTask(task);
}

/**
 * Register the RAZ task provider
 */
export function registerTaskProvider(context: vscode.ExtensionContext): vscode.Disposable {
	const provider = new RazTaskProvider(context);
	return vscode.tasks.registerTaskProvider(RazTaskProvider.razTaskType, provider);
}