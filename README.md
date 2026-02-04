# phantom

phantom is a lightweight, terminal-based linux text editor written in Rust. It combines the simplicity of a basic text editor with some powerful features inspired by Vim.

![image](https://github.com/user-attachments/assets/3f71a03d-68c3-4be0-b199-9e40a799d577)


## Features

- Simple and intuitive interface
- Vim-like modal editing (Normal, Insert, Visual, and Command modes)
- Syntax highlighting
- System clipboard integration
- Customizable (Currently Keybindings and Colors)
- Directory Navigation (Sidebar)
- CLI Coding Agents (Sidebar)
- Debug Output Menu
- Search in file
- Undo and Redo
- Tabs
- Minimap
- Git status
- Smart auto-indentation
- Enhanced search with regex and case-insensitive options
- Intelligent undo/redo with operation grouping
- Swappable Themes

## Cross-Plaform Status

- Linux: 100%
- MacOS: Not Planned
- Windows: Not Planned

## Installation

### Arch User Repository

#### Binary

[![binary](https://img.shields.io/aur/version/phantom-editor-bin)](https://aur.archlinux.org/packages/phantom-editor-bin)

#### Git

[![git](https://img.shields.io/aur/version/phantom-editor-git)](https://aur.archlinux.org/packages/phantom-editor-git)

### Binary Release

Download latest phantom executable from [releases](https://github.com/0xGingi/phantom/releases)

Place executable in /usr/bin (or in any folder in your path)

### Build From Source

1. Ensure you have Rust and Cargo installed on your system. If not, install them from [https://www.rust-lang.org/](https://www.rust-lang.org/).

2. Clone this repository:
   ```
   git clone https://github.com/0xGingi/phantom.git
   ```

3. Navigate to the project directory:
   ```
   cd phantom
   ```

4. Build the project:
   ```
   cargo build --release
   ```

5. The executable will be created in the `target/release` directory.

## Usage

To start phantom:
```
phantom
phantom file.txt
phantom ~/Project
```

If a filename is provided, phantom will attempt to open that file. Otherwise, it will start with a blank document.
If a directory is provided, phantom will enter directory navigation mode

## Default Keybinds and Commands

### Config file locations

- Linux: `~/.config/phantom`

### Global

- `Ctrl+Q`: Quit the editor
- `?`: Show keybinding help

### Normal Mode

- `i` or `Insert` : Enter Insert mode
- `a`: Enter Insert mode after the cursor
- `o`: Insert a new line below and enter Insert mode
- `O`: Insert a new line above and enter Insert mode
- `dd`: Delete the current line
- `yy`: Yank (copy) the current line
- `p`: Paste after the current line
- `Ctrl+Y`: Copy the current line to system clipboard
- `Ctrl+P`: Paste from system clipboard below the current line
- `v`: Enter Visual mode
- Arrow keys: Move the cursor
- `Home`: Move to the start of the line
- `End`: Move to the end of the line
- `Delete`: Delete the character under the cursor
- `:`: Enter Command mode
- `Ctrl+B`: Toggle debug menu visibility
- `Ctrl+E`: Enter directory navigation mode
- `Ctrl+G`: Open agents sidebar
- `/`: Enter Search mode
- `n`: Go to next search result
- `N`: Go to previous search result
- `PageUp`: Scroll up one page
- `PageDown`: Scroll down one page
- `Ctrl+U`: Undo
- `Ctrl+R`: Redo
- `Ctrl+T`: New Tab
- `Ctrl+W`: Close Tab
- `F1`-`F9`: Switch to Tab 1-9
- `Tab`: Swap Between Tabs
- `Ctrl+M`: Toggle Minimap
- `Ctrl+L`: Toggle line number mode (Off/Absolute/Relative/Hybrid)
- `Ctrl+I`: Toggle auto-indentation
- `Ctrl+J`: Toggle word wrap
- `Shift+T`: Cycle through color themes
- `?`: Show keybinding help
- `Mouse Click`: Move cursor to clicked position
- `Mouse Wheel`: Scroll up/down (3 lines)
- `Shift+Mouse Wheel`: Scroll left/right (5 columns)

### Insert Mode

- `Esc`: Return to Normal mode
- `Enter`: Insert a new line with smart auto-indentation
- `Backspace`: Delete the character before the cursor
- Any character key: Insert the character at the cursor position

### Visual Mode

- `Esc`: Return to Normal mode
- `y`: Copy selected text to system clipboard
- Arrow keys: Extend selection

### Command Mode

- `:w`: Save the current file
- `:w filename`: Save the current file as 'filename'
- `:q`: Quit the editor
- `:wq`: Save and quit
- `:e filename`: Open 'filename' for editing
- `:agents`: Toggle agents sidebar
- `:agent <command>`: Launch a custom agent command in the sidebar

### Search Mode

- `Enter`: Perform search and return to Normal mode
- `Esc`: Cancel search and return to Normal mode
- `Alt+I`: Toggle case-sensitive search
- `Alt+R`: Toggle regex search mode
- Any character: Live search as you type

## Enhanced Features

### Smart Auto-Indentation
- Automatically indents new lines based on the previous line
- Recognizes common programming constructs (if, for, while, brackets)
- Supports both tabs and spaces with configurable indent size
- Toggle with `Ctrl+I`

### Advanced Search
- **Live Search**: Results update as you type
- **Case Sensitivity**: Toggle with `Alt+I` in search mode
- **Regex Support**: Toggle with `Alt+R` in search mode
- **Multiple Matches**: Finds all occurrences in the file
- Use `n` and `N` to navigate between search results

### Intelligent Undo/Redo
- Groups consecutive character insertions/deletions into logical operations
- Time-based grouping (operations within 1 second)
- More intuitive undo behavior for better editing flow

### Mouse Support
- **Click to Position**: Click anywhere in the editor to move cursor
- **Vertical Scrolling**: Mouse wheel scrolls up/down
- **Horizontal Scrolling**: Shift+mouse wheel scrolls left/right
- **Text Selection**: Drag to select text, right-click to copy

### Line Numbers
- **Absolute**: Shows actual line numbers (1, 2, 3...)
- **Relative**: Shows distance from current line
- **Hybrid**: Shows current line number + relative distances
- Toggle modes with `Ctrl+L`

### Color Themes
- **One Dark** (default): Modern dark theme inspired by Atom/VS Code
- **Dracula**: Popular purple and pink accent theme
- **Solarized Dark**: Scientifically designed color scheme, easy on eyes
- **Nord**: Arctic-inspired cool blue theme
- **Monokai**: Classic warm colors, popular in Sublime Text
- **Gruvbox**: Retro groove colors with warm, earthy tones
- Cycle through themes with `Shift+T`

## Debug Output

phantom includes a debug output area that displays information about key presses, cursor position, and the results of operations like saving files.
