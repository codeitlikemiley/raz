# RAZ VS Code Extension - Installation Guide

## Install Pre-built Extension (Recommended)

Download `.vsix` from releases, then install:

**VS Code UI**: `Cmd+Shift+P` → `Extensions: Install from VSIX...` → Select file

**Extensions View**: `Cmd+Shift+X` → "..." menu → "Install from VSIX..."

**Command Line**: `code --install-extension path/to/raz-vscode-*.vsix`

## Build from Source

```bash
npm install -g @vscode/vsce
cd raz-adapters/vscode
npm install && npm run bundle
code --install-extension raz-vscode-*.vsix
```

## Usage

- `Cmd+R` / `Ctrl+R` to run current context
- Set breakpoints for auto-debug mode
- Search "raz" in VS Code settings for options

## Troubleshooting

**Extension not working**: Check View → Output → "RAZ" for errors

**Build issues**: Run `npm run compile` to see TypeScript errors

**Platform issues**: 
- macOS: Allow binary in Security & Privacy
- Windows: Ensure `.exe` extension present  
- Linux: Check binary permissions

## Development

Open `raz-adapters/vscode` in VS Code, press `F5` to launch development window