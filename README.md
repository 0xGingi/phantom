# phantom

phantom is a lightweight, terminal-based text editor written in Rust. It combines the simplicity of a basic text editor with some powerful features inspired by Vim.

<img src="https://github.com/user-attachments/assets/a48d515b-f5bd-4cdc-b715-20f0b46ccb27" width=50% height=50%><img src="https://github.com/user-attachments/assets/cae5dc0e-c6d1-4dd1-b265-803b8c1d3b4b" width=50% height=50%>

## Features

- Simple and intuitive interface
- Vim-like modal editing (Normal, Insert, Visual, and Command modes)
- Syntax highlighting
- System clipboard integration
- Customizable (Currently Keybindings and Colors)
- Directory Navigation (Sidebar)
- Debug Output Menu
- Search in file
- Undo and Redo
- Tabs
- Minimap

## Installation

### Arch User Repository

#### Binary

[![binary](https://img.shields.io/aur/version/phantom-editor-bin)](https://aur.archlinux.org/packages/phantom-editor-bin)

#### Git

[![git](https://img.shields.io/aur/version/phantom-editor-git)](https://aur.archlinux.org/packages/phantom-editor-git)

### Binary Release

Download latest phantom executable from [releases](https://github.com/0xGingi/phantom/releases)

Place executable in /usr/bin

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
./phantom
./phantom file.txt
./phantom ~/Project
```

If a filename is provided, phantom will attempt to open that file. Otherwise, it will start with a blank document.
If a directory is provided, phantom will enter directory navigation mode

## Default Keybinds and Commands

### ~/.config/phantom/config.toml - Keybinds
### ~/.config/phantom/colors.json - Colors

You can change the keybinds in the config file and the colors in the colors file.

### Global

- `Ctrl+Q`: Quit the editor

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

### Insert Mode

- `Esc`: Return to Normal mode
- `Enter`: Insert a new line
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

### Search Mode

- `Enter`: Perform search and return to Normal mode
- `Esc`: Cancel search and return to Normal mode

## Debug Output

phantom includes a debug output area that displays information about key presses, cursor position, and the results of operations like saving files.
