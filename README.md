# PhantomEditor

PhantomEditor is a lightweight, terminal-based text editor written in Rust. It combines the simplicity of a basic text editor with some powerful features inspired by Vim.

## Features

- Simple and intuitive interface
- Vim-like modal editing (Normal, Insert, and Command modes)
- Basic text manipulation operations
- File opening and saving capabilities
- Customizable (extendable through Rust)

## Installation

1. Ensure you have Rust and Cargo installed on your system. If not, install them from [https://www.rust-lang.org/](https://www.rust-lang.org/).

2. Clone this repository:
   ```
   git clone https://github.com/yourusername/PhantomEditor.git
   ```

3. Navigate to the project directory:
   ```
   cd PhantomEditor
   ```

4. Build the project:
   ```
   cargo build --release
   ```

5. The executable will be created in the `target/release` directory.

## Usage

To start PhantomEditor:
```
./PhantomEditor [filename]
```


If a filename is provided, PhantomEditor will attempt to open that file. Otherwise, it will start with a blank document.

## Keybinds and Commands

### Global

- `Ctrl+Q`: Quit the editor

### Normal Mode

- `i`: Enter Insert mode
- `a`: Enter Insert mode after the cursor
- `o`: Insert a new line below and enter Insert mode
- `O`: Insert a new line above and enter Insert mode
- `dd`: Delete the current line
- `yy`: Yank (copy) the current line
- `p`: Paste the copied/deleted line after the current line
- `P`: Paste the copied/deleted line before the current line
- `u`: Undo (placeholder, not implemented)
- `r`: Redo (placeholder, not implemented)
- Arrow keys: Move the cursor
- `Home`: Move to the start of the line
- `End`: Move to the end of the line
- `Delete`: Delete the character under the cursor
- `:`: Enter Command mode

### Insert Mode

- `Esc`: Return to Normal mode
- `Enter`: Insert a new line
- `Backspace`: Delete the character before the cursor
- Any character key: Insert the character at the cursor position

### Command Mode

- `:w`: Save the current file
- `:w filename`: Save the current file as 'filename'
- `:q`: Quit the editor
- `:wq`: Save and quit
- `:e filename`: Open 'filename' for editing