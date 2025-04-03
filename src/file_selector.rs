use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tui::{
    backend::Backend,
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use crate::config::ColorConfig;
use crate::ui;

pub struct FileSelector {
    pub current_dir: PathBuf,
    pub entries: Vec<PathBuf>,
    pub selected_index: usize,
    pub parent_dir_index: Option<usize>,
}

impl FileSelector {
    pub fn new(path: &Path) -> io::Result<Self> {
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

    pub fn up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn down(&mut self) {
        if self.selected_index < self.entries.len() - 1 {
            self.selected_index += 1;
        }
    }

    pub fn enter(&mut self) -> io::Result<Option<PathBuf>> {
        if self.selected_index < self.entries.len() {
            let selected = &self.entries[self.selected_index];
            if selected.is_dir() {
                self.current_dir = selected.canonicalize()?;
                self.entries = vec![self.current_dir.join("..").canonicalize()?];
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

    pub fn render<B: Backend>(&self, f: &mut Frame<B>, area: Rect, color_config: &ColorConfig) {
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
                .border_style(Style::default().fg(ui::parse_color(&color_config.file_selector_border))))
            .style(Style::default()
                .bg(ui::parse_color(&color_config.file_selector_background))
                .fg(ui::parse_color(&color_config.file_selector_foreground)))
            .highlight_style(
                Style::default()
                    .bg(ui::parse_color(&color_config.file_selector_highlight))
                    .add_modifier(Modifier::BOLD),
            );

        let mut state = ListState::default();
        state.select(Some(self.selected_index));
        f.render_stateful_widget(list, area, &mut state);
    }
} 