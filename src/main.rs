use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{error::Error, io};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::env;
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph, List, ListItem, ListState},
    Frame, Terminal,
};
use syntect::easy::HighlightLines;
use syntect::highlighting::{ThemeSet, Style as SyntectStyle};
use syntect::parsing::SyntaxSet;
use clipboard::{ClipboardContext, ClipboardProvider};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Deserialize, Serialize, Clone)]
struct Keybindings {
    normal_mode: HashMap<String, String>,
    insert_mode: HashMap<String, String>,
    visual_mode: HashMap<String, String>,
    command_mode: HashMap<String, String>,
    file_select_mode: HashMap<String, String>,
    search_mode: HashMap<String, String>,
}

#[derive(Clone)]
struct EditOperation {
    content: Vec<String>,
    cursor_position: (usize, usize),
}

impl Keybindings {
    fn default() -> Self {
        Keybindings {
            normal_mode: [
                ("i".to_string(), "enter_insert_mode".to_string()),
                ("Insert".to_string(), "enter_insert_mode".to_string()),
                ("a".to_string(), "append".to_string()),
                ("o".to_string(), "open_line_below".to_string()),
                ("O".to_string(), "open_line_above".to_string()),
                ("dd".to_string(), "delete_line".to_string()),
                ("yy".to_string(), "yank_line".to_string()),
                ("p".to_string(), "paste_after".to_string()),
                ("v".to_string(), "enter_visual_mode".to_string()),
                (":".to_string(), "enter_command_mode".to_string()),
                ("Ctrl+b".to_string(), "toggle_debug_menu".to_string()),
                ("Ctrl+e".to_string(), "enter_directory_nav_mode".to_string()),
                ("/".to_string(), "enter_search_mode".to_string()),
                ("n".to_string(), "next_search_result".to_string()),
                ("N".to_string(), "previous_search_result".to_string()),
                ("Ctrl+y".to_string(), "copy_selection".to_string()),
                ("Ctrl+p".to_string(), "paste_clipboard".to_string()),
                ("Ctrl+u".to_string(), "undo".to_string()),
                ("Ctrl+r".to_string(), "redo".to_string()),
            ].iter().cloned().collect(),
            insert_mode: [
                ("Esc".to_string(), "exit_insert_mode".to_string()),
            ].iter().cloned().collect(),
            visual_mode: [
                ("Esc".to_string(), "exit_visual_mode".to_string()),
                ("y".to_string(), "yank_selection".to_string()),
                ("d".to_string(), "delete_selection".to_string()),
            ].iter().cloned().collect(),
            command_mode: [
                ("Enter".to_string(), "execute_command".to_string()),
                ("Esc".to_string(), "exit_command_mode".to_string()),
            ].iter().cloned().collect(),
            file_select_mode: [
                ("Enter".to_string(), "select_file".to_string()),
                ("Esc".to_string(), "exit_file_select_mode".to_string()),
            ].iter().cloned().collect(),
            search_mode: [
                ("Enter".to_string(), "execute_search".to_string()),
                ("Esc".to_string(), "exit_search_mode".to_string()),
            ].iter().cloned().collect(),
        }
    }
}

#[derive(PartialEq)]
enum Mode {
    Normal,
    Insert,
    Command,
    Visual,
    FileSelect,
    DirectoryNav,
    Search,
}

struct FileSelector {
    current_dir: PathBuf,
    entries: Vec<PathBuf>,
    selected_index: usize,
    parent_dir_index: Option<usize>,
}

impl FileSelector {
    fn new(path: &Path) -> io::Result<Self> {
        let current_dir = path.to_path_buf();
        let mut entries = vec![current_dir.join("..")];
        entries.extend(fs::read_dir(&current_dir)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path()));
        
        Ok(FileSelector {
            current_dir,
            entries,
            selected_index: 0,
            parent_dir_index: Some(0),
        })
    }

    fn up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    fn down(&mut self) {
        if self.selected_index < self.entries.len() - 1 {
            self.selected_index += 1;
        }
    }

    fn enter(&mut self) -> io::Result<Option<PathBuf>> {
        if self.selected_index < self.entries.len() {
            let selected = &self.entries[self.selected_index];
            if selected.is_dir() {
                self.current_dir = selected.clone();
                self.entries = vec![self.current_dir.join("..")];
                self.entries.extend(fs::read_dir(&self.current_dir)?
                    .filter_map(|entry| entry.ok())
                    .map(|entry| entry.path()));
                self.selected_index = 0;
                self.parent_dir_index = Some(0);
                Ok(None)
            } else {
                Ok(Some(selected.clone()))
            }
        } else {
            Ok(None)
        }
    }

    fn render<B: tui::backend::Backend>(&self, f: &mut Frame<B>) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Percentage(100)].as_ref())
            .split(f.size());

        let items: Vec<ListItem> = self.entries
            .iter()
            .enumerate()
            .map(|(index, path)| {
                let name = if Some(index) == self.parent_dir_index {
                    ".. (Parent Directory)".to_string()
                } else {
                    path.file_name().unwrap_or_default().to_string_lossy().into_owned()
                };
                
                if path.is_dir() {
                    ListItem::new(format!("📁 {}", name))
                } else {
                    ListItem::new(format!("📄 {}", name))
                }
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().title("File Selector").borders(Borders::ALL))
            .highlight_style(
                Style::default()
                    .bg(Color::Yellow)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            );

        let mut state = ListState::default();
        state.select(Some(self.selected_index));
        f.render_stateful_widget(list, chunks[0], &mut state);
    }
}

struct Editor {
    content: Vec<String>,
    cursor_position: (usize, usize),
    mode: Mode,
    debug_messages: Vec<String>,
    command_buffer: String,
    current_file: Option<String>,
    ps: SyntaxSet,
    ts: ThemeSet,
    syntax: String,
    cursor_style: Style,
    clipboard_context: ClipboardContext,
    visual_start: (usize, usize),
    file_selector: Option<FileSelector>,
    show_debug: bool,
    search_query: String,
    search_results: Vec<(usize, usize)>,
    current_search_index: usize,
    scroll_offset: usize,
    horizontal_scroll: usize,
    keybindings: Keybindings,
    undo_stack: VecDeque<EditOperation>,
    redo_stack: VecDeque<EditOperation>,
}

impl Editor {
    fn new() -> Self {
        let keybindings = Self::load_config().unwrap_or_else(|_| Keybindings::default());
        Editor {
            content: vec![String::new()],
            cursor_position: (0, 0),
            mode: Mode::Normal,
            debug_messages: Vec::new(),
            command_buffer: String::new(),
            current_file: None,
            ps: SyntaxSet::load_defaults_newlines(),
            ts: ThemeSet::load_defaults(),
            syntax: "Plain Text".to_string(),
            cursor_style: Style::default().fg(Color::Yellow),
            clipboard_context: ClipboardProvider::new().expect("Failed to initialize clipboard"),
            visual_start: (0, 0),
            file_selector: None,
            show_debug: false,
            search_query: String::new(),
            search_results: Vec::new(),
            current_search_index: 0,
            scroll_offset: 0,
            horizontal_scroll: 0,
            keybindings,
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
        }
    }

    fn save_state(&mut self) {
        let operation = EditOperation {
            content: self.content.clone(),
            cursor_position: self.cursor_position,
        };
        self.undo_stack.push_front(operation);
        self.redo_stack.clear();

        if self.undo_stack.len() > 100 {
            self.undo_stack.pop_back();
        }
    }

    fn undo(&mut self) {
        if let Some(operation) = self.undo_stack.pop_front() {
            let current_state = EditOperation {
                content: self.content.clone(),
                cursor_position: self.cursor_position,
            };
            self.redo_stack.push_front(current_state);

            self.content = operation.content;
            self.cursor_position = operation.cursor_position;
        }
    }

    fn redo(&mut self) {
        if let Some(operation) = self.redo_stack.pop_front() {
            let current_state = EditOperation {
                content: self.content.clone(),
                cursor_position: self.cursor_position,
            };
            self.undo_stack.push_front(current_state);

            self.content = operation.content;
            self.cursor_position = operation.cursor_position;
        }
    }

    fn load_config() -> Result<Keybindings, Box<dyn Error>> {
        let config_dir = dirs::home_dir()
            .ok_or("Could not find home directory")?
            .join(".config")
            .join("phantom");
        let config_path = config_dir.join("config.toml");
    
        if !config_path.exists() {
            Self::create_default_config(&config_path)?;
        }
    
        let config_str = fs::read_to_string(&config_path)?;
        let config: Keybindings = toml::from_str(&config_str)?;
        Ok(config)
    }
        
    fn key_event_to_string(key: event::KeyEvent) -> String {
        let mut key_str = String::new();
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            key_str.push_str("Ctrl+");
        }
        if key.modifiers.contains(KeyModifiers::ALT) {
            key_str.push_str("Alt+");
        }
        if key.modifiers.contains(KeyModifiers::SHIFT) {
            key_str.push_str("Shift+");
        }
        match key.code {
            KeyCode::Char(c) => key_str.push(c),
            KeyCode::Enter => key_str.push_str("Enter"),
            KeyCode::Left => key_str.push_str("Left"),
            KeyCode::Right => key_str.push_str("Right"),
            KeyCode::Up => key_str.push_str("Up"),
            KeyCode::Down => key_str.push_str("Down"),
            KeyCode::Home => key_str.push_str("Home"),
            KeyCode::End => key_str.push_str("End"),
            KeyCode::PageUp => key_str.push_str("PageUp"),
            KeyCode::PageDown => key_str.push_str("PageDown"),
            KeyCode::Tab => key_str.push_str("Tab"),
            KeyCode::BackTab => key_str.push_str("BackTab"),
            KeyCode::Delete => key_str.push_str("Delete"),
            KeyCode::Insert => key_str.push_str("Insert"),
            KeyCode::F(n) => key_str.push_str(&format!("F{}", n)),
            KeyCode::Esc => key_str.push_str("Esc"),
            _ => {},
        }
        key_str
    }

    fn create_default_config(config_path: &PathBuf) -> Result<(), Box<dyn Error>> {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
    
        let default_config = toml::to_string_pretty(&Keybindings::default())?;
        fs::write(config_path, default_config)?;
        Ok(())
    }

    fn with_file(path: &Path) -> Result<Self, io::Error> {
        let mut editor = Self::new();
        editor.open_file(path)?;
        Ok(editor)
    }
    
    fn run(&mut self) -> Result<(), Box<dyn Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let res = self.run_app(&mut terminal);

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        if let Err(err) = res {
            println!("{:?}", err)
        }

        Ok(())
    }

    fn run_app<B: tui::backend::Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = event::read()? {
                self.debug_messages.push(format!("Key pressed: {:?}", key));
                self.debug_messages.push(format!("Cursor: ({}, {})", self.cursor_position.0, self.cursor_position.1));
                
                while self.debug_messages.len() > 5 {
                    self.debug_messages.remove(0);
                }

                match key.code {
                    KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(());
                    }
                    _ => {
                        if self.handle_key_event(key)? {
                            return Ok(());
                        }
                    }
                }
            }
        }
    }

    fn handle_key_event(&mut self, key: event::KeyEvent) -> io::Result<bool> {
        match self.mode {
            Mode::Normal => self.handle_normal_mode(key),
            Mode::Insert => self.handle_insert_mode(key),
            Mode::Command => {
                let result = self.handle_command_mode(key)?;
                if result {
                    return self.execute_command();
                }
                Ok(false)
            },
            Mode::Visual => self.handle_visual_mode(key),
            Mode::FileSelect | Mode::DirectoryNav => self.handle_file_select_mode(key),
            Mode::Search => self.handle_search_mode(key),
        }
    }

    fn handle_normal_mode(&mut self, key: event::KeyEvent) -> io::Result<bool> {
        let key_str = Self::key_event_to_string(key);
        if let Some(action) = self.keybindings.normal_mode.get(&key_str) {
            match action.as_str() {
                "enter_insert_mode" => self.mode = Mode::Insert,
                "append" => {
                    self.mode = Mode::Insert;
                    self.move_cursor_right();
                },
                "open_line_below" => {
                    self.insert_line_below();
                    self.mode = Mode::Insert;
                },
                "open_line_above" => {
                    self.insert_line_above();
                    self.mode = Mode::Insert;
                },
                "delete_line" => self.delete_line(),
                "yank_line" => self.yank_line(),
                "paste_after" => self.paste_after(),
                "enter_visual_mode" => {
                    self.mode = Mode::Visual;
                    self.visual_start = self.cursor_position;
                },
                "enter_command_mode" => {
                    self.mode = Mode::Command;
                    self.command_buffer.clear();
                },
                "toggle_debug_menu" => self.toggle_debug_menu(),
                "enter_directory_nav_mode" => self.enter_directory_nav_mode()?,
                "enter_search_mode" => self.enter_search_mode(),
                "next_search_result" => self.next_search_result(),
                "previous_search_result" => self.previous_search_result(),
                "copy_selection" => self.copy_selection(),
                "paste_clipboard" => self.paste_clipboard(),
                "undo" => self.undo(),
                "redo" => self.redo(),
                _ => {},
            }
        } else {
            // Handle default movements
            match key.code {
                KeyCode::Left => self.move_cursor_left(),
                KeyCode::Down => self.move_cursor_down(),
                KeyCode::Up => self.move_cursor_up(),
                KeyCode::Right => self.move_cursor_right(),
                KeyCode::Home => self.move_cursor_start_of_line(),
                KeyCode::End => self.move_cursor_end_of_line(),
                KeyCode::PageUp => self.page_up(),
                KeyCode::PageDown => self.page_down(),
                _ => {},
            }
        }
        Ok(false)
    }

    fn handle_insert_mode(&mut self, key: event::KeyEvent) -> io::Result<bool> {
        match key.code {
            KeyCode::Esc | KeyCode::Insert => self.mode = Mode::Normal,
            KeyCode::Enter => self.insert_newline(),
            KeyCode::Backspace => self.backspace(),
            KeyCode::Left => self.move_cursor_left(),
            KeyCode::Down => self.move_cursor_down(),
            KeyCode::Up => self.move_cursor_up(),
            KeyCode::Right => self.move_cursor_right(),
            KeyCode::Char(c) => self.insert_char(c),
            _ => {}
        }
        Ok(false)
    }

    fn handle_command_mode(&mut self, key: event::KeyEvent) -> io::Result<bool> {
        match key.code {
            KeyCode::Enter => return Ok(true),
            KeyCode::Char(c) => self.command_buffer.push(c),
            KeyCode::Backspace => { self.command_buffer.pop(); }
            KeyCode::Esc => self.mode = Mode::Normal,
            _ => {}
        }
        Ok(false)
    }

    fn handle_visual_mode(&mut self, key: event::KeyEvent) -> io::Result<bool> {
        match key.code {
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Left => self.move_cursor_left(),
            KeyCode::Down => self.move_cursor_down(),
            KeyCode::Up => self.move_cursor_up(),
            KeyCode::Right => self.move_cursor_right(),
            KeyCode::Char('y') => {
                self.copy_selection();
                self.mode = Mode::Normal;
            }
            KeyCode::Char('d') => {
                self.delete_selection();
                self.mode = Mode::Normal;
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_file_select_mode(&mut self, key: event::KeyEvent) -> io::Result<bool> {
        if let Some(file_selector) = &mut self.file_selector {
            match key.code {
                KeyCode::Up => file_selector.up(),
                KeyCode::Down => file_selector.down(),
                KeyCode::Enter => {
                    if let Some(path) = file_selector.enter()? {
                        self.open_file(&path)?;
                        self.mode = Mode::Normal;
                        self.file_selector = None;
                    }
                }
                KeyCode::Esc => {
                    self.mode = Mode::Normal;
                    self.file_selector = None;
                }
                _ => {}
            }
        }
        Ok(false)
    }

    fn execute_command(&mut self) -> io::Result<bool> {
        let command = self.command_buffer.clone();
        self.mode = Mode::Normal;
        self.command_buffer.clear();

        match command.as_str() {
            "q" => return Ok(true),
            "w" => self.save_file(None)?,
            cmd if cmd.starts_with("w ") => {
                let filename = cmd.split_whitespace().nth(1).unwrap();
                self.save_file(Some(Path::new(filename)))?;
            }
            "wq" => {
                self.save_file(None)?;
                return Ok(true);
            }
            cmd if cmd.starts_with("e ") => {
                let filename = cmd.split_whitespace().nth(1).unwrap();
                self.open_file(Path::new(filename))?;
            }
            _ => self.debug_messages.push(format!("Unknown command: {}", command)),
        }
        Ok(false)
    }

    fn move_cursor_up(&mut self) {
        if self.cursor_position.1 > 0 {
            self.cursor_position.1 -= 1;
            if self.cursor_position.1 < self.scroll_offset {
                self.scroll_offset = self.cursor_position.1;
            }
        }
    }
    
    fn move_cursor_down(&mut self) {
        if self.cursor_position.1 < self.content.len() - 1 {
            self.cursor_position.1 += 1;
            let editor_height = self.get_editor_height();
            if self.cursor_position.1 >= self.scroll_offset + editor_height {
                self.scroll_offset = self.cursor_position.1 - editor_height + 1;
            }
        }
    }

    fn move_cursor_left(&mut self) {
        if self.cursor_position.0 > 0 {
            self.cursor_position.0 -= 1;
            if self.cursor_position.0 < self.horizontal_scroll {
                self.horizontal_scroll = self.cursor_position.0;
            }
        } else if self.cursor_position.1 > 0 {
            self.cursor_position.1 -= 1;
            self.cursor_position.0 = self.content[self.cursor_position.1].len();
            self.adjust_horizontal_scroll();
        }
    }

    fn move_cursor_right(&mut self) {
        if self.cursor_position.0 < self.content[self.cursor_position.1].len() {
            self.cursor_position.0 += 1;
            self.adjust_horizontal_scroll();
        } else if self.cursor_position.1 < self.content.len() - 1 {
            self.cursor_position.1 += 1;
            self.cursor_position.0 = 0;
            self.horizontal_scroll = 0;
        }
    }
    
    fn get_editor_height(&self) -> usize {
        24
    }

    fn move_cursor_start_of_line(&mut self) {
        self.cursor_position.0 = 0;
    }

    fn move_cursor_end_of_line(&mut self) {
        self.cursor_position.0 = self.content[self.cursor_position.1].len();
    }

    fn insert_char(&mut self, c: char) {
        self.save_state();
        let line = &mut self.content[self.cursor_position.1];
        line.insert(self.cursor_position.0, c);
        self.cursor_position.0 += 1;
        self.adjust_horizontal_scroll();
    }

    fn insert_newline(&mut self) {
        self.save_state();
        let current_line = &mut self.content[self.cursor_position.1];
        let rest_of_line = current_line.split_off(self.cursor_position.0);
        self.content.insert(self.cursor_position.1 + 1, rest_of_line);
        self.cursor_position = (0, self.cursor_position.1 + 1);
    }

    fn page_up(&mut self) {
        let editor_height = self.get_editor_height();
        if self.scroll_offset > editor_height {
            self.scroll_offset -= editor_height;
        } else {
            self.scroll_offset = 0;
        }
        self.cursor_position.1 = self.scroll_offset;
    }
    
    fn page_down(&mut self) {
        let editor_height = self.get_editor_height();
        let max_scroll = self.content.len().saturating_sub(editor_height);
        if self.scroll_offset + editor_height < max_scroll {
            self.scroll_offset += editor_height;
        } else {
            self.scroll_offset = max_scroll;
        }
        self.cursor_position.1 = self.scroll_offset + editor_height - 1;
        if self.cursor_position.1 >= self.content.len() {
            self.cursor_position.1 = self.content.len() - 1;
        }
    }

    fn backspace(&mut self) {
        self.save_state();
        if self.cursor_position.0 > 0 {
            let line = &mut self.content[self.cursor_position.1];
            line.remove(self.cursor_position.0 - 1);
            self.cursor_position.0 -= 1;
        } else if self.cursor_position.1 > 0 {
            let current_line = self.content.remove(self.cursor_position.1);
            self.cursor_position.1 -= 1;
            self.cursor_position.0 = self.content[self.cursor_position.1].len();
            self.content[self.cursor_position.1].push_str(&current_line);
        }
    }

    fn delete_char(&mut self) {
        self.save_state();
        let line = &mut self.content[self.cursor_position.1];
        if self.cursor_position.0 < line.len() {
            line.remove(self.cursor_position.0);
        } else if self.cursor_position.1 < self.content.len() - 1 {
            let next_line = self.content.remove(self.cursor_position.1 + 1);
            self.content[self.cursor_position.1].push_str(&next_line);
        }
    }

    fn delete_line(&mut self) {
        self.save_state();
        if self.content.len() > 1 {
            self.content.remove(self.cursor_position.1);
        } else {
            self.content[0].clear();
        }
        if self.cursor_position.1 >= self.content.len() {
            self.cursor_position.1 = self.content.len() - 1;
        }
        self.cursor_position.0 = 0;
    }

    fn insert_line_below(&mut self) {
        self.save_state();
        self.content.insert(self.cursor_position.1 + 1, String::new());
        self.cursor_position = (0, self.cursor_position.1 + 1);
    }

    fn insert_line_above(&mut self) {
        self.save_state();
        self.content.insert(self.cursor_position.1, String::new());
        self.cursor_position.0 = 0;
    }

    fn yank_line(&mut self) {
        let line = &self.content[self.cursor_position.1];
        self.clipboard_context.set_contents(line.to_string()).unwrap();
    }

    fn paste_after(&mut self) {
        self.save_state();
        if let Ok(contents) = self.clipboard_context.get_contents() {
            let lines: Vec<&str> = contents.split('\n').collect();
            if lines.len() == 1 {
                let line = &mut self.content[self.cursor_position.1];
                line.insert_str(self.cursor_position.0, &contents);
                self.cursor_position.0 += contents.len();
            } else {
                for (i, &line) in lines.iter().enumerate() {
                    if i == 0 {
                        let current_line = &mut self.content[self.cursor_position.1];
                        let rest_of_line = current_line.split_off(self.cursor_position.0);
                        current_line.push_str(line);
                        self.content.insert(self.cursor_position.1 + 1, rest_of_line);
                    } else if i == lines.len() - 1 {
                        self.content[self.cursor_position.1 + i].insert_str(0, line);
                    } else {
                        self.content.insert(self.cursor_position.1 + i, line.to_string());
                    }
                }
                self.cursor_position = (lines.last().unwrap().len(), self.cursor_position.1 + lines.len() - 1);
            }
        }
    }

    fn copy_selection(&mut self) {
        let (start, end) = if self.visual_start <= self.cursor_position {
            (self.visual_start, self.cursor_position)
        } else {
            (self.cursor_position, self.visual_start)
        };

        let mut selected_text = String::new();
        for (i, line) in self.content.iter().enumerate().skip(start.1).take(end.1 - start.1 + 1) {
            if i == start.1 && i == end.1 {
                selected_text.push_str(&line[start.0..=end.0]);
            } else if i == start.1 {
                selected_text.push_str(&line[start.0..]);
                selected_text.push('\n');
            } else if i == end.1 {
                selected_text.push_str(&line[..=end.0]);
            } else {
                selected_text.push_str(line);
                selected_text.push('\n');
            }
        }

        self.clipboard_context.set_contents(selected_text).unwrap();
    }

    fn delete_selection(&mut self) {
        self.save_state();
        let (start, end) = if self.visual_start <= self.cursor_position {
            (self.visual_start, self.cursor_position)
        } else {
            (self.cursor_position, self.visual_start)
        };

        if start.1 == end.1 {
            let line = &mut self.content[start.1];
            line.replace_range(start.0..=end.0, "");
        } else {
            let mut new_line = self.content[start.1][..start.0].to_string();
            new_line.push_str(&self.content[end.1][end.0 + 1..]);
            self.content.drain(start.1..=end.1);
            self.content.insert(start.1, new_line);
        }

        self.cursor_position = start;
    }

    fn paste_clipboard(&mut self) {
        self.save_state();
        if let Ok(contents) = self.clipboard_context.get_contents() {
            let lines: Vec<&str> = contents.split('\n').collect();
            for (i, &line) in lines.iter().enumerate() {
                if i == 0 {
                    let current_line = &mut self.content[self.cursor_position.1];
                    current_line.insert_str(self.cursor_position.0, line);
                    self.cursor_position.0 += line.len();
                } else {
                    self.cursor_position.1 += 1;
                    self.content.insert(self.cursor_position.1, line.to_string());
                    self.cursor_position.0 = line.len();
                }
            }
        }
    }

    fn save_file(&mut self, filename: Option<&Path>) -> io::Result<()> {
        let filename = if let Some(name) = filename {
            name.to_path_buf()
        } else if let Some(ref name) = self.current_file {
            PathBuf::from(name)
        } else {
            return Err(io::Error::new(io::ErrorKind::Other, "No filename specified. Use :w <filename> to save."));
        };

        let mut file = fs::File::create(&filename)?;
        for line in &self.content {
            writeln!(file, "{}", line)?;
        }
        self.current_file = Some(filename.to_string_lossy().into_owned());
        self.debug_messages.push(format!("File saved: {}", filename.display()));
        Ok(())
    }

    fn open_file(&mut self, path: &Path) -> io::Result<()> {
        let content = fs::read_to_string(path)?;
        self.content = if content.is_empty() {
            vec![String::new()]
        } else {
            content.lines().map(String::from).collect()
        };
        self.cursor_position = (0, 0);
        self.current_file = Some(path.to_string_lossy().into_owned());
        
        if let Some(extension) = path.extension() {
            if let Some(ext_str) = extension.to_str() {
                if let Some(syntax) = self.ps.find_syntax_by_extension(ext_str) {
                    self.syntax = syntax.name.clone();
                }
            }
        }
        
        Ok(())
    }

    fn toggle_debug_menu(&mut self) {
        self.show_debug = !self.show_debug;
        self.debug_messages.push(if self.show_debug {
            "Debug menu shown".to_string()
        } else {
            "Debug menu hidden".to_string()
        });
    }

    fn enter_directory_nav_mode(&mut self) -> io::Result<()> {
        let current_dir = if let Some(ref file) = self.current_file {
            Path::new(file).parent().unwrap_or(Path::new(".")).to_path_buf()
        } else {
            env::current_dir()?
        };
        self.file_selector = Some(FileSelector::new(&current_dir)?);
        self.mode = Mode::DirectoryNav;
        Ok(())
    }

    fn ui<B: tui::backend::Backend>(&mut self, f: &mut Frame<B>) {
        if self.mode == Mode::FileSelect || self.mode == Mode::DirectoryNav {
            if let Some(file_selector) = &self.file_selector {
                file_selector.render(f);
            }
            return;
        }
    
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(
                if self.show_debug {
                    vec![
                        Constraint::Length(6),
                        Constraint::Min(1),
                        Constraint::Length(1)
                    ]
                } else {
                    vec![
                        Constraint::Min(1),
                        Constraint::Length(1)
                    ]
                }
            )
            .split(f.size());
    
        let mode_indicator = match self.mode {
            Mode::Normal => "NORMAL",
            Mode::Insert => "INSERT",
            Mode::Command => "COMMAND",
            Mode::Visual => "VISUAL",
            Mode::FileSelect => "FILE SELECT",
            Mode::DirectoryNav => "DIRECTORY NAV",
            Mode::Search => "SEARCH",
        };
    
        let block = Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(
                format!("Editor - {}", mode_indicator),
                Style::default().add_modifier(Modifier::BOLD),
            ));
    
        let syntax = self.ps.find_syntax_by_extension("rs")
            .or_else(|| self.ps.find_syntax_by_name(&self.syntax))
            .unwrap_or_else(|| self.ps.find_syntax_plain_text());
        let mut h = HighlightLines::new(syntax, &self.ts.themes["base16-ocean.dark"]);
    
        let editor_chunk_index = if self.show_debug { 1 } else { 0 };
        let editor_height = chunks[editor_chunk_index].height as usize - 2;
        let content_height = self.content.len();
        let editor_width = self.get_editor_width();

        let visible_content = self.content.iter()
            .skip(self.scroll_offset)
            .take(editor_height)
            .enumerate();


        if self.cursor_position.1 < self.scroll_offset {
            self.scroll_offset = self.cursor_position.1;
        } else if self.cursor_position.1 >= self.scroll_offset + editor_height {
            self.scroll_offset = self.cursor_position.1 - editor_height + 1;
        }
    
        let visible_content = self.content.iter()
            .skip(self.scroll_offset)
            .take(editor_height)
            .enumerate();
    
        let mut text = Vec::new();
        for (index, line) in visible_content {
            let ranges: Vec<(SyntectStyle, &str)> = h.highlight_line(line, &self.ps).unwrap();
            let mut styled_spans = Vec::new();
            let mut line_length = 0;
            for (style, content) in ranges {
                let color = style.foreground;
                let visible_content = if line_length >= self.horizontal_scroll {
                    content
                } else if line_length + content.len() > self.horizontal_scroll {
                    &content[self.horizontal_scroll - line_length..]
                } else {
                    ""
                };
                line_length += content.len();
                if !visible_content.is_empty() {
                    styled_spans.push(Span::styled(
                        visible_content.to_string(),
                        Style::default().fg(Color::Rgb(color.r, color.g, color.b))
                    ));
                }
                if line_length >= self.horizontal_scroll + editor_width {
                    break;
                }
            }
            
            if index + self.scroll_offset == self.cursor_position.1 {
                let mut line_spans = Vec::new();
                let mut current_len = 0;
                for span in styled_spans {
                    let span_len = span.content.len();
                    if current_len <= self.cursor_position.0 - self.horizontal_scroll && self.cursor_position.0 - self.horizontal_scroll < current_len + span_len {
                        let (before, after) = span.content.split_at(self.cursor_position.0 - self.horizontal_scroll - current_len);
                        if !before.is_empty() {
                            line_spans.push(Span::styled(before.to_string(), span.style));
                        }
                        line_spans.push(Span::styled("".to_string(), self.cursor_style));
                        if !after.is_empty() {
                            line_spans.push(Span::styled(after.to_string(), span.style));
                        }
                    } else {
                        line_spans.push(span);
                    }
                    current_len += span_len;
                }
                if self.cursor_position.0 - self.horizontal_scroll >= current_len {
                    line_spans.push(Span::styled("".to_string(), self.cursor_style));
                }
                text.push(Spans::from(line_spans));
            } else {
                text.push(Spans::from(styled_spans));
            }
        }
    
        let paragraph = Paragraph::new(text).block(block);
        f.render_widget(paragraph, chunks[editor_chunk_index]);
    
        if self.show_debug {
            let debug_messages: Vec<Spans> = self.debug_messages.iter().map(|m| Spans::from(m.clone())).collect();
            let debug_paragraph = Paragraph::new(debug_messages)
                .block(Block::default().borders(Borders::ALL).title("Debug Output"));
            f.render_widget(debug_paragraph, chunks[0]);
        }
    
        if self.mode == Mode::Command {
            let command_text = Spans::from(format!(":{}", self.command_buffer));
            let command_paragraph = Paragraph::new(vec![command_text]);
            f.render_widget(command_paragraph, chunks[chunks.len() - 1]);
        } else if self.mode == Mode::Search {
            let search_text = Spans::from(format!("Search: {}", self.search_query));
            let search_paragraph = Paragraph::new(vec![search_text]);
            f.render_widget(search_paragraph, chunks[chunks.len() - 1]);
        }
    
        let cursor_x = (self.cursor_position.0 - self.horizontal_scroll) as u16 + 2;
        let cursor_y = (self.cursor_position.1 - self.scroll_offset) as u16 + if self.show_debug { 8 } else { 2 };
        f.set_cursor(
            cursor_x.min(chunks[editor_chunk_index].width - 1),
            cursor_y.min(chunks[editor_chunk_index].height - 1),
        )
    }

    fn enter_search_mode(&mut self) {
        self.mode = Mode::Search;
        self.search_query.clear();
        self.search_results.clear();
        self.current_search_index = 0;
    }

    fn perform_search(&mut self) {
        self.search_results.clear();
        for (line_num, line) in self.content.iter().enumerate() {
            if let Some(col) = line.to_lowercase().find(&self.search_query.to_lowercase()) {
                self.search_results.push((line_num, col));
            }
        }
        self.current_search_index = 0;
        if !self.search_results.is_empty() {
            let (line, col) = self.search_results[0];
            self.cursor_position = (col, line);
        }
    }

    fn next_search_result(&mut self) {
        if !self.search_results.is_empty() {
            self.current_search_index = (self.current_search_index + 1) % self.search_results.len();
            let (line, col) = self.search_results[self.current_search_index];
            self.cursor_position = (col, line);
        }
    }

    fn previous_search_result(&mut self) {
        if !self.search_results.is_empty() {
            self.current_search_index = (self.current_search_index + self.search_results.len() - 1) % self.search_results.len();
            let (line, col) = self.search_results[self.current_search_index];
            self.cursor_position = (col, line);
        }
    }

    fn handle_search_mode(&mut self, key: event::KeyEvent) -> io::Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
            KeyCode::Enter => {
                self.perform_search();
                self.mode = Mode::Normal;
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
            }
            KeyCode::Backspace => {
                self.search_query.pop();
            }
            _ => {}
        }
        Ok(false)
    }

    fn adjust_horizontal_scroll(&mut self) {
        let editor_width = self.get_editor_width();
        if self.cursor_position.0 < self.horizontal_scroll {
            self.horizontal_scroll = self.cursor_position.0;
        } else if self.cursor_position.0 >= self.horizontal_scroll + editor_width {
            self.horizontal_scroll = self.cursor_position.0 - editor_width + 1;
        }
    }

    fn get_editor_width(&self) -> usize {
        80
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let mut editor = if args.len() > 1 {
        let path = Path::new(&args[1]);
        if path.is_dir() {
            let mut editor = Editor::new();
            editor.mode = Mode::FileSelect;
            editor.file_selector = Some(FileSelector::new(path)?);
            editor
        } else {
            match Editor::with_file(path) {
                Ok(ed) => ed,
                Err(e) => {
                    eprintln!("Error opening file: {}", e);
                    return Ok(());
                }
            }
        }
    } else {
        Editor::new()
    };

    if let Err(err) = editor.run() {
        eprintln!("Error: {:?}", err);
    }
    Ok(())
}