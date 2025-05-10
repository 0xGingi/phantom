use std::collections::VecDeque;
use std::fs;
use std::io;
use std::path::Path;
use syntect::parsing::SyntaxSet;
use git2::{Repository, Status};

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
    pub git_status: Option<Status>,
    pub git_branch: Option<String>,
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
            git_status: None,
            git_branch: None,
        }
    }

    pub fn from_file(path: &Path, ps: &SyntaxSet) -> io::Result<Self> {
        let content = if path.exists() {
            fs::read_to_string(path)?
        } else {
            String::new()
        };
        
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

        let (git_status, git_branch) = Self::get_git_info(path);

        let tab = Tab {
            content: lines,
            cursor_position: (0, 0),
            scroll_offset: 0,
            horizontal_scroll: 0,
            current_file: Some(path.to_string_lossy().into_owned()),
            syntax,
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            git_status,
            git_branch,
        };
        Ok(tab)
    }

    pub fn get_git_info(path: &Path) -> (Option<Status>, Option<String>) {
        let repo_result = Repository::discover(path);
        if repo_result.is_err() {
            return (None, None);
        }
        let repo = repo_result.unwrap();

        let status = if !path.exists() {
            Some(Status::WT_NEW)
        } else {
             match repo.workdir() {
                Some(workdir) => {
                    let relative_path = path.strip_prefix(workdir).ok();
                    relative_path.and_then(|p| repo.status_file(p).ok())
                },
                None => None,
            }
        };

        let branch = match repo.head() {
            Ok(head) => {
                if head.is_branch() {
                    head.shorthand().map(String::from)
                } else {
                    None
                }
            },
            Err(_) => None,
        };

        (status, branch)
    }

    pub fn adjust_horizontal_scroll(&mut self, editor_width: usize) {
        if self.cursor_position.0 < self.horizontal_scroll {
            self.horizontal_scroll = self.cursor_position.0;
        } else if self.cursor_position.0 >= self.horizontal_scroll + editor_width {
            self.horizontal_scroll = self.cursor_position.0 - editor_width + 1;
        }
    }
} 