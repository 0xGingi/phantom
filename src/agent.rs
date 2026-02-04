use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize, Child};
use vt100;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::config::ColorConfig;
use crate::ui;

#[derive(Clone)]
pub struct AgentDefinition {
    pub name: String,
    pub command: Vec<String>,
}

impl AgentDefinition {
    pub fn defaults() -> Vec<Self> {
        vec![
            AgentDefinition {
                name: "Codex".to_string(),
                command: vec!["codex".to_string()],
            },
            AgentDefinition {
                name: "Claude Code".to_string(),
                command: vec!["claude".to_string()],
            },
            AgentDefinition {
                name: "OpenCode".to_string(),
                command: vec!["opencode".to_string()],
            },
            AgentDefinition {
                name: "NanoCode".to_string(),
                command: vec!["nanocode".to_string()],
            },
        ]
    }
}

struct AgentSession {
    name: String,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    rx: Receiver<Vec<u8>>,
    _child: Box<dyn Child + Send>,
    finished: bool,
}

pub struct AgentSidebar {
    pub agents: Vec<AgentDefinition>,
    pub selected_index: usize,
    session: Option<AgentSession>,
    scroll_offset: usize,
    last_output_height: u16,
    last_size: Option<(u16, u16)>,
    last_error: Option<String>,
    dsr_match_index: usize,
    dsr_pending: Vec<u8>,
    parser: vt100::Parser,
    follow_output: bool,
}

impl AgentSidebar {
    pub fn new() -> Self {
        AgentSidebar {
            agents: AgentDefinition::defaults(),
            selected_index: 0,
            session: None,
            scroll_offset: 0,
            last_output_height: 0,
            last_size: None,
            last_error: None,
            dsr_match_index: 0,
            dsr_pending: Vec::new(),
            parser: vt100::Parser::new(24, 80, 2000),
            follow_output: true,
        }
    }

    pub fn is_running(&self) -> bool {
        self.session.as_ref().map_or(false, |s| !s.finished)
    }

    pub fn active_name(&self) -> Option<&str> {
        self.session.as_ref().map(|s| s.name.as_str())
    }

    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn select_next(&mut self) {
        if self.selected_index + 1 < self.agents.len() {
            self.selected_index += 1;
        }
    }

    pub fn start_selected(&mut self, cwd: &Path) -> io::Result<()> {
        if self.agents.is_empty() {
            return Ok(());
        }
        let agent = self.agents[self.selected_index].clone();
        self.start_command(agent.command, agent.name, cwd)
    }

    pub fn start_command(&mut self, command: Vec<String>, display_name: String, cwd: &Path) -> io::Result<()> {
        if command.is_empty() {
            return Ok(());
        }

        self.scroll_offset = 0;
        self.last_error = None;
        self.follow_output = true;

        let cols = self.last_size.map(|(w, _)| w).unwrap_or(80).max(10);
        let rows = self.last_size.map(|(_, h)| h).unwrap_or(24).max(5);
        self.parser = vt100::Parser::new(rows, cols, 2000);

        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        }).map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

        let mut cmd = CommandBuilder::new(&command[0]);
        cmd.cwd(cwd);
        for arg in command.iter().skip(1) {
            cmd.arg(arg);
        }

        let child = pair.slave.spawn_command(cmd).map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        let mut reader = pair.master.try_clone_reader().map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        let writer = pair.master.take_writer().map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let mut buffer = [0u8; 4096];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx.send(buffer[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        self.session = Some(AgentSession {
            name: display_name,
            master: pair.master,
            writer,
            rx,
            _child: child,
            finished: false,
        });

        Ok(())
    }

    pub fn send_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        if let Some(session) = &mut self.session {
            if session.finished {
                return Ok(());
            }
            session.writer.write_all(bytes)?;
            session.writer.flush()?;
        }
        Ok(())
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        if self.last_size == Some((cols, rows)) {
            return;
        }
        self.last_size = Some((cols, rows));
        self.parser = vt100::Parser::new(rows.max(5), cols.max(10), 2000);
        self.follow_output = true;
        if let Some(session) = &mut self.session {
            let _ = session.master.resize(PtySize {
                rows: rows.max(5),
                cols: cols.max(10),
                pixel_width: 0,
                pixel_height: 0,
            });
        }
    }

    pub fn drain_output(&mut self) {
        let mut chunks = Vec::new();
        let mut dsr_match_index = self.dsr_match_index;
        let mut dsr_pending = std::mem::take(&mut self.dsr_pending);

        if let Some(session) = &mut self.session {
            loop {
                match session.rx.try_recv() {
                    Ok(chunk) => {
                        let filtered = Self::filter_control_queries_bytes(
                            &chunk,
                            &mut dsr_match_index,
                            &mut dsr_pending,
                            &mut session.writer,
                        );
                        if !filtered.is_empty() {
                            chunks.push(filtered);
                        }
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        session.finished = true;
                        break;
                    }
                }
            }
        }

        self.dsr_match_index = dsr_match_index;
        self.dsr_pending = dsr_pending;

        for chunk in chunks {
            self.parser.process(&chunk);
        }

    }

    fn filter_control_queries_bytes(
        bytes: &[u8],
        dsr_match_index: &mut usize,
        dsr_pending: &mut Vec<u8>,
        writer: &mut dyn Write,
    ) -> Vec<u8> {
        const DSR_QUERY: [u8; 4] = [0x1b, b'[', b'6', b'n'];
        let mut output = Vec::new();
        let mut i = 0;

        while i < bytes.len() {
            let b = bytes[i];
            if *dsr_match_index > 0 || b == DSR_QUERY[0] {
                let expected = DSR_QUERY[*dsr_match_index];
                if b == expected {
                    dsr_pending.push(b);
                    *dsr_match_index += 1;
                    i += 1;
                    if *dsr_match_index == DSR_QUERY.len() {
                        let _ = writer.write_all(b"\x1b[1;1R");
                        let _ = writer.flush();
                        *dsr_match_index = 0;
                        dsr_pending.clear();
                    }
                    continue;
                } else {
                    output.extend_from_slice(dsr_pending);
                    dsr_pending.clear();
                    *dsr_match_index = 0;
                    continue;
                }
            }

            output.push(b);
            i += 1;
        }

        output
    }

    pub fn render<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect, color_config: &ColorConfig) {
        if area.width < 5 || area.height < 5 {
            return;
        }

        let list_height = (self.agents.len() as u16 + 2).min(area.height.saturating_sub(3)).min(8);
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(list_height),
                Constraint::Min(3),
            ])
            .split(area);
        self.last_output_height = layout[1].height.saturating_sub(2);

        let list_items: Vec<ListItem> = self
            .agents
            .iter()
            .enumerate()
            .map(|(_idx, agent)| {
                let marker = if Some(agent.name.as_str()) == self.active_name() {
                    if self.is_running() { "*" } else { "x" }
                } else {
                    " "
                };
                ListItem::new(format!("{} {}", marker, agent.name))
            })
            .collect();

        let list = List::new(list_items)
            .block(Block::default().title("Agents").borders(Borders::ALL)
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
        f.render_stateful_widget(list, layout[0], &mut state);

        let output_title = if let Some(name) = self.active_name() {
            if self.is_running() {
                format!("Agent Output - {}", name)
            } else {
                format!("Agent Output - {} (exited)", name)
            }
        } else {
            "Agent Output".to_string()
        };

        if self.follow_output {
            self.scroll_offset = 0;
        }
        self.parser.screen_mut().set_scrollback(self.scroll_offset);
        self.scroll_offset = self.parser.screen().scrollback();

        let (mut output_lines, _line_count, has_content) = self.render_screen_lines();

        if !has_content && !self.is_running() {
            output_lines.clear();
        }

        if output_lines.is_empty() {
            if let Some(error) = &self.last_error {
                output_lines.push(Spans::from(Span::raw(error.clone())));
            } else if self.is_running() {
                output_lines.push(Spans::from(Span::raw("Waiting for output...")));
            } else {
                output_lines.push(Spans::from(Span::raw("Select an agent and press Enter to start.")));
                output_lines.push(Spans::from(Span::raw("Use Ctrl+G to close the sidebar.")));
            }
        }

        let output_paragraph = Paragraph::new(output_lines)
            .block(Block::default().title(output_title).borders(Borders::ALL))
            .style(Style::default()
                .bg(ui::parse_color(&color_config.background))
                .fg(ui::parse_color(&color_config.foreground)))
            .scroll((0, 0));

        f.render_widget(output_paragraph, layout[1]);
    }

    pub fn set_error(&mut self, message: String) {
        self.last_error = Some(message);
    }

    pub fn scroll_up(&mut self, lines: usize) {
        if self.follow_output {
            self.follow_output = false;
        }
        self.scroll_offset = self.scroll_offset.saturating_add(lines);
    }

    pub fn scroll_down(&mut self, lines: usize) {
        if self.scroll_offset <= lines {
            self.scroll_offset = 0;
            self.follow_output = true;
        } else {
            self.scroll_offset = self.scroll_offset.saturating_sub(lines);
        }
    }

    pub fn page_up(&mut self) {
        let page = self.last_output_height.max(1) as usize;
        self.scroll_up(page);
    }

    pub fn page_down(&mut self) {
        let page = self.last_output_height.max(1) as usize;
        self.scroll_down(page);
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
        self.follow_output = true;
    }

    fn render_screen_lines(&self) -> (Vec<Spans<'static>>, usize, bool) {
        let screen = self.parser.screen();
        let (rows, cols) = screen.size();
        if rows == 0 || cols == 0 {
            return (Vec::new(), 0, false);
        }

        let mut lines = Vec::with_capacity(rows as usize);
        let mut has_content = false;
        for row in 0..rows {
            let mut spans = Vec::new();
            let mut col = 0;
            while col < cols {
                if let Some(cell) = screen.cell(row, col) {
                    if cell.is_wide_continuation() {
                        col += 1;
                        continue;
                    }

                    let content = if cell.has_contents() {
                        cell.contents().to_string()
                    } else {
                        " ".to_string()
                    };
                    if !has_content && cell.has_contents() {
                        has_content = true;
                    }

                    let mut fg = vt_color_to_tui(cell.fgcolor());
                    let mut bg = vt_color_to_tui(cell.bgcolor());
                    if cell.inverse() {
                        std::mem::swap(&mut fg, &mut bg);
                    }

                    let mut style = Style::default();
                    if let Some(color) = fg {
                        style = style.fg(color);
                    }
                    if let Some(color) = bg {
                        style = style.bg(color);
                    }
                    if cell.bold() {
                        style = style.add_modifier(Modifier::BOLD);
                    }
                    if cell.dim() {
                        style = style.add_modifier(Modifier::DIM);
                    }
                    if cell.italic() {
                        style = style.add_modifier(Modifier::ITALIC);
                    }
                    if cell.underline() {
                        style = style.add_modifier(Modifier::UNDERLINED);
                    }

                    spans.push(Span::styled(content, style));
                }
                col += 1;
            }
            lines.push(Spans::from(spans));
        }

        (lines, rows as usize, has_content)
    }
}

fn vt_color_to_tui(color: vt100::Color) -> Option<Color> {
    match color {
        vt100::Color::Default => None,
        vt100::Color::Idx(idx) => Some(Color::Indexed(idx)),
        vt100::Color::Rgb(r, g, b) => Some(Color::Rgb(r, g, b)),
    }
}
