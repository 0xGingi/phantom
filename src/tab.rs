use std::collections::VecDeque;
use std::fs;
use std::io;
use std::path::Path;
use syntect::parsing::SyntaxSet;

#[derive(Clone)]
pub struct EditOperation {
    pub content: Vec<String>,
    pub cursor_position: (usize, usize),
    pub scroll_offset: usize,
    pub horizontal_scroll: usize,
}

pub struct Tab {
    pub content: Vec<String>,
    pub cursor_position: (usize, usize),
    pub scroll_offset: usize,
    pub horizontal_scroll: usize,
    pub current_file: Option<String>,
    pub syntax: String,
    pub undo_stack: VecDeque<EditOperation>,
    pub redo_stack: VecDeque<EditOperation>,
}

impl Tab {
    pub fn new() -> Self {
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

    pub fn from_file(path: &Path, ps: &SyntaxSet) -> io::Result<Self> {
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

    pub fn adjust_horizontal_scroll(&mut self, editor_width: usize) {
        if self.cursor_position.0 < self.horizontal_scroll {
            self.horizontal_scroll = self.cursor_position.0;
        } else if self.cursor_position.0 >= self.horizontal_scroll + editor_width {
            self.horizontal_scroll = self.cursor_position.0 - editor_width + 1;
        }
    }
} 