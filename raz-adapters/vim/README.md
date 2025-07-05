# Vim/Neovim Integration Guide

This guide shows how to integrate RAZ with Vim and Neovim for seamless Rust development. With these configurations, you'll get VS Code-like functionality using RAZ's universal command generation and override persistence.

## Prerequisites

1. **Install RAZ CLI**:
   ```bash
   cargo install raz-cli
   ```

2. **Verify Installation**:
   ```bash
   raz --version
   ```

## Quick Setup

Add this to your `~/.vimrc` (Vim) or `~/.config/nvim/init.vim` (Neovim):

```vim
" RAZ Integration for Rust development
" Map Ctrl+R to run current file with RAZ
autocmd FileType rust nnoremap <buffer> <C-r> :call RAZRun()<CR>
" Map Ctrl+Shift+R to run with override input
autocmd FileType rust nnoremap <buffer> <C-S-r> :call RAZRunWithOverride()<CR>

function! RAZRun()
    let l:file_path = expand('%:p')
    let l:line = line('.')
    let l:col = col('.')
    let l:raz_command = 'raz "' . l:file_path . ':' . l:line . ':' . l:col . '"'
    
    " Save file if modified
    if &modified
        write
    endif
    
    " Run in terminal
    execute '!clear && ' . l:raz_command
endfunction

function! RAZRunWithOverride()
    let l:file_path = expand('%:p')
    let l:line = line('.')
    let l:col = col('.')
    
    " Prompt for override options
    let l:override = input('Override options (e.g., RUST_BACKTRACE=1 --release -- --exact): ')
    if l:override == ''
        echo "\nCancelled."
        return
    endif
    
    let l:raz_command = 'raz --save-override "' . l:file_path . ':' . l:line . ':' . l:col . '" ' . l:override
    
    " Save file if modified
    if &modified
        write
    endif
    
    " Run in terminal
    execute '!clear && ' . l:raz_command
endfunction
```

## Neovim Lua Configuration

For modern Neovim users using Lua configuration (`~/.config/nvim/init.lua`):

```lua
-- RAZ Integration for Rust development
local function raz_run()
    local file_path = vim.fn.expand('%:p')
    local line = vim.fn.line('.')
    local col = vim.fn.col('.')
    local raz_command = string.format('raz "%s:%d:%d"', file_path, line, col)
    
    -- Save file if modified
    if vim.bo.modified then
        vim.cmd('write')
    end
    
    -- Run in terminal
    vim.cmd('!clear && ' .. raz_command)
end

local function raz_run_with_override()
    local file_path = vim.fn.expand('%:p')
    local line = vim.fn.line('.')
    local col = vim.fn.col('.')
    
    -- Prompt for override options
    local override = vim.fn.input('Override options (e.g., RUST_BACKTRACE=1 --release -- --exact): ')
    if override == '' then
        print('\nCancelled.')
        return
    end
    
    local raz_command = string.format('raz --save-override "%s:%d:%d" %s', file_path, line, col, override)
    
    -- Save file if modified
    if vim.bo.modified then
        vim.cmd('write')
    end
    
    -- Run in terminal
    vim.cmd('!clear && ' .. raz_command)
end

-- Set up keymaps for Rust files
vim.api.nvim_create_autocmd('FileType', {
    pattern = 'rust',
    callback = function()
        local opts = { buffer = true, silent = true }
        vim.keymap.set('n', '<C-r>', raz_run, opts)
        vim.keymap.set('n', '<C-S-r>', raz_run_with_override, opts)
    end,
})
```

## Advanced Neovim Configuration

For a more sophisticated setup with better UI and error handling:

```lua
-- Advanced RAZ Integration for Neovim
local M = {}

-- Configuration
M.config = {
    save_on_run = true,
    show_command = true,
    use_floating_terminal = true,
}

-- Run RAZ command in floating terminal (requires nvim 0.7+)
local function run_in_floating_terminal(command)
    local buf = vim.api.nvim_create_buf(false, true)
    local width = math.ceil(vim.o.columns * 0.8)
    local height = math.ceil(vim.o.lines * 0.8)
    local row = math.ceil((vim.o.lines - height) / 2)
    local col = math.ceil((vim.o.columns - width) / 2)
    
    local win = vim.api.nvim_open_win(buf, true, {
        relative = 'editor',
        width = width,
        height = height,
        row = row,
        col = col,
        style = 'minimal',
        border = 'rounded',
        title = ' RAZ Output ',
        title_pos = 'center',
    })
    
    -- Run command in terminal
    vim.fn.termopen(command, {
        on_exit = function(_, exit_code)
            if exit_code == 0 then
                print('✓ RAZ command completed successfully')
            else
                print('✗ RAZ command failed with exit code: ' .. exit_code)
            end
        end
    })
    
    -- Enter insert mode in terminal
    vim.cmd('startinsert')
    
    -- Close window with 'q' in normal mode
    vim.keymap.set('n', 'q', function()
        vim.api.nvim_win_close(win, true)
    end, { buffer = buf })
end

-- Main RAZ run function
function M.run(with_override)
    local file_path = vim.fn.expand('%:p')
    local line = vim.fn.line('.')
    local col = vim.fn.col('.')
    
    -- Check if file is a Rust file
    if vim.bo.filetype ~= 'rust' then
        print('RAZ: Not a Rust file')
        return
    end
    
    -- Save file if configured and modified
    if M.config.save_on_run and vim.bo.modified then
        vim.cmd('write')
        print('File saved')
    end
    
    local raz_command
    if with_override then
        local override = vim.fn.input('Override options: ')
        if override == '' then
            print('\nCancelled.')
            return
        end
        raz_command = string.format('raz --save-override "%s:%d:%d" %s', file_path, line, col, override)
    else
        raz_command = string.format('raz "%s:%d:%d"', file_path, line, col)
    end
    
    -- Show command if configured
    if M.config.show_command then
        print('Running: ' .. raz_command)
    end
    
    -- Run command
    if M.config.use_floating_terminal and vim.fn.has('nvim-0.7') == 1 then
        run_in_floating_terminal(raz_command)
    else
        vim.cmd('!' .. raz_command)
    end
end

-- Override management functions
function M.list_overrides()
    vim.cmd('!raz override list')
end

function M.clear_overrides()
    local confirm = vim.fn.confirm('Clear all RAZ overrides?', '&Yes\n&No', 2)
    if confirm == 1 then
        vim.cmd('!raz override clear')
    end
end

function M.override_stats()
    vim.cmd('!raz override stats')
end

-- Set up keymaps
function M.setup_keymaps()
    local opts = { silent = true, desc = 'RAZ' }
    
    -- Basic keymaps
    vim.keymap.set('n', '<leader>rr', function() M.run(false) end, 
        vim.tbl_extend('force', opts, { desc = 'RAZ: Run' }))
    vim.keymap.set('n', '<leader>ro', function() M.run(true) end, 
        vim.tbl_extend('force', opts, { desc = 'RAZ: Run with Override' }))
    
    -- Override management
    vim.keymap.set('n', '<leader>rl', M.list_overrides, 
        vim.tbl_extend('force', opts, { desc = 'RAZ: List Overrides' }))
    vim.keymap.set('n', '<leader>rc', M.clear_overrides, 
        vim.tbl_extend('force', opts, { desc = 'RAZ: Clear Overrides' }))
    vim.keymap.set('n', '<leader>rs', M.override_stats, 
        vim.tbl_extend('force', opts, { desc = 'RAZ: Override Stats' }))
end

-- Auto-setup for Rust files
vim.api.nvim_create_autocmd('FileType', {
    pattern = 'rust',
    callback = function()
        -- File-specific keymaps (Ctrl+R like VS Code)
        local opts = { buffer = true, silent = true }
        vim.keymap.set('n', '<C-r>', function() M.run(false) end, opts)
        vim.keymap.set('n', '<C-S-r>', function() M.run(true) end, opts)
    end,
})

-- Initialize
M.setup_keymaps()

return M
```

## Key Mappings

### Default Mappings

| Key | Mode | Action |
|-----|------|--------|
| `<C-r>` | Normal | Run current file with RAZ |
| `<C-S-r>` | Normal | Run with override input |

### Advanced Mappings (Leader-based)

| Key | Mode | Action |
|-----|------|--------|
| `<leader>rr` | Normal | Run RAZ |
| `<leader>ro` | Normal | Run with override |
| `<leader>rl` | Normal | List saved overrides |
| `<leader>rc` | Normal | Clear all overrides |
| `<leader>rs` | Normal | Show override statistics |

## Usage Examples

### Basic Usage

1. **Open a Rust file** in Vim/Neovim
2. **Position cursor** on a test function or main function
3. **Press `<C-r>`** to run with RAZ
4. **View output** in terminal

### Saving Overrides

1. **Position cursor** on a test function
2. **Press `<C-S-r>`** (or `<leader>ro`)
3. **Enter overrides** when prompted:
   ```
   RUST_BACKTRACE=1 --release -- --exact --nocapture
   ```
4. **Future runs** automatically use saved overrides

### Override Management

```vim
" List all saved overrides
:call RAZListOverrides()

" Clear specific override (via CLI)
:!raz override clear src/lib.rs:test_function

" Show override statistics
:!raz override stats
```

## Terminal Integration

For users who prefer terminal-based workflow:

### Tmux Integration

Add to `~/.tmux.conf`:

```bash
# RAZ integration for Rust development
bind-key r run-shell 'cd #{pane_current_path} && raz #{pane_current_path}/src/main.rs'
```

### Shell Aliases

Add to `~/.bashrc` or `~/.zshrc`:

```bash
# RAZ shortcuts
alias rr='raz'
alias rro='raz --save-override'
alias rrl='raz override list'
alias rrc='raz override clear'
alias rrs='raz override stats'

# Function to run RAZ on current file with cursor position
raz_here() {
    if [ -f "$1" ]; then
        raz "$1:${2:-1}:${3:-1}"
    else
        echo "Usage: raz_here <file> [line] [col]"
    fi
}
```

## Plugin Development

For plugin developers wanting to create full RAZ integration:

### Plugin Structure

```
raz-vim/
├── plugin/
│   └── raz.vim
├── autoload/
│   └── raz.vim
├── doc/
│   └── raz.txt
└── README.md
```

### Basic Plugin Template

`plugin/raz.vim`:
```vim
" RAZ Vim Plugin
if exists('g:loaded_raz') || &cp
    finish
endif
let g:loaded_raz = 1

" Configuration
let g:raz_save_on_run = get(g:, 'raz_save_on_run', 1)
let g:raz_show_command = get(g:, 'raz_show_command', 0)

" Commands
command! RAZRun call raz#run()
command! RAZRunWithOverride call raz#run_with_override()
command! RAZListOverrides call raz#list_overrides()
command! RAZClearOverrides call raz#clear_overrides()

" Auto-setup for Rust files
augroup RAZAutoSetup
    autocmd!
    autocmd FileType rust call raz#setup_buffer()
augroup END
```

## Troubleshooting

### Common Issues

#### "raz command not found"
```bash
# Add to ~/.bashrc or ~/.zshrc
export PATH="$HOME/.cargo/bin:$PATH"
```

#### "Permission denied"
```bash
chmod +x ~/.cargo/bin/raz
```

#### "File not saved"
The functions automatically save files before running. If you see this error:
```vim
:set autowrite
```

#### "Terminal not clearing"
If terminal output is messy:
```vim
" Add to your vimrc
set shell=/bin/bash
```

### Debugging

Enable debug mode to see what commands are being executed:

```vim
" Add to your configuration
let g:raz_debug = 1

function! RAZRun()
    " ... existing code ...
    if exists('g:raz_debug') && g:raz_debug
        echo 'RAZ Command: ' . l:raz_command
    endif
    " ... rest of function ...
endfunction
```

## Advanced Features

### Override Persistence Across Sessions

RAZ automatically saves overrides to `.raz/overrides.toml` in your project. These persist across:
- Vim sessions
- Different IDEs
- Terminal usage

### Framework Detection

RAZ automatically detects frameworks and adjusts commands:
- **Leptos**: Uses `cargo leptos watch` for development
- **Dioxus**: Uses `dx serve` with platform detection
- **Tauri**: Uses `cargo tauri dev`
- **Standard Cargo**: Uses appropriate `cargo` commands

### Test Detection

RAZ uses tree-sitter parsing to accurately detect:
- Individual test functions
- Test modules
- Benchmark functions
- Doc tests

## Integration with Popular Neovim Plugins

### With nvim-tree

```lua
-- Add RAZ command to nvim-tree context menu
local function raz_run_file()
    local node = require('nvim-tree.lib').get_node_at_cursor()
    if node and node.absolute_path:match('%.rs$') then
        vim.cmd('!raz "' .. node.absolute_path .. '"')
    end
end

-- Add to nvim-tree setup
require('nvim-tree').setup({
    view = {
        mappings = {
            list = {
                { key = '<C-r>', action = 'raz_run', action_cb = raz_run_file },
            }
        }
    }
})
```

### With Telescope

```lua
-- Add RAZ command picker to Telescope
local pickers = require('telescope.pickers')
local finders = require('telescope.finders')
local actions = require('telescope.actions')

local function raz_command_picker()
    pickers.new({}, {
        prompt_title = 'RAZ Commands',
        finder = finders.new_table({
            results = {
                { 'Run Current File', 'run' },
                { 'Run with Override', 'override' },
                { 'List Overrides', 'list' },
                { 'Clear Overrides', 'clear' },
                { 'Override Stats', 'stats' },
            },
            entry_maker = function(entry)
                return {
                    value = entry[2],
                    display = entry[1],
                    ordinal = entry[1],
                }
            end
        }),
        attach_mappings = function(prompt_bufnr)
            actions.select_default:replace(function()
                local selection = actions.get_selected_entry()
                actions.close(prompt_bufnr)
                
                if selection.value == 'run' then
                    require('raz').run(false)
                elseif selection.value == 'override' then
                    require('raz').run(true)
                elseif selection.value == 'list' then
                    require('raz').list_overrides()
                elseif selection.value == 'clear' then
                    require('raz').clear_overrides()
                elseif selection.value == 'stats' then
                    require('raz').override_stats()
                end
            end)
            return true
        end,
    }):find()
end

-- Add keymap
vim.keymap.set('n', '<leader>fr', raz_command_picker, { desc = 'RAZ Commands' })
```

## Testing Your Setup

Create a test Rust file to verify your integration:

```rust
// test_raz.rs
#[cfg(test)]
mod tests {
    #[test]
    fn test_basic() {
        assert_eq!(2 + 2, 4);
    }
    
    #[test]
    fn test_with_output() {
        println!("This test should show output");
        assert!(true);
    }
}

fn main() {
    println!("Hello from RAZ!");
}
```

1. **Open the file** in Vim/Neovim
2. **Position cursor** on `test_basic` function
3. **Press `<C-r>`** - should run the specific test
4. **Press `<C-S-r>`** and enter `-- --nocapture` - should save override and show println output
5. **Press `<C-r>`** again - should automatically use the saved `--nocapture` flag

## Contributing

Help improve Vim/Neovim integration:
- Test on different Vim versions
- Report compatibility issues
- Suggest better key mappings
- Contribute to plugin development

---

For more information, see the [main RAZ documentation](../README.md) or [create an issue](https://github.com/codeitlikemiley/raz/issues) for Vim-specific questions.