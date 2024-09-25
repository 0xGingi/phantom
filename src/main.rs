use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{error::Error, io};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::env;
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use unicode_width::UnicodeWidthStr;

#[derive(PartialEq)]
enum Mode {
    Normal,
    Insert,
    Command,
}

struct Editor {
    content: Vec<String>,
    cursor_position: (usize, usize),
    mode: Mode,
    clipboard: String,
    debug_messages: Vec<String>,
    command_buffer: String,
    current_file: Option<String>,
}

impl Editor {
    fn new() -> Self {
        Editor {
            content: vec![String::new()],
            cursor_position: (0, 0),
            mode: Mode::Normal,
            clipboard: String::new(),
            debug_messages: Vec::new(),
            command_buffer: String::new(),
            current_file: None,
        }
    }

    fn with_file(filename: &str) -> Result<Self, io::Error> {
        let mut editor = Self::new();
        editor.open_file(filename)?;
        editor.current_file = Some(filename.to_string());
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
                if let Event::Key(next_key) = event::read()? {
                    if next_key.code == KeyCode::Char('y') {
                        self.yank_line();
                    }
                }
            }
            KeyCode::Char('p') => self.paste_after(),
            KeyCode::Char('P') => self.paste_before(),
            KeyCode::Char('u') => self.undo(),
            KeyCode::Char('r') => self.redo(),
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
            KeyCode::Left => self.move_cursor_left(),
            KeyCode::Down => self.move_cursor_down(),
            KeyCode::Up => self.move_cursor_up(),
            KeyCode::Right => self.move_cursor_right(),
            KeyCode::Home => self.move_cursor_start_of_line(),
            KeyCode::End => self.move_cursor_end_of_line(),
            _ => {}
        }
        Ok(false)
    }

    fn handle_command_mode(&mut self, key: event::KeyEvent) -> io::Result<bool> {
        match key.code {
            KeyCode::Enter => {
                self.mode = Mode::Normal;
                Ok(true)
            }
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.command_buffer.clear();
                Ok(false)
            }
            KeyCode::Char(c) => {
                self.command_buffer.push(c);
                Ok(false)
            }
            KeyCode::Backspace => {
                self.command_buffer.pop();
                Ok(false)
            }
            _ => Ok(false)
        }
    }

    fn execute_command(&mut self) -> io::Result<bool> {
        let command = self.command_buffer.clone();
        let command = command.trim();
        
        match command {
            "w" => self.save_file(None)?,
            "q" => return Ok(true),
            "wq" => {
                self.save_file(None)?;
                return Ok(true);
            },
            _ if command.starts_with("w ") => {
                let filename = command.get(2..).map(|s| s.trim());
                self.save_file(filename)?;
            },
            _ if command.starts_with("e ") => {
                let filename = &command[2..];
                match self.open_file(filename) {
                    Ok(_) => {
                        self.current_file = Some(filename.to_string());
                        self.debug_messages.push(format!("File opened: {}", filename));
                    },
                    Err(e) => self.debug_messages.push(format!("Error opening file: {}", e)),
                }
            },
            _ => {
                self.debug_messages.push(format!("Unknown command: {}", command));
            }
        }
        self.command_buffer.clear();
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
        let (_, y) = self.cursor_position;
        self.cursor_position = (0, y);
    }

    fn move_cursor_end_of_line(&mut self) {
        let (_, y) = self.cursor_position;
        self.cursor_position = (self.content[y].len(), y);
    }

    fn insert_char(&mut self, c: char) {
        let (x, y) = self.cursor_position;
        self.content[y].insert(x, c);
        self.cursor_position = (x + 1, y);
    }

    fn insert_newline(&mut self) {
        let (x, y) = self.cursor_position;
        let current_line = self.content[y].split_off(x);
        self.content.insert(y + 1, current_line);
        self.cursor_position = (0, y + 1);
    }

    fn backspace(&mut self) {
        let (x, y) = self.cursor_position;
        if x > 0 {
            self.content[y].remove(x - 1);
            self.cursor_position = (x - 1, y);
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

    fn insert_line_below(&mut self) {
        let (_, y) = self.cursor_position;
        self.content.insert(y + 1, String::new());
        self.cursor_position = (0, y + 1);
    }

    fn insert_line_above(&mut self) {
        let (_, y) = self.cursor_position;
        self.content.insert(y, String::new());
        self.cursor_position = (0, y);
    }

    fn delete_line(&mut self) {
        let (_, y) = self.cursor_position;
        if self.content.len() > 1 {
            self.clipboard = self.content.remove(y);
            if y == self.content.len() {
                self.cursor_position = (0, y - 1);
            }
        } else {
            self.clipboard = self.content[0].clone();
            self.content[0].clear();
        }
    }

    fn yank_line(&mut self) {
        let (_, y) = self.cursor_position;
        self.clipboard = self.content[y].clone();
    }

    fn paste_after(&mut self) {
        let (_, y) = self.cursor_position;
        self.content.insert(y + 1, self.clipboard.clone());
        self.cursor_position = (0, y + 1);
    }

    fn paste_before(&mut self) {
        let (_, y) = self.cursor_position;
        self.content.insert(y, self.clipboard.clone());
    }

    fn undo(&mut self) {
    }

    fn redo(&mut self) {
    }

    fn save_file(&mut self, filename: Option<&str>) -> io::Result<()> {
        let filename = if let Some(name) = filename {
            name.to_string()
        } else if let Some(ref name) = self.current_file {
            name.clone()
        } else {
            self.debug_messages.push("Enter filename to save:".to_string());
            return Ok(());
        };

        let mut file = fs::File::create(&filename)?;
        for line in &self.content {
            writeln!(file, "{}", line)?;
        }
        self.current_file = Some(filename.clone());
        self.debug_messages.push(format!("File saved: {}", filename));
        Ok(())
    }

    fn open_file(&mut self, filename: &str) -> io::Result<()> {
        let content = fs::read_to_string(filename)?;
        self.content = content.lines().map(String::from).collect();
        self.cursor_position = (0, 0);
        Ok(())
    }

    fn ui<B: tui::backend::Backend>(&self, f: &mut Frame<B>) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Length(6), Constraint::Min(1)].as_ref())
            .split(f.size());

        let debug_messages: Vec<Spans> = self.debug_messages.iter().map(|m| Spans::from(m.clone())).collect();
        let debug_paragraph = Paragraph::new(debug_messages)
            .block(Block::default().borders(Borders::ALL).title("Debug Output"));
        f.render_widget(debug_paragraph, chunks[0]);

        let mode_indicator = match self.mode {
            Mode::Normal => "NORMAL",
            Mode::Insert => "INSERT",
            Mode::Command => "COMMAND",
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(
                format!("Editor - {}", mode_indicator),
                Style::default().add_modifier(Modifier::BOLD),
            ));

        let mut text = Vec::new();
        for line in &self.content {
            text.push(Spans::from(line.clone()));
        }

        if self.mode == Mode::Command {
            text.push(Spans::from(format!(":{}", self.command_buffer)));
        }

        let paragraph = Paragraph::new(text).block(block);
        f.render_widget(paragraph, chunks[1]);

        let cursor_x = self.cursor_position.0 as u16 + 2;
        let cursor_y = self.cursor_position.1 as u16 + 8;
        f.set_cursor(
            cursor_x.min(chunks[1].width - 1),
            cursor_y.min(chunks[1].height - 1),
        )
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let mut editor = if args.len() > 1 {
        match Editor::with_file(&args[1]) {
            Ok(ed) => ed,
            Err(e) => {
                eprintln!("Error opening file: {}", e);
                return Ok(());
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