use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseEventKind, MouseButton, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{error::Error, io};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::env;
use std::fmt;
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph, List, ListItem, ListState, Tabs},
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
struct ColorConfig {
    background: String,
    foreground: String,
    cursor: String,
    selection: String,
    comment: String,
    keyword: String,
    string: String,
    function: String,
    number: String,
    minimap_highlight: String,
    minimap_background: String,
    minimap_content: String,
    minimap_border: String,
    tab_active: String,
    tab_inactive: String,
    tab_background: String,
    file_selector_background: String,
    file_selector_foreground: String,
    file_selector_highlight: String,
    file_selector_border: String,
}

#[derive(Deserialize, Serialize, Clone)]
struct Keybindings {
    normal_mode: HashMap<String, String>,
    insert_mode: HashMap<String, String>,
    visual_mode: HashMap<String, String>,
    command_mode: HashMap<String, String>,
    file_select_mode: HashMap<String, String>,
    search_mode: HashMap<String, String>,
    tab_mode: HashMap<String, String>,

}

#[derive(Clone)]
struct EditOperation {
    content: Vec<String>,
    cursor_position: (usize, usize),
    scroll_offset: usize,
    horizontal_scroll: usize,
}

struct Tab {
    content: Vec<String>,
    cursor_position: (usize, usize),
    scroll_offset: usize,
    horizontal_scroll: usize,
    current_file: Option<String>,
    syntax: String,
    undo_stack: VecDeque<EditOperation>,
    redo_stack: VecDeque<EditOperation>,
}

struct DummyClipboard;

impl DummyClipboard {
    fn new() -> Result<Self, Box<dyn Error>> {
        Ok(DummyClipboard)
    }

    fn get_contents(&mut self) -> Result<String, Box<dyn Error>> {
        Ok(String::new())
    }

    fn set_contents(&mut self, _: String) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

enum ClipboardWrapper {
    Real(ClipboardContext),
    Dummy(DummyClipboard),
}

impl ClipboardWrapper {
    fn new() -> Self {
        match ClipboardProvider::new() {
            Ok(clipboard) => ClipboardWrapper::Real(clipboard),
            Err(_) => ClipboardWrapper::Dummy(DummyClipboard::new().unwrap()),
        }
    }

    fn get_contents(&mut self) -> Result<String, Box<dyn Error>> {
        match self {
            ClipboardWrapper::Real(clipboard) => clipboard.get_contents().map_err(|e| e.into()),
            ClipboardWrapper::Dummy(dummy) => dummy.get_contents(),
        }
    }

    fn set_contents(&mut self, contents: String) -> Result<(), Box<dyn Error>> {
        match self {
            ClipboardWrapper::Real(clipboard) => clipboard.set_contents(contents).map_err(|e| e.into()),
            ClipboardWrapper::Dummy(dummy) => dummy.set_contents(contents),
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Mode::Normal => write!(f, "Normal"),
            Mode::Insert => write!(f, "Insert"),
            Mode::Visual => write!(f, "Visual"),
            Mode::Command => write!(f, "Command"),
            Mode::Search => write!(f, "Search"),
            Mode::FileSelect => write!(f, "FileSelect"),
            Mode::DirectoryNav => write!(f, "DirectoryNav"),
            Mode::SidebarActive => write!(f, "SidebarActive"),
        }
    }
}

impl Tab {
    fn new() -> Self {
        Tab {
            content: vec![String::new()],
            cursor_position: (0, 0),
            scroll_offset: 0,
            horizontal_scroll: 0,
            current_file: None,
            syntax: "Plain Text".to_string(),
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
        }
    }

    fn from_file(path: &Path, ps: &SyntaxSet) -> io::Result<Self> {
        let content = fs::read_to_string(path)?;
        let lines = if content.is_empty() {
            vec![String::new()]
        } else {
            content.lines().map(String::from).collect()
        };

        let mut syntax = "Plain Text".to_string();
        if let Some(extension) = path.extension() {
            if let Some(ext_str) = extension.to_str() {
                if let Some(s) = ps.find_syntax_by_extension(ext_str) {
                    syntax = s.name.clone();
                }
            }
        }

        let tab = Tab {
            content: lines,
            cursor_position: (0, 0),
            scroll_offset: 0,
            horizontal_scroll: 0,
            current_file: Some(path.to_string_lossy().into_owned()),
            syntax,
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
        };
        Ok(tab)
    }

    fn adjust_horizontal_scroll(&mut self) {
        let editor_width = 80;
        if self.cursor_position.0 < self.horizontal_scroll {
            self.horizontal_scroll = self.cursor_position.0;
        } else if self.cursor_position.0 >= self.horizontal_scroll + editor_width {
            self.horizontal_scroll = self.cursor_position.0 - editor_width + 1;
        }
    }
}

impl ColorConfig {
    fn default() -> Self {
        ColorConfig {
            background: "#1E1E1E".to_string(),
            foreground: "#CCCCCC".to_string(),
            cursor: "#FFFFFF".to_string(),
            selection: "#264F78".to_string(),
            comment: "#7F848E".to_string(),
            keyword: "#61AFEF".to_string(),
            string: "#C678DD".to_string(),
            function: "#E5C07B".to_string(),
            number: "#D19A66".to_string(),
            minimap_highlight: "#264F78".to_string(),
            minimap_background: "#1E1E1E".to_string(),
            minimap_content: "#404040".to_string(),
            minimap_border: "#404040".to_string(),
            tab_active: "#61AFEF".to_string(),
            tab_inactive: "#7F848E".to_string(),
            tab_background: "#252526".to_string(),
            file_selector_background: "#2C2C2C".to_string(),
            file_selector_foreground: "#CCCCCC".to_string(),
            file_selector_highlight: "#3A3D41".to_string(),
            file_selector_border: "#4A4A4A".to_string(),
        }
    }

    fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

impl Keybindings {
    fn default() -> Self {
        Keybindings {
            normal_mode: [
                ("dd".to_string(), "delete_line".to_string()),
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
                ("Ctrl+e".to_string(), "toggle_sidebar".to_string()),
                ("/".to_string(), "enter_search_mode".to_string()),
                ("n".to_string(), "next_search_result".to_string()),
                ("N".to_string(), "previous_search_result".to_string()),
                ("Ctrl+y".to_string(), "copy_selection".to_string()),
                ("Ctrl+p".to_string(), "paste_clipboard".to_string()),
                ("Ctrl+u".to_string(), "undo".to_string()),
                ("Ctrl+r".to_string(), "redo".to_string()),
                ("Tab".to_string(), "next_tab".to_string()),
                ("F1".to_string(), "switch_to_tab_1".to_string()),
                ("F2".to_string(), "switch_to_tab_2".to_string()),
                ("F3".to_string(), "switch_to_tab_3".to_string()),
                ("F4".to_string(), "switch_to_tab_4".to_string()),
                ("F5".to_string(), "switch_to_tab_5".to_string()),
                ("F6".to_string(), "switch_to_tab_6".to_string()),
                ("F7".to_string(), "switch_to_tab_7".to_string()),
                ("F8".to_string(), "switch_to_tab_8".to_string()),
                ("F9".to_string(), "switch_to_tab_9".to_string()),
                ("Ctrl+t".to_string(), "new_tab".to_string()),
                ("Ctrl+w".to_string(), "close_tab".to_string()),
                ("Ctrl+Shift+Tab".to_string(), "previous_tab".to_string()),
                ("Ctrl+m".to_string(), "toggle_minimap".to_string()),
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
            tab_mode: [
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
    SidebarActive,
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

    fn render<B: tui::backend::Backend>(&self, f: &mut Frame<B>, area: Rect, color_config: &ColorConfig) {
        let items: Vec<ListItem> = self.entries
            .iter()
            .enumerate()
            .map(|(index, path)| {
                let name = if Some(index) == self.parent_dir_index {
                    ".. (Parent Directory)".to_string()
                } else {
                    path.file_name().unwrap_or_default().to_string_lossy().into_owned()
                };
                
                let icon = if path.is_dir() {
                    "📁"
                } else {
                    match path.extension().and_then(|s| s.to_str()) {
                        Some("rs") => "🦀",
                        Some("js") => "🟨",
                        Some("py") => "🐍",
                        Some("html") => "🌐",
                        Some("css") => "🎨",
                        Some("json") => "📊",
                        Some("md") => "📝",
                        Some("txt") => "📄",
                        Some("pdf") => "📕",
                        Some("jpg") | Some("jpeg") | Some("png") | Some("gif") => "🖼️",
                        Some("mp3") | Some("wav") | Some("ogg") => "🎵",
                        Some("mp4") | Some("avi") | Some("mov") => "🎬",
                        Some("zip") | Some("tar") | Some("gz") => "🗜️",
                        Some("exe") | Some("msi") => "⚙️",
                        _ => "📄",
                    }
                };
                
                ListItem::new(format!("{} {}", icon, name))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().title("File Selector").borders(Borders::ALL)
                .border_style(Style::default().fg(Editor::parse_color(&color_config.file_selector_border))))
            .style(Style::default()
                .bg(Editor::parse_color(&color_config.file_selector_background))
                .fg(Editor::parse_color(&color_config.file_selector_foreground)))
            .highlight_style(
                Style::default()
                    .bg(Editor::parse_color(&color_config.file_selector_highlight))
                    .add_modifier(Modifier::BOLD),
            );

        let mut state = ListState::default();
        state.select(Some(self.selected_index));
        f.render_stateful_widget(list, area, &mut state);
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
    clipboard_context: ClipboardWrapper,
    visual_start: (usize, usize),
    file_selector: Option<FileSelector>,
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
}

impl Editor {
    fn new() -> Self {
        let keybindings = Self::load_config().unwrap_or_else(|_| Keybindings::default());
        let color_config = Self::load_color_config().unwrap_or_else(|_| ColorConfig::default());
        let clipboard_context = ClipboardWrapper::new();
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
        }
    }

    fn is_minimap_area(&self, x: u16, y: u16) -> bool {
        let minimap_x = self.get_editor_width() as u16;
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
        let editor_height = self.get_editor_height();
        let new_scroll_offset = new_cursor_line.saturating_sub(editor_height / 2);
    
        let tab = &mut self.tabs[self.active_tab];
        tab.cursor_position.1 = new_cursor_line;
        tab.scroll_offset = new_scroll_offset;
    
        self.ensure_cursor_visible();
    }

    fn ensure_cursor_visible(&mut self) {
        let editor_height = self.get_editor_height();
        let tab = &mut self.tabs[self.active_tab];

        if tab.cursor_position.1 < tab.scroll_offset {
            tab.scroll_offset = tab.cursor_position.1;
        } else if tab.cursor_position.1 >= tab.scroll_offset + editor_height {
            tab.scroll_offset = tab.cursor_position.1 - editor_height + 1;
        }
    }

    fn toggle_minimap(&mut self) -> io::Result<bool> {
        if !self.show_minimap {
            if !self.tabs[self.active_tab].content.iter().all(|line| line.is_empty()) {
                self.show_minimap = true;
                self.debug_messages.push("Minimap shown".to_string());
            } else {
                self.debug_messages.push("Cannot show minimap: No content".to_string());
            }
        } else {
            self.show_minimap = false;
            self.debug_messages.push("Minimap hidden".to_string());
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
                    .bg(Self::parse_color(&self.color_config.minimap_background))
                    .fg(Self::parse_color(&self.color_config.minimap_content)));
            f.render_widget(empty_minimap, area);
            return;
        }
    
        let total_lines = content.len();
        let minimap_height = area.height as usize - 2;
        let minimap_width = (area.width as usize - 2) * 2;
    
        let scale_y = (total_lines as f32 / minimap_height as f32).max(1.0);
        let scale_x = 4;
    
        let background_color = Self::parse_color(&self.color_config.minimap_background);
        let foreground_color = Self::parse_color(&self.color_config.minimap_content);
        let comment_color = Self::parse_color(&self.color_config.comment);
        let keyword_color = Self::parse_color(&self.color_config.keyword);
        let string_color = Self::parse_color(&self.color_config.string);
        let function_color = Self::parse_color(&self.color_config.function);
        let minimap_highlight_color = Self::parse_color(&self.color_config.minimap_highlight);
    
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
                .border_style(Style::default().fg(Self::parse_color(&self.color_config.minimap_border))))
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

    fn with_file(path: &Path) -> io::Result<Self> {
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

    fn parse_color(color_str: &str) -> Color {
        if let Ok(rgb) = u32::from_str_radix(&color_str[1..], 16) {
            Color::Rgb(
                ((rgb >> 16) & 0xFF) as u8,
                ((rgb >> 8) & 0xFF) as u8,
                (rgb & 0xFF) as u8,
            )
        } else {
            Color::Reset
        }
    }
    
    fn load_color_config() -> Result<ColorConfig, Box<dyn Error>> {
        let config_dir = dirs::home_dir()
            .ok_or("Could not find home directory")?
            .join(".config")
            .join("phantom");
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
            KeyCode::Char(c) => key_string.push(c),
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
                Path::new(file).parent().unwrap_or(Path::new(".")).to_path_buf()
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
                match key.code {
                    KeyCode::Left => self.move_cursor_left(),
                    KeyCode::Down => self.move_cursor_down(),
                    KeyCode::Up => self.move_cursor_up(),
                    KeyCode::Right => self.move_cursor_right(),
                    KeyCode::Home => self.move_cursor_start_of_line(),
                    KeyCode::End => self.move_cursor_end_of_line(),
                    KeyCode::PageUp => self.page_up(),
                    KeyCode::PageDown => self.page_down(),
                    KeyCode::Tab => {
                        self.next_tab();
                        self.update_current_tab_info();
                    },
                    KeyCode::BackTab => {
                        self.previous_tab();
                        self.update_current_tab_info();
                    },
                    _ => {},
                }
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
        match key.code {
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Enter => self.insert_newline(),
            KeyCode::Backspace => self.backspace(),
            KeyCode::Delete => self.delete_char(),
            KeyCode::Left => self.move_cursor_left(),
            KeyCode::Down => self.move_cursor_down(),
            KeyCode::Up => self.move_cursor_up(),
            KeyCode::Right => self.move_cursor_right(),
            KeyCode::Char(c) => self.insert_char(c),
            _ => {}
        }
        Ok(false)
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
    
    fn handle_file_select_mode(&mut self, key: KeyEvent) -> io::Result<bool> {
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
        let editor_height = self.get_editor_height();
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
            tab.adjust_horizontal_scroll();
        }
    }

    fn move_cursor_right(&mut self) {
        let tab = &mut self.tabs[self.active_tab];
        if tab.cursor_position.0 < tab.content[tab.cursor_position.1].len() {
            tab.cursor_position.0 += 1;
            tab.adjust_horizontal_scroll();
        } else if tab.cursor_position.1 < tab.content.len() - 1 {
            tab.cursor_position.1 += 1;
            tab.cursor_position.0 = 0;
            tab.horizontal_scroll = 0;
        }
    }
    
    fn get_editor_height(&self) -> usize {
        24
    }

    fn move_cursor_start_of_line(&mut self) {
        let tab = &mut self.tabs[self.active_tab];
        tab.cursor_position.0 = 0;
        tab.adjust_horizontal_scroll();
    }

    fn move_cursor_end_of_line(&mut self) {
        let tab = &mut self.tabs[self.active_tab];
        tab.cursor_position.0 = tab.content[tab.cursor_position.1].len();
        tab.adjust_horizontal_scroll();
    }

    fn insert_char(&mut self, c: char) {
        self.save_state();
        let tab = &mut self.tabs[self.active_tab];
        let line = &mut tab.content[tab.cursor_position.1];
        line.insert(tab.cursor_position.0, c);
        tab.cursor_position.0 += 1;
        tab.adjust_horizontal_scroll();
    }

    fn insert_newline(&mut self) {
        self.save_state();
        let tab = &mut self.tabs[self.active_tab];
        let current_line = &mut tab.content[tab.cursor_position.1];
        let rest_of_line = current_line.split_off(tab.cursor_position.0);
        tab.content.insert(tab.cursor_position.1 + 1, rest_of_line);
        tab.cursor_position = (0, tab.cursor_position.1 + 1);
    }

    fn page_up(&mut self) {
        let visible_lines = self.get_editor_height();
        let tab = &mut self.tabs[self.active_tab];
        if tab.scroll_offset > visible_lines {
            tab.scroll_offset -= visible_lines;
        } else {
            tab.scroll_offset = 0;
        }
        tab.cursor_position.1 = tab.scroll_offset;
    }
    
    fn page_down(&mut self) {
        let visible_lines = self.get_editor_height();
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
        let tab = &mut self.tabs[self.active_tab];
        let filename = if let Some(name) = filename {
            name.to_path_buf()
        } else if let Some(ref name) = tab.current_file {
            PathBuf::from(name)
        } else {
            return Err(io::Error::new(io::ErrorKind::Other, "No filename specified. Use :w <filename> to save."));
        };
    
        if let Some(parent) = filename.parent() {
            fs::create_dir_all(parent)?;
        }
    
        let mut file = fs::File::create(&filename)?;
        for line in &tab.content {
            writeln!(file, "{}", line)?;
        }
        tab.current_file = Some(filename.to_string_lossy().into_owned());
        self.update_tab_name();
        self.debug_messages.push(format!("File saved: {}", filename.display()));
        Ok(())
    }

    fn open_file(&mut self, path: &Path) -> io::Result<()> {
        let new_tab = if path.exists() {
            Tab::from_file(path, &self.ps)?
        } else {
            let mut tab = Tab::new();
            tab.current_file = Some(path.to_string_lossy().into_owned());
            tab
        };
    
        if self.tabs.len() == 1 && self.tabs[0].content == vec![String::new()] && self.tabs[0].current_file.is_none() {
            self.tabs[0] = new_tab;
            self.active_tab = 0;
        } else {
            self.tabs.push(new_tab);
            self.active_tab = self.tabs.len() - 1;
        }
        
        self.update_tab_name();
        
        if path.exists() {
            self.debug_messages.push(format!("File opened: {}", path.display()));
        } else {
            self.debug_messages.push(format!("New file: {} (not yet saved)", path.display()));
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
        let total_width = f.size().width;
        let sidebar_width = if self.show_sidebar { self.sidebar_width } else { 0 };
        let minimap_width = if self.show_minimap && !self.tabs[self.active_tab].content.is_empty() { self.minimap_width } else { 0 };
        let editor_width = total_width.saturating_sub(sidebar_width + minimap_width);
        
        let mut constraints = vec![];
        if sidebar_width > 0 {
            constraints.push(Constraint::Length(sidebar_width));
        }
        constraints.push(Constraint::Length(editor_width));
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
        let debug_height = if self.show_debug { 6 } else { 0 };
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
                let title = tab.current_file.as_ref()
                    .and_then(|f| Path::new(f).file_name())
                    .and_then(|f| f.to_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("Untitled-{}", i + 1));
        
                let style = if i == self.active_tab {
                    Style::default().fg(Self::parse_color(&self.color_config.tab_active))
                } else {
                    Style::default().fg(Self::parse_color(&self.color_config.tab_inactive))
                };
                Spans::from(vec![
                    Span::styled(format!(" {} ", i + 1), style),
                    Span::styled(title, style),
                    Span::raw(" "),
                ])
            }).collect();
        
        let tab_bar = Tabs::new(tab_titles)
            .block(Block::default().borders(Borders::ALL).title("Tabs"))
            .select(self.active_tab)
            .style(Style::default().bg(Self::parse_color(&self.color_config.tab_background)))
            .highlight_style(Style::default().fg(Self::parse_color(&self.color_config.tab_active)));
    
        f.render_widget(tab_bar, editor_layout[0]);

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
                    .fg(Self::parse_color(&self.color_config.foreground))
                    .add_modifier(Modifier::BOLD),
            ));
    
        let syntax = self.ps.find_syntax_by_extension("rs")
            .or_else(|| self.ps.find_syntax_by_name(&self.syntax))
            .unwrap_or_else(|| self.ps.find_syntax_plain_text());
    
        let theme = &self.ts.themes["base16-ocean.dark"];
        let _background_color = Self::parse_color(&self.color_config.background);
        let _foreground_color = Self::parse_color(&self.color_config.foreground);
    
        let mut h = HighlightLines::new(syntax, theme);
    
        let editor_chunk_index = if self.show_debug { 2 } else { 1 };
        let editor_height = editor_layout[editor_chunk_index].height as usize - 2;
        let editor_width = self.get_editor_width();
    
        let active_tab = &self.tabs[self.active_tab];
        let content = &active_tab.content;
        let cursor_position = active_tab.cursor_position;
        let scroll_offset = active_tab.scroll_offset;
        let horizontal_scroll = active_tab.horizontal_scroll;
    
        let visible_content = content.iter()
            .skip(scroll_offset)
            .take(editor_height)
            .enumerate();
        
        let mut text = Vec::new();
        for (index, line) in visible_content {
            let ranges: Vec<(SyntectStyle, &str)> = h.highlight_line(line, &self.ps).unwrap();
            let mut styled_spans = Vec::new();
            let mut line_length = 0;
            for (style, content) in ranges {
                let color = style.foreground;
                let visible_content = if line_length >= horizontal_scroll {
                    content
                } else if line_length + content.len() > horizontal_scroll {
                    &content[horizontal_scroll - line_length..]
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
                if line_length >= horizontal_scroll + editor_width {
                    break;
                }
            }
    
            if let (Some(start), Some(end)) = (self.mouse_selection_start, self.mouse_selection_end) {
                if start != end {
                    let (start, end) = if start <= end { (start, end) } else { (end, start) };
                    let y = index + scroll_offset;
                    if y >= start.1 && y <= end.1 {
                        let start_x = if y == start.1 { start.0.saturating_sub(horizontal_scroll) } else { 0 };
                        let end_x = if y == end.1 { end.0.saturating_sub(horizontal_scroll) } else { editor_width };
                        
                        styled_spans = styled_spans.into_iter().enumerate().flat_map(|(i, span)| {
                            let mut result = Vec::new();
                            let span_start = i;
                            let span_end = span_start + span.content.len();
    
                            if span_end <= start_x || span_start >= end_x {
                                vec![span]
                            } else {
                                if span_start < start_x {
                                    result.push(Span::styled(
                                        span.content[..(start_x - span_start).min(span.content.len())].to_string(),
                                        span.style
                                    ));
                                }
                                let highlight_start = start_x.saturating_sub(span_start);
                                let highlight_end = (end_x.saturating_sub(span_start)).min(span.content.len());
                                if highlight_start < highlight_end {
                                    result.push(Span::styled(
                                        span.content[highlight_start..highlight_end].to_string(),
                                        Style::default().bg(Color::Gray).fg(Color::Black)
                                    ));
                                }
                                if span_end > end_x {
                                    result.push(Span::styled(
                                        span.content[(end_x.saturating_sub(span_start))..].to_string(),
                                        span.style
                                    ));
                                }
                                result
                            }
                        }).collect();
                    }
                }
            }
                                            
            if index + scroll_offset == cursor_position.1 {
                let mut line_spans = Vec::new();
                let mut current_len = 0;
                for span in styled_spans {
                    let span_len = span.content.len();
                    if current_len <= cursor_position.0 - horizontal_scroll && cursor_position.0 - horizontal_scroll < current_len + span_len {
                        let (before, after) = span.content.split_at(cursor_position.0 - horizontal_scroll - current_len);
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
                if cursor_position.0 - horizontal_scroll >= current_len {
                    line_spans.push(Span::styled("".to_string(), self.cursor_style));
                }
                text.push(Spans::from(line_spans));
            } else {
                text.push(Spans::from(styled_spans));
            }
        }
            
        let paragraph = Paragraph::new(text)
            .block(block)
            .style(Style::default().bg(Self::parse_color(&self.color_config.background)));
        f.render_widget(paragraph, editor_layout[editor_chunk_index]);
    
        if self.show_debug {
            let debug_messages: Vec<Spans> = self.debug_messages.iter().map(|m| Spans::from(m.clone())).collect();
            let debug_paragraph = Paragraph::new(debug_messages)
                .block(Block::default().borders(Borders::ALL).title("Debug Output"));
            f.render_widget(debug_paragraph, editor_layout[1]);
        }
    
        if self.mode == Mode::Command {
            let command_text = Spans::from(format!(":{}", self.command_buffer));
            let command_paragraph = Paragraph::new(vec![command_text]);
            f.render_widget(command_paragraph, editor_layout[editor_layout.len() - 1]);
        } else if self.mode == Mode::Search {
            let search_text = Spans::from(format!("Search: {}", self.search_query));
            let search_paragraph = Paragraph::new(vec![search_text]);
            f.render_widget(search_paragraph, editor_layout[editor_layout.len() - 1]);
        }
    
        let cursor_x = (cursor_position.0 - horizontal_scroll) as u16 + 1 + if self.show_sidebar { self.sidebar_width } else { 0 };
        let cursor_y = (cursor_position.1 - scroll_offset) as u16 + 1 + tab_bar_height + debug_height;
    
        let max_y = editor_layout[editor_chunk_index].height.saturating_sub(1);
        let cursor_y = cursor_y.min(max_y);
    
        let adjusted_cursor_x = cursor_x;
        let adjusted_cursor_y = cursor_y;
    
        f.set_cursor(
            adjusted_cursor_x.min(editor_area.width.saturating_sub(1)),
            adjusted_cursor_y
        );

        if self.show_minimap && !self.tabs[self.active_tab].content.is_empty() && current_layout_index < main_layout.len() {
            self.render_minimap(f, main_layout[current_layout_index]);
        }
        
        if self.show_minimap {
            let minimap_area = Rect::new(
                editor_area.right(),
                editor_area.top(),
                self.minimap_width,
                editor_area.height
            );
            self.render_minimap(f, minimap_area);
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
