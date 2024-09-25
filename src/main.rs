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

#[derive(PartialEq)]
enum Mode {
    Normal,
    Insert,
    Command,
    Visual,
    FileSelect,
    DirectoryNav,
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
}

impl Editor {
    fn new() -> Self {
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
        }
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
        }
    }

    fn handle_normal_mode(&mut self, key: event::KeyEvent) -> io::Result<bool> {
        match key.code {
            KeyCode::Char('q') if key.modifiers == KeyModifiers::CONTROL => return Ok(true),
            KeyCode::Char('i') => self.mode = Mode::Insert,
            KeyCode::Insert => self.mode = Mode::Insert,
            KeyCode::Char('a') => {
                self.mode = Mode::Insert;
                self.move_cursor_right();
            }
            KeyCode::Char('o') => {
                self.insert_line_below();
                self.mode = Mode::Insert;
            }
            KeyCode::Char('O') => {
                self.insert_line_above();
                self.mode = Mode::Insert;
            }
            KeyCode::Char('d') => {
                if let Event::Key(next_key) = event::read()? {
                    if next_key.code == KeyCode::Char('d') {
                        self.delete_line();
                    }
                }
            }
            KeyCode::Char('y') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.copy_selection();
                } else {
                    if let Event::Key(next_key) = event::read()? {
                        if next_key.code == KeyCode::Char('y') {
                            self.yank_line();
                        }
                    }
                }
            }
            KeyCode::Char('p') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.paste_clipboard();
                } else {
                    self.paste_after();
                }
            }
            KeyCode::Char('v') => {
                self.mode = Mode::Visual;
                self.visual_start = self.cursor_position;
            }
            KeyCode::Left => self.move_cursor_left(),
            KeyCode::Down => self.move_cursor_down(),
            KeyCode::Up => self.move_cursor_up(),
            KeyCode::Right => self.move_cursor_right(),
            KeyCode::Home => self.move_cursor_start_of_line(),
            KeyCode::End => self.move_cursor_end_of_line(),
            KeyCode::Delete => self.delete_char(),
            KeyCode::Char(':') => {
                self.mode = Mode::Command;
                self.command_buffer.clear();
            }
            KeyCode::Char('b') if key.modifiers == KeyModifiers::CONTROL => {
                self.toggle_debug_menu();
            }
            KeyCode::Char('e') if key.modifiers == KeyModifiers::CONTROL => {
                self.enter_directory_nav_mode()?;
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_insert_mode(&mut self, key: event::KeyEvent) -> io::Result<bool> {
        match key.code {
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Enter => self.insert_newline(),
            KeyCode::Backspace => self.backspace(),
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
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.command_buffer.clear();
            }
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

    fn move_cursor_left(&mut self) {
        let (x, y) = self.cursor_position;
        if x > 0 {
            self.cursor_position = (x - 1, y);
        }
    }

    fn move_cursor_right(&mut self) {
        let (x, y) = self.cursor_position;
        if x < self.content[y].len() {
            self.cursor_position = (x + 1, y);
        }
    }

    fn move_cursor_up(&mut self) {
        let (x, y) = self.cursor_position;
        if y > 0 {
            self.cursor_position = (x.min(self.content[y - 1].len()), y - 1);
        }
    }

    fn move_cursor_down(&mut self) {
        let (x, y) = self.cursor_position;
        if y < self.content.len() - 1 {
            self.cursor_position = (x.min(self.content[y + 1].len()), y + 1);
        }
    }

    fn move_cursor_start_of_line(&mut self) {
        self.cursor_position.0 = 0;
    }

    fn move_cursor_end_of_line(&mut self) {
        let y = self.cursor_position.1;
        self.cursor_position.0 = self.content[y].len();
    }

    fn insert_newline(&mut self) {
        let (x, y) = self.cursor_position;
        let current_line = self.content[y].clone();
        let (left, right) = current_line.split_at(x);
        self.content[y] = left.to_string();
        self.content.insert(y + 1, right.to_string());
        self.cursor_position = (0, y + 1);
    }

    fn insert_char(&mut self, c: char) {
        let (x, y) = self.cursor_position;
        self.content[y].insert(x, c);
        self.cursor_position.0 += 1;
    }

    fn backspace(&mut self) {
        let (x, y) = self.cursor_position;
        if x > 0 {
            self.content[y].remove(x - 1);
            self.cursor_position.0 -= 1;
        } else if y > 0 {
            let current_line = self.content.remove(y);
            let previous_line_len = self.content[y - 1].len();
            self.content[y - 1].push_str(&current_line);
            self.cursor_position = (previous_line_len, y - 1);
        }
    }

    fn delete_char(&mut self) {
        let (x, y) = self.cursor_position;
        if x < self.content[y].len() {
            self.content[y].remove(x);
        } else if y < self.content.len() - 1 {
            let next_line = self.content.remove(y + 1);
            self.content[y].push_str(&next_line);
        }
    }

    fn delete_line(&mut self) {
        let y = self.cursor_position.1;
        if self.content.len() > 1 {
            self.content.remove(y);
        } else {
            self.content[0].clear();
        }
        if y >= self.content.len() {
            self.cursor_position.1 = self.content.len() - 1;
        }
        self.cursor_position.0 = 0;
    }

    fn yank_line(&mut self) {
        let y = self.cursor_position.1;
        let line = self.content[y].clone();
        if let Err(e) = self.clipboard_context.set_contents(line) {
            self.debug_messages.push(format!("Failed to copy to clipboard: {}", e));
        } else {
            self.debug_messages.push("Line copied to clipboard".to_string());
        }
    }

    fn paste_after(&mut self) {
        if let Ok(content) = self.clipboard_context.get_contents() {
            let (_, y) = self.cursor_position;
            self.content.insert(y + 1, content);
            self.cursor_position = (0, y + 1);
            self.debug_messages.push("Clipboard content pasted".to_string());
        } else {
            self.debug_messages.push("Failed to paste from clipboard".to_string());
        }
    }

    fn insert_line_below(&mut self) {
        let y = self.cursor_position.1;
        self.content.insert(y + 1, String::new());
        self.cursor_position = (0, y + 1);
    }

    fn insert_line_above(&mut self) {
        let y = self.cursor_position.1;
        self.content.insert(y, String::new());
        self.cursor_position = (0, y);
    }

    fn copy_selection(&mut self) {
        let (start, end) = if self.visual_start.1 <= self.cursor_position.1 {
            (self.visual_start, self.cursor_position)
        } else {
            (self.cursor_position, self.visual_start)
        };
        
        let content = self.content[start.1..=end.1].join("\n");
        if let Err(e) = self.clipboard_context.set_contents(content) {
            self.debug_messages.push(format!("Failed to copy to clipboard: {}", e));
        } else {
            self.debug_messages.push(format!("{} lines copied to clipboard", end.1 - start.1 + 1));
        }
    }

    fn paste_clipboard(&mut self) {
        if let Ok(content) = self.clipboard_context.get_contents() {
            let (_, y) = self.cursor_position;
            let new_lines: Vec<String> = content.lines().map(String::from).collect();
            self.content.splice(y+1..y+1, new_lines);
            self.cursor_position = (0, y + 1);
            self.debug_messages.push("Clipboard content pasted".to_string());
        } else {
            self.debug_messages.push("Failed to paste from clipboard".to_string());
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

    fn ui<B: tui::backend::Backend>(&self, f: &mut Frame<B>) {
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

        let mut text = Vec::new();
        for (index, line) in self.content.iter().enumerate() {
            let ranges: Vec<(SyntectStyle, &str)> = h.highlight_line(line, &self.ps).unwrap();
            let mut styled_spans = Vec::new();
            for (style, content) in ranges {
                let color = style.foreground;
                styled_spans.push(Span::styled(
                    content.to_string(),
                    Style::default().fg(Color::Rgb(color.r, color.g, color.b))
                ));
            }
            
            if index == self.cursor_position.1 {
                let mut line_spans = Vec::new();
                let mut current_len = 0;
                for span in styled_spans {
                    let span_len = span.content.len();
                    if current_len <= self.cursor_position.0 && self.cursor_position.0 < current_len + span_len {
                        let (before, after) = span.content.split_at(self.cursor_position.0 - current_len);
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
                if self.cursor_position.0 >= current_len {
                    line_spans.push(Span::styled("".to_string(), self.cursor_style));
                }
                text.push(Spans::from(line_spans));
            } else {
                text.push(Spans::from(styled_spans));
            }
        }

        let editor_chunk_index = if self.show_debug { 1 } else { 0 };
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
        }

        let cursor_x = self.cursor_position.0 as u16 + 2;
        let cursor_y = self.cursor_position.1 as u16 + if self.show_debug { 8 } else { 2 };
        f.set_cursor(
            cursor_x.min(chunks[editor_chunk_index].width - 1),
            cursor_y.min(chunks[editor_chunk_index].height - 1),
        )
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