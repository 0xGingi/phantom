use crate::{
    config::{ColorConfig, Keybindings},
    file_selector::FileSelector,
    mode::{Mode, LineNumberMode},
    tab::{EditOperation, Tab},
    ui,
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseEventKind, MouseButton, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{error::Error, io};
use std::fs;
use std::path::{Path, PathBuf};
use std::env;
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect, Margin, Alignment},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame, Terminal,
};
use syntect::easy::HighlightLines;
use syntect::highlighting::{ThemeSet, Style as SyntectStyle};
use syntect::parsing::SyntaxSet;
use copypasta::ClipboardProvider;
use dirs;
use git2::Status;

pub struct Editor {
    content: Vec<String>,
    cursor_position: (usize, usize),
    pub mode: Mode,
    debug_messages: Vec<String>,
    command_buffer: String,
    current_file: Option<String>,
    ps: SyntaxSet,
    ts: ThemeSet,
    syntax: String,
    cursor_style: Style,
    clipboard_context: crate::ClipboardWrapper,
    visual_start: (usize, usize),
    pub file_selector: Option<FileSelector>,
    show_debug: bool,
    search_query: String,
    search_results: Vec<(usize, usize)>,
    current_search_index: usize,
    scroll_offset: usize,
    horizontal_scroll: usize,
    keybindings: Keybindings,
    color_config: ColorConfig,
    show_sidebar: bool,
    sidebar_width: u16,
    pending_key: Option<String>,
    tabs: Vec<Tab>,
    active_tab: usize,
    mouse_selection_start: Option<(usize, usize)>,
    mouse_selection_end: Option<(usize, usize)>,
    show_minimap: bool,
    minimap_width: u16,
    minimap_line_mapping: Vec<(usize, usize)>,
    current_editor_height: usize,
    current_editor_width: usize,
    line_number_mode: LineNumberMode,
    line_number_width: u16,
    highlight_bracket_matches: bool,
}

impl Editor {
    pub fn new() -> Self {
        let keybindings = Self::load_config().unwrap_or_else(|_| Keybindings::default());
        let color_config = Self::load_color_config().unwrap_or_else(|_| ColorConfig::default());
        let clipboard_context = crate::ClipboardWrapper::new();
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
            clipboard_context,
            visual_start: (0, 0),
            file_selector: None,
            show_debug: false,
            search_query: String::new(),
            search_results: Vec::new(),
            current_search_index: 0,
            scroll_offset: 0,
            horizontal_scroll: 0,
            keybindings,
            color_config,
            show_sidebar: false,
            sidebar_width: 30,
            pending_key: None,
            tabs: vec![Tab::new()],
            active_tab: 0,
            mouse_selection_start: None,
            mouse_selection_end: None,
            show_minimap: false,
            minimap_width: 30,
            minimap_line_mapping: Vec::new(),
            current_editor_height: 24,
            current_editor_width: 80,
            line_number_mode: LineNumberMode::Absolute,
            line_number_width: 0,
            highlight_bracket_matches: true,
        }
    }

    fn is_minimap_area(&self, x: u16, y: u16) -> bool {
        let minimap_x = self.current_editor_width as u16;
        let minimap_width = self.minimap_width;
        let minimap_y = 1;
        let minimap_height = self.minimap_line_mapping.len() as u16 + 1;
    
        x >= minimap_x && x < minimap_x + minimap_width && y >= minimap_y && y < minimap_y + minimap_height
    }

    fn handle_minimap_click(&mut self, _x: u16, y: u16) {
        let total_lines = self.tabs[self.active_tab].content.len();
    
        let adjusted_y = y.saturating_sub(1) as usize;
    
        if adjusted_y >= self.minimap_line_mapping.len() {
            return;
        }
    
        let (min_line, max_line) = self.minimap_line_mapping[adjusted_y];
        let clicked_line = (min_line + max_line) / 2;
    
        let new_cursor_line = clicked_line.min(total_lines.saturating_sub(1));
        let editor_height = self.current_editor_height;
        let new_scroll_offset = new_cursor_line.saturating_sub(editor_height / 2);
    
        let tab = &mut self.tabs[self.active_tab];
        tab.cursor_position.1 = new_cursor_line;
        tab.scroll_offset = new_scroll_offset;
    
        self.ensure_cursor_visible();
    }

    fn ensure_cursor_visible(&mut self) {
        let editor_height = self.current_editor_height;
        let tab = &mut self.tabs[self.active_tab];

        if tab.cursor_position.1 < tab.scroll_offset {
            tab.scroll_offset = tab.cursor_position.1;
        } else if tab.cursor_position.1 >= tab.scroll_offset + editor_height {
            tab.scroll_offset = tab.cursor_position.1 - editor_height + 1;
        }
    }

    fn toggle_minimap(&mut self) -> io::Result<bool> {
        self.show_minimap = !self.show_minimap;
        let status = if self.show_minimap { "shown" } else { "hidden" };
        
        self.debug_messages.push(format!("Minimap toggle attempted. New state: {}", status));
        
        if self.show_minimap {
            if self.tabs[self.active_tab].content.iter().all(|line| line.is_empty()) {
                self.show_minimap = false;
                self.debug_messages.push("Cannot show minimap: No content".to_string());
            } else {
                self.debug_messages.push(format!("Minimap {} (content available)", status));
            }
        } else {
            self.debug_messages.push(format!("Minimap {}", status));
        }
        
        if let Ok((width, height)) = crossterm::terminal::size() {
            self.debug_messages.push(format!("Terminal size: {}x{}", width, height));
        } else {
            self.debug_messages.push("Failed to get terminal size".to_string());
        }
        
        Ok(false)
    }

    fn render_minimap<B: tui::backend::Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        let tab = &self.tabs[self.active_tab];
        let content = &tab.content;
    
        if content.is_empty() {
            let empty_minimap = Paragraph::new("No content")
                .block(Block::default().borders(Borders::ALL).title("Minimap"))
                .style(Style::default()
                    .bg(ui::parse_color(&self.color_config.minimap_background))
                    .fg(ui::parse_color(&self.color_config.minimap_content)));
            f.render_widget(empty_minimap, area);
            return;
        }
    
        let total_lines = content.len();
        let minimap_height = area.height as usize - 2;
        let minimap_width = (area.width as usize - 2) * 2;
    
        let scale_y = (total_lines as f32 / minimap_height as f32).max(1.0);
        let scale_x = 4;
    
        let background_color = ui::parse_color(&self.color_config.minimap_background);
        let foreground_color = ui::parse_color(&self.color_config.minimap_content);
        let comment_color = ui::parse_color(&self.color_config.comment);
        let keyword_color = ui::parse_color(&self.color_config.keyword);
        let string_color = ui::parse_color(&self.color_config.string);
        let function_color = ui::parse_color(&self.color_config.function);
        let minimap_highlight_color = ui::parse_color(&self.color_config.minimap_highlight);
    
        let current_line = tab.cursor_position.1;
        let mut minimap_content = Vec::new();
        let mut line_mapping = Vec::new();
    
        for y in 0..minimap_height {
            let mut line_spans = Vec::new();
            let min_line = (y as f32 * scale_y) as usize;
            let max_line = ((y + 1) as f32 * scale_y).min(total_lines as f32) as usize - 1;
    
            for x in (0..minimap_width).step_by(2) {
                let mut braille_char = 0x2800;
                let mut dot_count = 0;
    
                for dy in 0..4 {
                    for dx in 0..2 {
                        let content_y = (min_line + dy).min(total_lines - 1);
                        let content_x = x / 2 * scale_x + dx;
    
                        if content_x < content[content_y].len() {
                            braille_char |= 1 << (dy + 4 * dx);
                            dot_count += 1;
                        }
                    }
                }
    
                let color = match dot_count {
                    0 => background_color,
                    1..=2 => comment_color,
                    3..=4 => string_color,
                    5..=6 => keyword_color,
                    7..=8 => function_color,
                    _ => foreground_color,
                };
    
                let style = if current_line >= min_line && current_line <= max_line {
                    Style::default().fg(color).bg(minimap_highlight_color)
                } else {
                    Style::default().fg(color)
                };
    
                line_spans.push(Span::styled(
                    char::from_u32(braille_char).unwrap().to_string(),
                    style
                ));
            }
            minimap_content.push(Spans::from(line_spans));
            line_mapping.push((min_line, max_line));
        }
    
        let minimap = Paragraph::new(minimap_content)
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Minimap")
                .border_style(Style::default().fg(ui::parse_color(&self.color_config.minimap_border))))
            .style(Style::default().bg(background_color));
    
        f.render_widget(minimap, area);
    
        self.minimap_line_mapping = line_mapping;
    }

    fn switch_to_tab(&mut self, tab_index: usize) {
        if tab_index < self.tabs.len() {
            self.active_tab = tab_index;
            self.debug_messages.push(format!("Switched to tab {}", tab_index + 1));
            self.update_current_tab_info();
        } else {
            self.debug_messages.push(format!("Tab {} does not exist", tab_index + 1));
        }
    }

    pub fn with_file(path: &Path) -> io::Result<Self> {
        let mut editor = Editor::new();
        editor.open_file(path)?;
        Ok(editor)
    }

    fn close_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.tabs.remove(self.active_tab);
            if self.active_tab >= self.tabs.len() {
                self.active_tab = self.tabs.len() - 1;
            }
            self.update_current_tab_info();
            self.update_tab_name();
        }
    }

    fn update_tab_name(&mut self) {
        let tab = &mut self.tabs[self.active_tab];
        if let Some(path) = &tab.current_file {
            let _file_name = Path::new(path).file_name().unwrap().to_str().unwrap().to_string();
        }
    }

    fn ensure_cursor_in_bounds(&mut self) {
        let tab = &mut self.tabs[self.active_tab];
        if tab.content.is_empty() {
            tab.content.push(String::new());
        }
        tab.cursor_position.1 = tab.cursor_position.1.min(tab.content.len() - 1);
        let line_length = tab.content[tab.cursor_position.1].len();
        tab.cursor_position.0 = tab.cursor_position.0.min(line_length);
    }

    fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab = (self.active_tab + 1) % self.tabs.len();
            self.update_current_tab_info();
        }
    }

    fn new_tab(&mut self) {
        if self.tabs.len() == 1 && self.tabs[0].content == vec![String::new()] && self.tabs[0].current_file.is_none() {
            self.active_tab = 0;
        } else {
            self.tabs.push(Tab::new());
            self.active_tab = self.tabs.len() - 1;
        }
        self.update_tab_name();
    }

    fn previous_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab = (self.active_tab + self.tabs.len() - 1) % self.tabs.len();
            self.update_current_tab_info();
        }
    }

    fn update_current_tab_info(&mut self) {
        let tab = &self.tabs[self.active_tab];
        self.content = tab.content.clone();
        self.cursor_position = tab.cursor_position;
        self.scroll_offset = tab.scroll_offset;
        self.horizontal_scroll = tab.horizontal_scroll;
        self.current_file = tab.current_file.clone();
        self.syntax = tab.syntax.clone();
    }

    fn get_config_dir() -> Option<PathBuf> {
        let mut config_dir = dirs::config_dir()?;
        config_dir.push("phantom");
        Some(config_dir)
    }    

    fn load_color_config() -> Result<ColorConfig, Box<dyn Error>> {
        let config_dir = Self::get_config_dir().ok_or("Could not find config directory")?;
        let config_path = config_dir.join("colors.json");
    
        if !config_path.exists() {
            Self::create_default_color_config(&config_path)?;
        }
    
        let config_str = fs::read_to_string(&config_path)?;
        let config = ColorConfig::from_json(&config_str)?;
        Ok(config)
    }
    
    fn create_default_color_config(config_path: &PathBuf) -> Result<(), Box<dyn Error>> {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
    
        let default_config = ColorConfig::default().to_json()?;
        fs::write(config_path, default_config)?;
        Ok(())
    }
    
    fn save_state(&mut self) {
        let tab_index = self.active_tab;
        let tab = &mut self.tabs[tab_index];
        let operation = EditOperation {
            content: tab.content.clone(),
            cursor_position: tab.cursor_position,
            scroll_offset: tab.scroll_offset,
            horizontal_scroll: tab.horizontal_scroll,
        };
        tab.undo_stack.push_front(operation);
        tab.redo_stack.clear();

        if tab.undo_stack.len() > 100 {
            tab.undo_stack.pop_back();
        }
    }

    fn undo(&mut self) {
        let tab = &mut self.tabs[self.active_tab];
        if let Some(operation) = tab.undo_stack.pop_front() {
            let current_state = EditOperation {
                content: tab.content.clone(),
                cursor_position: tab.cursor_position,
                scroll_offset: tab.scroll_offset,
                horizontal_scroll: tab.horizontal_scroll,
            };
            tab.redo_stack.push_front(current_state);

            tab.content = operation.content;
            tab.cursor_position = operation.cursor_position;
            tab.scroll_offset = operation.scroll_offset;
            tab.horizontal_scroll = operation.horizontal_scroll;
        }
    }

    fn redo(&mut self) {
        let tab = &mut self.tabs[self.active_tab];
        if let Some(operation) = tab.redo_stack.pop_front() {
            let current_state = EditOperation {
                content: tab.content.clone(),
                cursor_position: tab.cursor_position,
                scroll_offset: tab.scroll_offset,
                horizontal_scroll: tab.horizontal_scroll,
            };
            tab.undo_stack.push_front(current_state);

            tab.content = operation.content;
            tab.cursor_position = operation.cursor_position;
            tab.scroll_offset = operation.scroll_offset;
            tab.horizontal_scroll = operation.horizontal_scroll;
        }
    }

    fn load_config() -> Result<Keybindings, Box<dyn Error>> {
        let config_dir = Self::get_config_dir().ok_or("Could not find config directory")?;
        let config_path = config_dir.join("config.toml");
    
        if !config_path.exists() {
            Self::create_default_config(&config_path)?;
        }
    
        let config_str = fs::read_to_string(&config_path)?;
        let config: Keybindings = toml::from_str(&config_str)?;
        Ok(config)
    }
            
    fn key_event_to_string(key: event::KeyEvent) -> String {
        let mut key_string = String::new();
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            key_string.push_str("Ctrl+");
        }
        if key.modifiers.contains(KeyModifiers::ALT) {
            key_string.push_str("Alt+");
        }
        if key.modifiers.contains(KeyModifiers::SHIFT) {
            key_string.push_str("Shift+");
        }
        match key.code {
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    key_string.push(c.to_ascii_lowercase());
                } else {
                    key_string.push(c);
                }
            },
            KeyCode::F(n) => key_string.push_str(&format!("F{}", n)),
            KeyCode::Enter => key_string.push_str("Enter"),
            KeyCode::Left => key_string.push_str("Left"),
            KeyCode::Right => key_string.push_str("Right"),
            KeyCode::Up => key_string.push_str("Up"),
            KeyCode::Down => key_string.push_str("Down"),
            KeyCode::Backspace => key_string.push_str("Backspace"),
            KeyCode::Delete => key_string.push_str("Delete"),
            KeyCode::Home => key_string.push_str("Home"),
            KeyCode::End => key_string.push_str("End"),
            KeyCode::PageUp => key_string.push_str("PageUp"),
            KeyCode::PageDown => key_string.push_str("PageDown"),
            KeyCode::Tab => key_string.push_str("Tab"),
            KeyCode::BackTab => key_string.push_str("BackTab"),
            KeyCode::Insert => key_string.push_str("Insert"),
            KeyCode::Esc => key_string.push_str("Esc"),
            _ => key_string.push_str(&format!("{:?}", key.code)),
        }
        key_string
    }

    fn create_default_config(config_path: &PathBuf) -> Result<(), Box<dyn Error>> {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
    
        let default_config = toml::to_string_pretty(&Keybindings::default())?;
        fs::write(config_path, default_config)?;
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
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

    fn run_app<B: tui::backend::Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<bool> {
        loop {
            terminal.draw(|f| self.ui(f))?;
    
            if let Ok(event) = event::read() {
                match event {
                    Event::Mouse(mouse_event) => {
                        match mouse_event.kind {
                            MouseEventKind::Down(MouseButton::Left) => {
                                let (x, y) = (mouse_event.column, mouse_event.row);
                                if self.is_minimap_area(x, y) {
                                    self.handle_minimap_click(x, y);
                                } else {
                                    let (x, y) = (mouse_event.column as usize, mouse_event.row as usize);
                                    self.start_mouse_selection(x, y);
                                }
                            }
                            MouseEventKind::Drag(MouseButton::Left) => {
                                let (x, y) = (mouse_event.column as usize, mouse_event.row as usize);
                                self.update_mouse_selection(x, y);
                            }
                            MouseEventKind::Up(MouseButton::Right) => {
                                self.copy_selection_to_clipboard();
                                self.end_mouse_selection();
                            }          
                            _ => {}
                        }
                    }
                    Event::Key(key) => {
                        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('q') {
                            return Ok(true);
                        }

                        self.debug_messages.push(format!("Key pressed: {:?}", key));
                        self.debug_messages.push(format!("Cursor: ({}, {})", self.cursor_position.0, self.cursor_position.1));
                        
                        while self.debug_messages.len() > 5 {
                            self.debug_messages.remove(0);
                        }
    
                        if self.handle_key_event(key)? {
                            return Ok(true);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn copy_selection_to_clipboard(&mut self) {
        if let (Some(start), Some(end)) = (self.mouse_selection_start, self.mouse_selection_end) {
            let (start, end) = if start <= end { (start, end) } else { (end, start) };
            let tab = &self.tabs[self.active_tab];
            let mut selected_text = String::new();
    
            for i in start.1..=end.1 {
                if i >= tab.content.len() {
                    break;
                }
                let line = &tab.content[i];
                if i == start.1 && i == end.1 {
                    selected_text.push_str(&line[start.0.min(line.len())..end.0.min(line.len())]);
                } else if i == start.1 {
                    selected_text.push_str(&line[start.0.min(line.len())..]);
                } else if i == end.1 {
                    selected_text.push_str(&line[..end.0.min(line.len())]);
                } else {
                    selected_text.push_str(line);
                }
                if i != end.1 {
                    selected_text.push('\n');
                }
            }
    
            if let Err(e) = self.clipboard_context.set_contents(selected_text) {
                self.debug_messages.push(format!("Failed to copy to clipboard: {}", e));
            } else {
                self.debug_messages.push("Text copied to clipboard".to_string());
            }
        }
    }
    
    fn start_mouse_selection(&mut self, x: usize, y: usize) {
        let position = self.screen_to_content_position(x, y);
        self.mouse_selection_start = Some(position);
        self.mouse_selection_end = Some(position);
    }

    fn update_mouse_selection(&mut self, x: usize, y: usize) {
        let position = self.screen_to_content_position(x, y);
        self.mouse_selection_end = Some(position);
    }

    fn end_mouse_selection(&mut self) {
        self.mouse_selection_start = None;
        self.mouse_selection_end = None;
    }

    fn screen_to_content_position(&self, x: usize, y: usize) -> (usize, usize) {
        let tab = &self.tabs[self.active_tab];
        let line = y.saturating_sub(4) + tab.scroll_offset;
        let column = x.saturating_sub(1) + tab.horizontal_scroll;
        (column, line)
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> io::Result<bool> {
        let _key_str = Self::key_event_to_string(key);
        
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('m') {
            self.debug_messages.push("Ctrl+M detected, toggling minimap".to_string());
            return self.toggle_minimap();
        }

        match key.code {
            KeyCode::F(n) if n >= 1 && n <= 9 => {
                let tab_index = n as usize - 1;
                if tab_index < self.tabs.len() {
                    self.switch_to_tab(tab_index);
                    return Ok(false);
                }
            }
            KeyCode::Char('q') if key.modifiers == KeyModifiers::CONTROL => return Ok(true),
            _ => {}
        }
            
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
            Mode::SidebarActive => self.handle_sidebar_active_mode(key),
        }
    }
    
    fn toggle_sidebar(&mut self) -> io::Result<bool> {
        self.show_sidebar = !self.show_sidebar;
        if self.show_sidebar {
            let current_dir = if let Some(ref file) = self.current_file {
                let path = Path::new(file);
                if let Some(parent) = path.parent() {
                    if parent.exists() {
                        parent.to_path_buf()
                    } else {
                        env::current_dir()?
                    }
                } else {
                    env::current_dir()?
                }
            } else {
                env::current_dir()?
            };
            self.file_selector = Some(FileSelector::new(&current_dir)?);
            self.mode = Mode::SidebarActive;
        } else {
            self.mode = Mode::Normal;
        }
        Ok(false)
    }

    fn handle_normal_mode(&mut self, key: KeyEvent) -> io::Result<bool> {
        let key_str = Self::key_event_to_string(key);
        
        if let Some(pending) = self.pending_key.take() {
            let combined_key = format!("{}{}", pending, key_str);
            if let Some(action) = self.keybindings.normal_mode.get(&combined_key).cloned() {
                return self.execute_action(&action);
            }
        }
    
        if let Some(action) = self.keybindings.normal_mode.get(&key_str).cloned() {
            self.execute_action(&action)
        } else {
            if self.keybindings.normal_mode.keys().any(|k| k.starts_with(&key_str)) {
                self.pending_key = Some(key_str);
                Ok(false)
            } else {
                Ok(false)
            }
        }
    }

    fn execute_action(&mut self, action: &str) -> io::Result<bool> {
        match action {
            "enter_insert_mode" => {
                self.mode = Mode::Insert;
                Ok(false)
            },
            "append" => {
                self.mode = Mode::Insert;
                self.move_cursor_right();
                Ok(false)
            },
            "open_line_below" => {
                self.insert_line_below();
                self.mode = Mode::Insert;
                Ok(false)
            },
            "open_line_above" => {
                self.insert_line_above();
                self.mode = Mode::Insert;
                Ok(false)
            },
            "delete_line" => {
                self.delete_line();
                Ok(false)
            },
            "yank_line" => {
                self.yank_line();
                Ok(false)
            },
            "paste_after" => {
                self.paste_after();
                Ok(false)
            },
            "enter_visual_mode" => {
                self.mode = Mode::Visual;
                self.visual_start = self.cursor_position;
                Ok(false)
            },
            "enter_command_mode" => {
                self.mode = Mode::Command;
                self.command_buffer.clear();
                Ok(false)
            },
            "toggle_debug_menu" => {
                self.toggle_debug_menu();
                Ok(false)
            },
            "toggle_line_numbers" => self.toggle_line_numbers(),
            "enter_directory_nav_mode" => self.enter_directory_nav_mode(),
            "enter_search_mode" => {
                self.enter_search_mode();
                Ok(false)
            },
            "next_search_result" => {
                self.next_search_result();
                Ok(false)
            },
            "previous_search_result" => {
                self.previous_search_result();
                Ok(false)
            },
            "copy_selection" => {
                self.copy_selection();
                Ok(false)
            },
            "paste_clipboard" => {
                self.paste_clipboard();
                Ok(false)
            },
            "undo" => {
                self.undo();
                Ok(false)
            },
            "redo" => {
                self.redo();
                Ok(false)
            },
            "toggle_sidebar" => self.toggle_sidebar(),
            "next_tab" => {
                self.next_tab();
                self.update_current_tab_info();
                Ok(false)
            },
            "previous_tab" => {
                self.previous_tab();
                self.update_current_tab_info();
                Ok(false)
            },
            "switch_to_tab_1" => {
                self.switch_to_tab(0);
                self.update_current_tab_info();
                Ok(false)
            },
            "switch_to_tab_2" => {
                self.switch_to_tab(1);
                self.update_current_tab_info();
                Ok(false)
            },
            "switch_to_tab_3" => {
                self.switch_to_tab(2);
                self.update_current_tab_info();
                Ok(false)
            },
            "switch_to_tab_4" => {
                self.switch_to_tab(3);
                self.update_current_tab_info();
                Ok(false)
            },
            "switch_to_tab_5" => {
                self.switch_to_tab(4);
                self.update_current_tab_info();
                Ok(false)
            },
            "switch_to_tab_6" => {
                self.switch_to_tab(5);
                self.update_current_tab_info();
                Ok(false)
            },
            "switch_to_tab_7" => {
                self.switch_to_tab(6);
                self.update_current_tab_info();
                Ok(false)
            },
            "switch_to_tab_8" => {
                self.switch_to_tab(7);
                self.update_current_tab_info();
                Ok(false)
            },
            "switch_to_tab_9" => {
                self.switch_to_tab(8);
                self.update_current_tab_info();
                Ok(false)
            },
            "new_tab" => {
                self.new_tab();
                self.update_current_tab_info();
                Ok(false)
            },
            "close_tab" => {
                self.close_tab();
                self.update_current_tab_info();
                Ok(false)
            },
            "toggle_minimap" => self.toggle_minimap(),
            "exit_insert_mode" => {
                self.mode = Mode::Normal;
                Ok(false)
            },
            "insert_newline" => {
                self.insert_newline();
                Ok(false)
            },
            "backspace" => {
                self.backspace();
                Ok(false)
            },
            "delete_char" => {
                self.delete_char();
                Ok(false)
            },
            "move_cursor_left" => {
                self.move_cursor_left();
                Ok(false)
            },
            "move_cursor_right" => {
                self.move_cursor_right();
                Ok(false)
            },
            "move_cursor_up" => {
                self.move_cursor_up();
                Ok(false)
            },
            "move_cursor_down" => {
                self.move_cursor_down();
                Ok(false)
            },
            "move_cursor_start_of_line" => {
                self.move_cursor_start_of_line();
                Ok(false)
            },
            "move_cursor_end_of_line" => {
                self.move_cursor_end_of_line();
                Ok(false)
            },
            "page_up" => {
                self.page_up();
                Ok(false)
            },
            "page_down" => {
                self.page_down();
                Ok(false)
            },
            "execute_search" => {
                self.perform_search();
                self.mode = Mode::Normal;
                Ok(false)
            },
            "exit_search_mode" => {
                self.mode = Mode::Normal;
                Ok(false)
            },
            "delete_selection" => {
                self.delete_selection();
                self.mode = Mode::Normal;
                Ok(false)
            },
            "yank_selection" => {
                self.copy_selection();
                self.mode = Mode::Normal;
                Ok(false)
            },
            "exit_visual_mode" => {
                self.mode = Mode::Normal;
                Ok(false)
            },
            "select_file" => {
                if let Some(file_selector) = &mut self.file_selector {
                    if let Some(path) = file_selector.enter()? {
                        self.open_file(&path)?;
                        self.mode = Mode::Normal;
                        self.file_selector = None;
                    }
                }
                Ok(false)
            },
            "exit_file_select_mode" => {
                self.mode = Mode::Normal;
                self.file_selector = None;
                Ok(false)
            },
            "search_backspace" => {
                self.search_query.pop();
                Ok(false)
            },
            _ => Ok(false),
        }
    }

    fn handle_sidebar_active_mode(&mut self, key: KeyEvent) -> io::Result<bool> {
        let key_str = Self::key_event_to_string(key);
        
        if let Some(action) = self.keybindings.normal_mode.get(&key_str) {
            if action == "toggle_sidebar" {
                return self.toggle_sidebar();
            }
        }
    
        if let Some(file_selector) = &mut self.file_selector {
            match key.code {
                KeyCode::Up => file_selector.up(),
                KeyCode::Down => file_selector.down(),
                KeyCode::Enter => {
                    if let Some(path) = file_selector.enter()? {
                        self.open_file(&path)?;
                        self.toggle_sidebar()?;
                    }
                }
                KeyCode::Esc => {
                    self.toggle_sidebar()?;
                }
                _ => {}
            }
        }
        Ok(false)
    }

    fn handle_insert_mode(&mut self, key: KeyEvent) -> io::Result<bool> {
        let key_str = Self::key_event_to_string(key);
        
        if let Some(action) = self.keybindings.insert_mode.get(&key_str).cloned() {
            self.execute_action(&action)
        } else {
            match key.code {
                KeyCode::Char(c) => self.insert_char(c),
                _ => {}
            }
            Ok(false)
        }
    }

    fn handle_command_mode(&mut self, key: KeyEvent) -> io::Result<bool> {
        match key.code {
            KeyCode::Enter => return Ok(true),
            KeyCode::Char(c) => self.command_buffer.push(c),
            KeyCode::Backspace => { self.command_buffer.pop(); }
            KeyCode::Esc => self.mode = Mode::Normal,
            _ => {}
        }
        Ok(false)
    }
    
    fn handle_visual_mode(&mut self, key: KeyEvent) -> io::Result<bool> {
        let key_str = Self::key_event_to_string(key);
        
        if let Some(action) = self.keybindings.visual_mode.get(&key_str).cloned() {
            self.execute_action(&action)
        } else {
            Ok(false)
        }
    }
    
    fn handle_file_select_mode(&mut self, key: KeyEvent) -> io::Result<bool> {
        let key_str = Self::key_event_to_string(key);
        
        if let Some(action) = self.keybindings.file_select_mode.get(&key_str).cloned() {
            if let Some(file_selector) = &mut self.file_selector {
                match action.as_str() {
                    "move_cursor_up" => {
                        file_selector.up();
                        Ok(false)
                    },
                    "move_cursor_down" => {
                        file_selector.down();
                        Ok(false)
                    },
                    _ => self.execute_action(&action)
                }
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }
    
    fn execute_command(&mut self) -> io::Result<bool> {
        let command = self.command_buffer.clone();
        self.mode = Mode::Normal;
        self.command_buffer.clear();

        match command.as_str() {
            "q" => {
                if self.tabs.len() > 1 {
                    self.close_tab();
                    Ok(false)
                } else {
                    Ok(true)
                }
            }
            "w" => {
                self.save_file(None)?;
                Ok(false)
            }
            cmd if cmd.starts_with("w ") => {
                let filename = cmd.split_whitespace().nth(1).unwrap();
                self.save_file(Some(Path::new(filename)))?;
                Ok(false)
            }
            "wq" => {
                self.save_file(None)?;
                if self.tabs.len() > 1 {
                    self.close_tab();
                    Ok(false)
                } else {
                    Ok(true)
                }
            }

            cmd if cmd.starts_with("e ") => {
                let filename = cmd.split_whitespace().nth(1).unwrap();
                self.open_file(Path::new(filename))?;
                Ok(false)
            }
            _ => {
                self.debug_messages.push(format!("Unknown command: {}", command));
                Ok(false)                
            }
        }
    }

    fn move_cursor_up(&mut self) {
        let tab = &mut self.tabs[self.active_tab];
        if tab.cursor_position.1 > 0 {
            tab.cursor_position.1 -= 1;
            if tab.cursor_position.1 < tab.scroll_offset {
                tab.scroll_offset = tab.cursor_position.1;
            }
        }
    }
    
    fn move_cursor_down(&mut self) {
        let editor_height = self.current_editor_height;
        let tab = &mut self.tabs[self.active_tab];
        if tab.cursor_position.1 < tab.content.len() - 1 {
            tab.cursor_position.1 += 1;
            if tab.cursor_position.1 >= tab.scroll_offset + editor_height {
                tab.scroll_offset = tab.cursor_position.1 - editor_height + 1;
            }
        }
    }

    fn move_cursor_left(&mut self) {
        let tab = &mut self.tabs[self.active_tab];
        if tab.cursor_position.0 > 0 {
            tab.cursor_position.0 -= 1;
            if tab.cursor_position.0 < tab.horizontal_scroll {
                tab.horizontal_scroll = tab.cursor_position.0;
            }
        } else if tab.cursor_position.1 > 0 {
            tab.cursor_position.1 -= 1;
            tab.cursor_position.0 = tab.content[tab.cursor_position.1].len();
            tab.adjust_horizontal_scroll(self.current_editor_width);
        }
    }

    fn move_cursor_right(&mut self) {
        let tab = &mut self.tabs[self.active_tab];
        if tab.cursor_position.0 < tab.content[tab.cursor_position.1].len() {
            tab.cursor_position.0 += 1;
            tab.adjust_horizontal_scroll(self.current_editor_width);
        } else if tab.cursor_position.1 < tab.content.len() - 1 {
            tab.cursor_position.1 += 1;
            tab.cursor_position.0 = 0;
            tab.horizontal_scroll = 0;
        }
    }
    
    fn move_cursor_start_of_line(&mut self) {
        let tab = &mut self.tabs[self.active_tab];
        tab.cursor_position.0 = 0;
        tab.adjust_horizontal_scroll(self.current_editor_width);
    }

    fn move_cursor_end_of_line(&mut self) {
        let tab = &mut self.tabs[self.active_tab];
        tab.cursor_position.0 = tab.content[tab.cursor_position.1].len();
        tab.adjust_horizontal_scroll(self.current_editor_width);
    }

    fn insert_char(&mut self, c: char) {
        self.save_state();
        let tab = &mut self.tabs[self.active_tab];
        let line = &mut tab.content[tab.cursor_position.1];
        line.insert(tab.cursor_position.0, c);
        tab.cursor_position.0 += 1;
        tab.adjust_horizontal_scroll(self.current_editor_width);
    }

    fn insert_newline(&mut self) {
        self.save_state();
        let tab = &mut self.tabs[self.active_tab];
        let (x, y) = tab.cursor_position;
    
        let current_line = &mut tab.content[y];
    
        let leading_whitespace = current_line.chars()
            .take_while(|c| c.is_whitespace())
            .collect::<String>();
    
        let rest_of_line = current_line.split_off(x);
    
        let mut new_line = leading_whitespace.clone();
        new_line.push_str(&rest_of_line);
    
        tab.content.insert(y + 1, new_line);
    
        tab.cursor_position = (leading_whitespace.len(), y + 1);
    
        self.ensure_cursor_visible(); 
    }

    fn page_up(&mut self) {
        let visible_lines = self.current_editor_height;
        let tab = &mut self.tabs[self.active_tab];
        if tab.scroll_offset > visible_lines {
            tab.scroll_offset -= visible_lines;
        } else {
            tab.scroll_offset = 0;
        }
        tab.cursor_position.1 = tab.scroll_offset;
    }
    
    fn page_down(&mut self) {
        let visible_lines = self.current_editor_height;
        let tab = &mut self.tabs[self.active_tab];
        let max_scroll = tab.content.len().saturating_sub(visible_lines);
        if tab.scroll_offset + visible_lines < max_scroll {
            tab.scroll_offset += visible_lines;
        } else {
            tab.scroll_offset = max_scroll;
        }
        tab.cursor_position.1 = tab.scroll_offset + visible_lines - 1;
        if tab.cursor_position.1 >= tab.content.len() {
            tab.cursor_position.1 = tab.content.len() - 1;
        }
    }

    fn backspace(&mut self) {
        self.save_state();
        let tab = &mut self.tabs[self.active_tab];
        if tab.cursor_position.0 > 0 {
            let line = &mut tab.content[tab.cursor_position.1];
            line.remove(tab.cursor_position.0 - 1);
            tab.cursor_position.0 -= 1;
        } else if tab.cursor_position.1 > 0 {
            let current_line = tab.content.remove(tab.cursor_position.1);
            tab.cursor_position.1 -= 1;
            tab.cursor_position.0 = tab.content[tab.cursor_position.1].len();
            tab.content[tab.cursor_position.1].push_str(&current_line);
        }
    }

    fn delete_char(&mut self) {
        self.save_state();
        let tab = &mut self.tabs[self.active_tab];
        let line = &mut tab.content[tab.cursor_position.1];
        if tab.cursor_position.0 < line.len() {
            line.remove(tab.cursor_position.0);
        } else if tab.cursor_position.1 < tab.content.len() - 1 {
            let next_line = tab.content.remove(tab.cursor_position.1 + 1);
            tab.content[tab.cursor_position.1].push_str(&next_line);
        }
    }

    fn delete_line(&mut self) {
        let tab_index = self.active_tab;
        
        if self.tabs[tab_index].cursor_position.1 < self.tabs[tab_index].content.len() {
            self.save_state();

            let tab = &mut self.tabs[tab_index];
            let cursor_y = tab.cursor_position.1;
            
            let line = tab.content.remove(cursor_y);
            self.clipboard_context.set_contents(line).unwrap();
            
            if tab.content.is_empty() {
                tab.content.push(String::new());
            }
            
            if cursor_y == tab.content.len() && cursor_y > 0 {
                tab.cursor_position.1 -= 1;
            }
            
            tab.cursor_position.0 = 0;
        }
    }

    fn insert_line_below(&mut self) {
        self.save_state();
        let tab = &mut self.tabs[self.active_tab];
        tab.content.insert(tab.cursor_position.1 + 1, String::new());
        tab.cursor_position = (0, tab.cursor_position.1 + 1);
    }

    fn insert_line_above(&mut self) {
        self.save_state();
        let tab = &mut self.tabs[self.active_tab];
        tab.content.insert(tab.cursor_position.1, String::new());
        tab.cursor_position = (0, tab.cursor_position.1);
    }

    fn yank_line(&mut self) {
        self.save_state();
        let tab = &mut self.tabs[self.active_tab];
        if tab.cursor_position.1 < tab.content.len() {
            let line = tab.content[tab.cursor_position.1].clone();
            self.clipboard_context.set_contents(line).unwrap();
        }
    }

    fn paste_after(&mut self) {
        if let Ok(content) = self.clipboard_context.get_contents() {
            self.save_state();
            
            let tab = &mut self.tabs[self.active_tab];
            let current_line = tab.cursor_position.1;
            let current_column = tab.cursor_position.0;

            if current_line >= tab.content.len() {
                tab.content.push(String::new());
            }

            let line = tab.content[current_line].clone();
            let (left, right) = line.split_at(current_column.min(line.len()));

            let mut new_lines: Vec<String> = content.split('\n').map(String::from).collect();
            
            if new_lines.is_empty() {
                new_lines.push(String::new());
            }

            let first_new_line = new_lines.remove(0);
            let mut combined_lines = vec![format!("{}{}", left, first_new_line)];
            combined_lines.extend(new_lines);
            combined_lines.push(right.to_string());

            let combined_lines_len = combined_lines.len();
            tab.content.splice(current_line..=current_line, combined_lines);

            let last_inserted_line = current_line + combined_lines_len - 1;
            tab.cursor_position = (tab.content[last_inserted_line].len() - right.len(), last_inserted_line);
        }
        self.ensure_cursor_in_bounds();
    }

    fn copy_selection(&mut self) {
        let tab = &mut self.tabs[self.active_tab];
        let (start, end) = if self.visual_start <= tab.cursor_position {
            (self.visual_start, tab.cursor_position)
        } else {
            (tab.cursor_position, self.visual_start)
        };

        let mut selected_text = String::new();
        for (i, line) in tab.content.iter().enumerate().skip(start.1).take(end.1 - start.1 + 1) {
            if i == start.1 {
                selected_text.push_str(&line[start.0.min(line.len())..]);
            } else if i == end.1 {
                selected_text.push_str(&line[..end.0.min(line.len())]);
            } else {
                selected_text.push_str(line);
            }
            if i != end.1 {
                selected_text.push('\n');
            }
        }

        if let Err(e) = self.clipboard_context.set_contents(selected_text) {
            self.debug_messages.push(format!("Failed to copy to clipboard: {}", e));
        } else {
            self.debug_messages.push("Text copied to clipboard".to_string());
        }
    }

    fn delete_selection(&mut self) {
        self.save_state();
        let tab = &mut self.tabs[self.active_tab];
        let (start, end) = if self.visual_start <= tab.cursor_position {
            (self.visual_start, tab.cursor_position)
        } else {
            (tab.cursor_position, self.visual_start)
        };
    
        if start.1 == end.1 {
            let line = &mut tab.content[start.1];
            line.replace_range(start.0..=end.0, "");
        } else {
            let mut new_line = tab.content[start.1][..start.0].to_string();
            new_line.push_str(&tab.content[end.1][end.0 + 1..]);
            tab.content.drain(start.1..=end.1);
            tab.content.insert(start.1, new_line);
        }
    
        tab.cursor_position = start;
    }

    fn paste_clipboard(&mut self) {
        match self.clipboard_context.get_contents() {
            Ok(_content) => {

            if let Ok(content) = self.clipboard_context.get_contents() {
                self.save_state();
                let tab = &mut self.tabs[self.active_tab];
                let lines: Vec<&str> = content.split('\n').collect();
                if lines.len() == 1 {
                    let line = &mut tab.content[tab.cursor_position.1];
                    line.insert_str(tab.cursor_position.0, &content);
                    tab.cursor_position.0 += content.len();
                } else {
                    let current_line = &mut tab.content[tab.cursor_position.1];
                    let rest_of_line = current_line.split_off(tab.cursor_position.0);
                    current_line.push_str(lines[0]);
                    for line in lines.iter().skip(1).take(lines.len() - 2) {
                        tab.content.insert(tab.cursor_position.1 + 1, line.to_string());
                        tab.cursor_position.1 += 1;
                    }
                    tab.content.insert(tab.cursor_position.1 + 1, format!("{}{}", lines.last().unwrap_or(&""), rest_of_line));
                    tab.cursor_position = (lines.last().unwrap_or(&"").len(), tab.cursor_position.1 + 1);
                    }
                }
            }
            Err(e) => {
                self.debug_messages.push(format!("Failed to paste from clipboard: {}", e));
            }
        }
    }

    fn save_file(&mut self, filename: Option<&Path>) -> io::Result<()> {
        let tab_index = self.active_tab;
        let filename_path = if let Some(name) = filename {
            name.to_path_buf()
        } else if let Some(ref name) = self.tabs[tab_index].current_file {
            PathBuf::from(name)
        } else {
            return Err(io::Error::new(io::ErrorKind::Other, "No filename specified. Use :w <filename> to save."));
        };

        if let Some(parent) = filename_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content_to_save = self.tabs[tab_index].content.join("\n");
        fs::write(&filename_path, content_to_save)?;
        
        let tab = &mut self.tabs[tab_index];
        tab.current_file = Some(filename_path.to_string_lossy().into_owned());
        let (status, branch) = Tab::get_git_info(&filename_path);
        tab.git_status = status;
        tab.git_branch = branch;

        self.update_tab_name();
        self.debug_messages.push(format!("File saved: {}", filename_path.display()));
        Ok(())
    }

    fn open_file(&mut self, path: &Path) -> io::Result<()> {
        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        let new_tab = Tab::from_file(&canonical_path, &self.ps)?;

        if self.tabs.len() == 1 && self.tabs[0].content == vec![String::new()] && self.tabs[0].current_file.is_none() {
            self.tabs[0] = new_tab;
            self.active_tab = 0;
        } else {
            if let Some(existing_index) = self.tabs.iter().position(|t| t.current_file.as_deref() == Some(canonical_path.to_string_lossy().as_ref())) {
                self.active_tab = existing_index;
            } else {
                self.tabs.push(new_tab);
                self.active_tab = self.tabs.len() - 1;
            }
        }

        self.update_current_tab_info();
        self.update_tab_name();

        if canonical_path.exists() {
            self.debug_messages.push(format!("File opened: {}", canonical_path.display()));
        } else {
            self.debug_messages.push(format!("New file: {} (not yet saved)", canonical_path.display()));
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

    fn toggle_line_numbers(&mut self) -> io::Result<bool> {
        self.line_number_mode = match self.line_number_mode {
            LineNumberMode::Off => LineNumberMode::Absolute,
            LineNumberMode::Absolute => LineNumberMode::Relative,
            LineNumberMode::Relative => LineNumberMode::Hybrid,
            LineNumberMode::Hybrid => LineNumberMode::Off,
        };
        self.debug_messages.push(format!("Line number mode: {:?}", self.line_number_mode));
        Ok(false)
    }

    fn enter_directory_nav_mode(&mut self) -> io::Result<bool> {
        let current_dir = if let Some(ref file) = self.current_file {
            Path::new(file).parent().unwrap_or(Path::new(".")).to_path_buf()
        } else {
            env::current_dir()?
        };
        self.file_selector = Some(FileSelector::new(&current_dir)?);
        self.mode = Mode::DirectoryNav;
        Ok(false)
    }

    fn ui<B: tui::backend::Backend>(&mut self, f: &mut Frame<B>) {
        if self.mode == Mode::FileSelect {
            if let Some(file_selector) = &self.file_selector {
                let area = f.size();
                file_selector.render(f, area, &self.color_config);
                return;
            }
        }

        let active_tab = &self.tabs[self.active_tab];
        let total_lines = active_tab.content.len();
        let cursor_position = active_tab.cursor_position;
        let scroll_offset = active_tab.scroll_offset;
        let horizontal_scroll = active_tab.horizontal_scroll;

        let maybe_match_pos = if self.highlight_bracket_matches {
            self.find_matching_bracket(cursor_position)
        } else {
            None
        };
        let bracket_match_style = Style::default()
            .bg(ui::parse_color(&self.color_config.bracket_match))
            .add_modifier(Modifier::BOLD);

        let line_number_digit_count = total_lines.to_string().len();
        let calculated_line_number_width = if self.line_number_mode != LineNumberMode::Off {
            (line_number_digit_count as u16).max(3) + 2
        } else {
            0
        };
        self.line_number_width = calculated_line_number_width;

        let total_width = f.size().width;
        let sidebar_width = if self.show_sidebar { self.sidebar_width } else { 0 };
        let minimap_width = if self.show_minimap && !active_tab.content.is_empty() { self.minimap_width } else { 0 };
        let editor_column_width = total_width.saturating_sub(sidebar_width + minimap_width);

        let mut constraints = vec![];
        if sidebar_width > 0 {
            constraints.push(Constraint::Length(sidebar_width));
        }
        constraints.push(Constraint::Length(editor_column_width));
        if minimap_width > 0 {
            constraints.push(Constraint::Length(minimap_width));
        }

        let main_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(f.size());

        let mut current_layout_index = 0;

        if self.show_sidebar {
            if let Some(file_selector) = &self.file_selector {
                file_selector.render(f, main_layout[current_layout_index], &self.color_config);
            }
            current_layout_index += 1;
        }

        let editor_area = main_layout[current_layout_index];
        current_layout_index += 1;

        let tab_bar_height = 3;
        let editor_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                if self.show_debug {
                    vec![
                        Constraint::Length(tab_bar_height),
                        Constraint::Length(6),
                        Constraint::Min(1),
                        Constraint::Length(1)
                    ]
                } else {
                    vec![
                        Constraint::Length(tab_bar_height),
                        Constraint::Min(1),
                        Constraint::Length(1)
                    ]
                }
            )
            .split(editor_area);

        let tab_titles: Vec<Spans> = self.tabs.iter().enumerate().map(|(i, tab)| {
            let base_title = tab.current_file.as_ref()
                .and_then(|f| Path::new(f).file_name())
                .and_then(|f| f.to_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("Untitled-{}", i + 1));

            let status_indicator = match tab.git_status {
                Some(s) if s.contains(Status::WT_MODIFIED) | s.contains(Status::INDEX_MODIFIED) => "*",
                Some(s) if s.contains(Status::WT_NEW) | s.contains(Status::INDEX_NEW) => "+",
                Some(s) if s.contains(Status::WT_DELETED) | s.contains(Status::INDEX_DELETED) => "-",
                Some(s) if s.contains(Status::WT_RENAMED) | s.contains(Status::INDEX_RENAMED) => "R",
                Some(s) if s.contains(Status::WT_TYPECHANGE) | s.contains(Status::INDEX_TYPECHANGE) => "T",
                _ => "",
            };
            let title_with_status = format!("{}{}", base_title, status_indicator);

            let style = if i == self.active_tab {
                Style::default().fg(ui::parse_color(&self.color_config.tab_active))
            } else {
                Style::default().fg(ui::parse_color(&self.color_config.tab_inactive))
            };
            Spans::from(vec![
                Span::styled(format!(" {} ", i + 1), style),
                Span::styled(title_with_status, style),
                Span::raw(" "),
            ])
        }).collect();

        let tab_bar = Tabs::new(tab_titles)
            .block(Block::default().borders(Borders::ALL).title("Tabs"))
            .select(self.active_tab)
            .style(Style::default().bg(ui::parse_color(&self.color_config.tab_background)))
            .highlight_style(Style::default().fg(ui::parse_color(&self.color_config.tab_active)));

        f.render_widget(tab_bar, editor_layout[0]);

        let editor_chunk_index = if self.show_debug { 2 } else { 1 };
        let editor_widget_rect = editor_layout[editor_chunk_index];

        let editor_height = editor_widget_rect.height.saturating_sub(2) as usize;
        let editor_text_width = editor_widget_rect.width
            .saturating_sub(2)
            .saturating_sub(self.line_number_width) as usize;

        self.current_editor_height = editor_height;
        self.current_editor_width = editor_text_width;

        let content = &active_tab.content;

        let inner_rect = editor_widget_rect.inner(&Margin { vertical: 1, horizontal: 1 });

        let (line_number_area, text_content_area) = if self.line_number_mode != LineNumberMode::Off {
            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(self.line_number_width),
                    Constraint::Min(0),
                ])
                .split(inner_rect);
            (layout[0], layout[1])
        } else {
            (Rect::default(), inner_rect)
        };

        let mode_indicator = match self.mode {
            Mode::Normal => "NORMAL",
            Mode::Insert => "INSERT",
            Mode::Command => "COMMAND",
            Mode::Visual => "VISUAL",
            Mode::FileSelect => "FILE SELECT",
            Mode::DirectoryNav => "DIRECTORY NAV",
            Mode::Search => "SEARCH",
            Mode::SidebarActive => "SIDEBAR",
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(
                format!("Phantom - {}", mode_indicator),
                Style::default()
                    .fg(ui::parse_color(&self.color_config.foreground))
                    .add_modifier(Modifier::BOLD),
            ));
        f.render_widget(block, editor_widget_rect);

        if self.line_number_mode != LineNumberMode::Off {
            let line_number_style = Style::default()
                .fg(ui::parse_color(&self.color_config.line_number))
                .bg(ui::parse_color(&self.color_config.background));
            let current_line_style = line_number_style.add_modifier(Modifier::BOLD);
            let max_width = (self.line_number_width - 1) as usize;

            let line_numbers: Vec<Spans> = (0..editor_height)
                .map(|i| {
                    let current_absolute_line = scroll_offset + i + 1;
                    if current_absolute_line <= total_lines {
                        let (display_num, style) = match self.line_number_mode {
                            LineNumberMode::Absolute => {
                                (current_absolute_line.to_string(), line_number_style)
                            }
                            LineNumberMode::Relative => {
                                if current_absolute_line == cursor_position.1 + 1 {
                                    (current_absolute_line.to_string(), current_line_style)
                                } else {
                                    let diff = ((cursor_position.1 + 1) as isize - current_absolute_line as isize).abs();
                                    (diff.to_string(), line_number_style)
                                }
                            }
                            LineNumberMode::Hybrid => {
                                if current_absolute_line == cursor_position.1 + 1 {
                                    (current_absolute_line.to_string(), current_line_style)
                                } else {
                                    let diff = ((cursor_position.1 + 1) as isize - current_absolute_line as isize).abs();
                                    (diff.to_string(), line_number_style)
                                }
                            }
                            LineNumberMode::Off => unreachable!(),
                        };
                        Spans::from(Span::styled(
                            format!("{:>width$} ", display_num, width = max_width),
                            style,
                        ))
                    } else {
                        Spans::from(Span::styled(
                            format!("{:>width$} ", "~", width = max_width),
                            line_number_style,
                        ))
                    }
                })
                .collect();

            let line_number_paragraph = Paragraph::new(line_numbers)
                .style(line_number_style)
                .alignment(Alignment::Right);

            f.render_widget(line_number_paragraph, line_number_area);
        }

        let syntax = self.ps.find_syntax_by_extension("rs")
            .or_else(|| self.ps.find_syntax_by_name(&self.syntax))
            .unwrap_or_else(|| self.ps.find_syntax_plain_text());
        let theme = &self.ts.themes["base16-ocean.dark"];
        let mut h = HighlightLines::new(syntax, theme);

        let visible_content = content.iter()
            .skip(scroll_offset)
            .take(editor_height)
            .enumerate();

        let mut text_spans = Vec::new();
        for (index, line) in visible_content {
            let absolute_line_index = scroll_offset + index;
            let ranges: Vec<(SyntectStyle, &str)> = h.highlight_line(line, &self.ps).unwrap();
            let mut current_line_styled_spans = Vec::new();
            let mut current_char_pos = 0;

            for (style, segment) in ranges {
                let color = style.foreground;
                let segment_len = segment.len();
                let segment_start = current_char_pos;
                let segment_end = segment_start + segment_len;

                let visible_start = horizontal_scroll.max(segment_start);
                let visible_end = (horizontal_scroll + editor_text_width).min(segment_end);

                if visible_start < visible_end {
                    let visible_segment_offset = visible_start - segment_start;
                    let visible_segment_len = visible_end - visible_start;
                    
                    let visible_text = Self::safe_slice(segment, visible_segment_offset, Some(visible_segment_offset + visible_segment_len));

                    current_line_styled_spans.push(Span::styled(
                        visible_text,
                        Style::default().fg(Color::Rgb(color.r, color.g, color.b)),
                    ));
                }
                current_char_pos += segment_len;
                if current_char_pos >= horizontal_scroll + editor_text_width {
                    break;
                }
            }

            if let (Some(start), Some(end)) = (self.mouse_selection_start, self.mouse_selection_end) {
                 if start != end {
                    let (selection_start, selection_end) = if start <= end { (start, end) } else { (end, start) };
                    if absolute_line_index >= selection_start.1 && absolute_line_index <= selection_end.1 {
                        let line_selection_start_col = if absolute_line_index == selection_start.1 { selection_start.0 } else { 0 };
                        let line_selection_end_col = if absolute_line_index == selection_end.1 { selection_end.0 } else { usize::MAX };

                        let visible_selection_start = line_selection_start_col.saturating_sub(horizontal_scroll);
                        let visible_selection_end = line_selection_end_col.saturating_sub(horizontal_scroll);

                        let mut highlighted_spans = Vec::new();
                        let mut current_col = 0;
                        for span in current_line_styled_spans {
                             let span_len = span.content.len();
                            let span_start_col = current_col;
                            let span_end_col = current_col + span_len;

                            let overlap_start = span_start_col.max(visible_selection_start);
                            let overlap_end = span_end_col.min(visible_selection_end);

                            if overlap_start < overlap_end {
                                if span_start_col < overlap_start {
                                    highlighted_spans.push(Span::styled(
                                        Self::safe_slice(&span.content, 0, Some(overlap_start - span_start_col)),
                                        span.style,
                                    ));
                                }
                                highlighted_spans.push(Span::styled(
                                    Self::safe_slice(&span.content, overlap_start - span_start_col, Some(overlap_end - span_start_col)),
                                     Style::default().bg(Color::DarkGray).fg(Color::White)
                                ));
                                if span_end_col > overlap_end {
                                     highlighted_spans.push(Span::styled(
                                        Self::safe_slice(&span.content, overlap_end - span_start_col, None),
                                        span.style,
                                    ));
                                }
                            } else {
                                highlighted_spans.push(span);
                            }
                            current_col += span_len;
                        }
                         current_line_styled_spans = highlighted_spans;
                    }
                 }
            }

            if let Some(match_pos) = maybe_match_pos {
                let cursor_bracket_visible = absolute_line_index == cursor_position.1 &&
                                             cursor_position.0 >= horizontal_scroll &&
                                             cursor_position.0 < horizontal_scroll + editor_text_width;
                let match_bracket_visible = absolute_line_index == match_pos.1 &&
                                            match_pos.0 >= horizontal_scroll &&
                                            match_pos.0 < horizontal_scroll + editor_text_width;

                if cursor_bracket_visible || match_bracket_visible {
                    let mut spans_with_brackets = Vec::new();
                    let mut current_col_offset = 0;
                    for span in current_line_styled_spans {
                        let span_len = span.content.len();
                        let span_start_abs = horizontal_scroll + current_col_offset;
                        let span_end_abs = span_start_abs + span_len;

                        let mut last_split = 0;
                        let mut modified = false;

                        if cursor_bracket_visible && absolute_line_index == cursor_position.1 &&
                           cursor_position.0 >= span_start_abs && cursor_position.0 < span_end_abs {
                            let bracket_offset = cursor_position.0 - span_start_abs;
                            if bracket_offset > last_split {
                                spans_with_brackets.push(Span::styled(Self::safe_slice(&span.content, last_split, Some(bracket_offset)), span.style));
                            }
                            spans_with_brackets.push(Span::styled(Self::safe_slice(&span.content, bracket_offset, Some(bracket_offset + 1)), bracket_match_style));
                            last_split = bracket_offset + 1;
                            modified = true;
                        }

                        if match_bracket_visible && absolute_line_index == match_pos.1 &&
                           match_pos.0 >= span_start_abs && match_pos.0 < span_end_abs {
                            let bracket_offset = match_pos.0 - span_start_abs;
                            if bracket_offset > last_split {
                                spans_with_brackets.push(Span::styled(Self::safe_slice(&span.content, last_split, Some(bracket_offset)), span.style));
                            }
                            if !(cursor_bracket_visible && cursor_position == match_pos) {
                                spans_with_brackets.push(Span::styled(Self::safe_slice(&span.content, bracket_offset, Some(bracket_offset + 1)), bracket_match_style));
                            }
                            last_split = bracket_offset + 1;
                            modified = true;
                        }

                        if modified {
                            if last_split < span.content.len() {
                                spans_with_brackets.push(Span::styled(Self::safe_slice(&span.content, last_split, None), span.style));
                            }
                        } else {
                            spans_with_brackets.push(span);
                        }
                        current_col_offset += span_len;
                    }
                    current_line_styled_spans = spans_with_brackets;
                }
            }

            if absolute_line_index == cursor_position.1 {
                 let mut spans_with_cursor = Vec::new();
                let mut current_len = 0;
                let cursor_col_in_view = cursor_position.0.saturating_sub(horizontal_scroll);

                for span in current_line_styled_spans {
                    let span_len = span.content.len();
                    let span_start = current_len;
                    let span_end = current_len + span_len;

                    if span_start <= cursor_col_in_view && cursor_col_in_view < span_end {
                        let cursor_offset = cursor_col_in_view - span_start;
                        let before = Self::safe_slice(&span.content, 0, Some(cursor_offset));
                        let after = Self::safe_slice(&span.content, cursor_offset, None);
                        
                        if !before.is_empty() {
                            spans_with_cursor.push(Span::styled(before, span.style));
                        }
                        spans_with_cursor.push(Span::styled("".to_string(), self.cursor_style));
                        if !after.is_empty() {
                            spans_with_cursor.push(Span::styled(after, span.style));
                        }
                    } else {
                         spans_with_cursor.push(span);
                    }
                    current_len += span_len;
                }
                 if cursor_col_in_view >= current_len {
                     spans_with_cursor.push(Span::styled("".to_string(), self.cursor_style));
                }
                 text_spans.push(Spans::from(spans_with_cursor));
            } else {
                 text_spans.push(Spans::from(current_line_styled_spans));
            }
        }

        let text_paragraph = Paragraph::new(text_spans)
            .style(Style::default().bg(ui::parse_color(&self.color_config.background)));
        f.render_widget(text_paragraph, text_content_area);

        if self.show_debug {
            let debug_messages: Vec<Spans> = self.debug_messages.iter().map(|m| Spans::from(m.clone())).collect();
            let debug_paragraph = Paragraph::new(debug_messages)
                .block(Block::default().borders(Borders::ALL).title("Debug Output"));
            f.render_widget(debug_paragraph, editor_layout[1]);
        }

        let bottom_bar_index = editor_layout.len() - 1;
        let bottom_bar_area = editor_layout[bottom_bar_index];
        let status_bar_style = Style::default()
            .fg(ui::parse_color(&self.color_config.foreground))
            .bg(ui::parse_color(&self.color_config.background));

        if self.mode == Mode::Command {
            let command_text = Spans::from(format!(":{}", self.command_buffer));
            let command_paragraph = Paragraph::new(vec![command_text]).style(status_bar_style);
            f.render_widget(command_paragraph, bottom_bar_area);
        } else if self.mode == Mode::Search {
            let search_text = Spans::from(format!("Search: {}", self.search_query));
            let search_paragraph = Paragraph::new(vec![search_text]).style(status_bar_style);
            f.render_widget(search_paragraph, bottom_bar_area);
        } else {
            let mode_str = match self.mode {
                Mode::Normal => "NORMAL",
                Mode::Insert => "INSERT",
                Mode::Visual => "VISUAL",
                _ => "",
            };

            let filename = active_tab.current_file.as_deref().unwrap_or("[No Name]");
            let branch = active_tab.git_branch.as_deref().unwrap_or("");
            let (cursor_col, cursor_line) = active_tab.cursor_position;

            let left_status = format!(" {} | {} ", mode_str, filename);
            let middle_status = format!(" {} ", branch);
            let right_status = format!(" {}:{} ", cursor_line + 1, cursor_col + 1);

            let total_width = bottom_bar_area.width as usize;
            let left_len = left_status.len();
            let middle_len = middle_status.len();
            let right_len = right_status.len();

            let padding = total_width.saturating_sub(left_len + middle_len + right_len);
            let left_padding = padding / 2;
            let right_padding = padding - left_padding;

            let status_line = Spans::from(vec![
                Span::styled(left_status, status_bar_style),
                Span::styled(" ".repeat(left_padding), status_bar_style),
                Span::styled(middle_status, status_bar_style.add_modifier(Modifier::BOLD)),
                Span::styled(" ".repeat(right_padding), status_bar_style),
                Span::styled(right_status, status_bar_style),
            ]);

            let status_paragraph = Paragraph::new(status_line);
            f.render_widget(status_paragraph, bottom_bar_area);
        }

        let relative_cursor_x = cursor_position.0.saturating_sub(horizontal_scroll) as u16;
        let relative_cursor_y = cursor_position.1.saturating_sub(scroll_offset) as u16;

        let absolute_cursor_x = text_content_area.x + relative_cursor_x;
        let absolute_cursor_y = text_content_area.y + relative_cursor_y;

        let clamped_absolute_cursor_x = absolute_cursor_x.min(text_content_area.right().saturating_sub(1));
        let clamped_absolute_cursor_y = absolute_cursor_y.min(text_content_area.bottom().saturating_sub(1));


        f.set_cursor(
            clamped_absolute_cursor_x,
            clamped_absolute_cursor_y
        );

        if self.show_minimap && !active_tab.content.is_empty() && current_layout_index < main_layout.len() {
            let minimap_area = Rect::new(
                 editor_area.right(),
                editor_area.y,
                self.minimap_width,
                 editor_area.height
            );
             let clipped_minimap_area = minimap_area.intersection(f.size());
             if clipped_minimap_area.width > 0 && clipped_minimap_area.height > 0 {
                 self.render_minimap(f, clipped_minimap_area);
             }
        }
    }

    fn enter_search_mode(&mut self) {
        self.mode = Mode::Search;
        self.search_query.clear();
        self.search_results.clear();
        self.current_search_index = 0;
    }

    fn perform_search(&mut self) {
        self.search_results.clear();
        let tab = &self.tabs[self.active_tab];
        for (line_num, line) in tab.content.iter().enumerate() {
            if let Some(col) = line.to_lowercase().find(&self.search_query.to_lowercase()) {
                self.search_results.push((line_num, col));
            }
        }
        self.current_search_index = 0;
        if !self.search_results.is_empty() {
            let (line, col) = self.search_results[0];
            let tab = &mut self.tabs[self.active_tab];
            tab.cursor_position = (col, line);
        }
    }

    fn next_search_result(&mut self) {
        if !self.search_results.is_empty() {
            self.current_search_index = (self.current_search_index + 1) % self.search_results.len();
            let (line, col) = self.search_results[self.current_search_index];
            let tab = &mut self.tabs[self.active_tab];
            tab.cursor_position = (col, line);
        }
    }

    fn previous_search_result(&mut self) {
        if !self.search_results.is_empty() {
            self.current_search_index = (self.current_search_index + self.search_results.len() - 1) % self.search_results.len();
            let (line, col) = self.search_results[self.current_search_index];
            let tab = &mut self.tabs[self.active_tab];
            tab.cursor_position = (col, line);
        }
    }

    fn handle_search_mode(&mut self, key: KeyEvent) -> io::Result<bool> {
        let key_str = Self::key_event_to_string(key);
        
        if let Some(action) = self.keybindings.search_mode.get(&key_str).cloned() {
            self.execute_action(&action)
        } else {
            match key.code {
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                }
                _ => {}
            }
            Ok(false)
        }
    }

    fn find_matching_bracket(&self, position: (usize, usize)) -> Option<(usize, usize)> {
        let tab = &self.tabs[self.active_tab];
        let (x, y) = position;
    
        if y >= tab.content.len() || x >= tab.content[y].len() {
            return None;
        }
    
        let current_char = tab.content[y].chars().nth(x)?;
        let (expected_match, search_forward) = match current_char {
            '(' => (')', true),
            ')' => ('(', false),
            '[' => (']', true),
            ']' => ('[', false),
            '{' => ('}', true),
            '}' => ('{', false),
            _ => return None,
        };
    
        let mut level = 0;
        let mut current_pos = position;
    
        if search_forward {
            current_pos.0 += 1;
            for line_idx in current_pos.1..tab.content.len() {
                let line = &tab.content[line_idx];
                let start_col = if line_idx == current_pos.1 { current_pos.0 } else { 0 };
                for (col_idx, char) in line.chars().enumerate().skip(start_col) {
                    if char == current_char {
                        level += 1;
                    } else if char == expected_match {
                        if level == 0 {
                            return Some((col_idx, line_idx));
                        } else {
                            level -= 1;
                        }
                    }
                }
            }
        } else {
            if current_pos.0 == 0 {
                if current_pos.1 == 0 {
                     return None;
                }
                current_pos.1 -= 1;
                current_pos.0 = tab.content[current_pos.1].len();
            } else {
                current_pos.0 -= 1;
            }

            for line_idx in (0..=current_pos.1).rev() {
                let line = &tab.content[line_idx];
                let end_col = if line_idx == current_pos.1 { current_pos.0 + 1 } else { line.len() };
                let chars_in_range: Vec<(usize, char)> = line.chars()
                                                             .enumerate()
                                                             .take(end_col)
                                                             .collect();
                for (col_idx, char) in chars_in_range.into_iter().rev() {
                    if char == current_char {
                        level += 1;
                    } else if char == expected_match {
                        if level == 0 {
                            return Some((col_idx, line_idx));
                        } else {
                            level -= 1;
                        }
                    }
                }
            }
        }
    
        None
    }

    fn safe_slice(s: &str, start_byte: usize, end_byte: Option<usize>) -> String {
        let char_indices: Vec<_> = s.char_indices().collect();
        
        if char_indices.is_empty() {
            return String::new();
        }
        
        let start_char_idx = char_indices.iter()
            .position(|(byte_idx, _)| *byte_idx >= start_byte)
            .unwrap_or(char_indices.len());
            
        let end_char_idx = if let Some(end) = end_byte {
            char_indices.iter()
                .position(|(byte_idx, _)| *byte_idx > end)
                .unwrap_or(char_indices.len())
        } else {
            char_indices.len()
        };
        
        if start_char_idx < end_char_idx && start_char_idx < char_indices.len() {
            let start_byte_pos = char_indices[start_char_idx].0;
            let end_byte_pos = if end_char_idx < char_indices.len() {
                char_indices[end_char_idx].0
            } else {
                s.len()
            };
            
            s[start_byte_pos..end_byte_pos].to_string()
        } else {
            String::new()
        }
    }
} 