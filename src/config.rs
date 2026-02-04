use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Clone)]
pub struct ColorConfig {
    pub version: Option<String>,
    pub background: String,
    pub foreground: String,
    pub cursor: String,
    pub selection: String,
    pub comment: String,
    pub keyword: String,
    pub string: String,
    pub function: String,
    pub number: String,
    pub minimap_highlight: String,
    pub minimap_background: String,
    pub minimap_content: String,
    pub minimap_border: String,
    pub tab_active: String,
    pub tab_inactive: String,
    pub tab_background: String,
    pub file_selector_background: String,
    pub file_selector_foreground: String,
    pub file_selector_highlight: String,
    pub file_selector_border: String,
    pub line_number: String,
    pub bracket_match: String,
}

impl ColorConfig {
    pub fn default() -> Self {
        Self::one_dark()
    }
    
    pub fn gruvbox() -> Self {
        ColorConfig {
            version: Some("2.0.0".to_string()),
            background: "#282828".to_string(),
            foreground: "#ebdbb2".to_string(),
            cursor: "#ebdbb2".to_string(),
            selection: "#504945".to_string(),
            comment: "#928374".to_string(),
            keyword: "#fe8019".to_string(),
            string: "#b8bb26".to_string(),
            function: "#fabd2f".to_string(),
            number: "#d3869b".to_string(),
            minimap_highlight: "#504945".to_string(),
            minimap_background: "#1d2021".to_string(),
            minimap_content: "#928374".to_string(),
            minimap_border: "#504945".to_string(),
            tab_active: "#fabd2f".to_string(),
            tab_inactive: "#928374".to_string(),
            tab_background: "#282828".to_string(),
            file_selector_background: "#3c3836".to_string(),
            file_selector_foreground: "#ebdbb2".to_string(),
            file_selector_highlight: "#504945".to_string(),
            file_selector_border: "#504945".to_string(),
            line_number: "#7c6f64".to_string(),
            bracket_match: "#fabd2f".to_string(),
        }
    }
    
    pub fn one_dark() -> Self {
        ColorConfig {
            version: Some("2.0.0".to_string()),
            background: "#282c34".to_string(),
            foreground: "#abb2bf".to_string(),
            cursor: "#528bff".to_string(),
            selection: "#3e4451".to_string(),
            comment: "#5c6370".to_string(),
            keyword: "#c678dd".to_string(),
            string: "#98c379".to_string(),
            function: "#61afef".to_string(),
            number: "#d19a66".to_string(),
            minimap_highlight: "#3e4451".to_string(),
            minimap_background: "#21252b".to_string(),
            minimap_content: "#5c6370".to_string(),
            minimap_border: "#3e4451".to_string(),
            tab_active: "#61afef".to_string(),
            tab_inactive: "#5c6370".to_string(),
            tab_background: "#282c34".to_string(),
            file_selector_background: "#3e4451".to_string(),
            file_selector_foreground: "#abb2bf".to_string(),
            file_selector_highlight: "#528bff".to_string(),
            file_selector_border: "#528bff".to_string(),
            line_number: "#636d83".to_string(),
            bracket_match: "#528bff".to_string(),
        }
    }
    
    pub fn dracula() -> Self {
        ColorConfig {
            version: Some("2.0.0".to_string()),
            background: "#282a36".to_string(),
            foreground: "#f8f8f2".to_string(),
            cursor: "#ff79c6".to_string(),
            selection: "#44475a".to_string(),
            comment: "#6272a4".to_string(),
            keyword: "#ff79c6".to_string(),
            string: "#f1fa8c".to_string(),
            function: "#50fa7b".to_string(),
            number: "#bd93f9".to_string(),
            minimap_highlight: "#44475a".to_string(),
            minimap_background: "#21222c".to_string(),
            minimap_content: "#6272a4".to_string(),
            minimap_border: "#44475a".to_string(),
            tab_active: "#ff79c6".to_string(),
            tab_inactive: "#6272a4".to_string(),
            tab_background: "#282a36".to_string(),
            file_selector_background: "#44475a".to_string(),
            file_selector_foreground: "#f8f8f2".to_string(),
            file_selector_highlight: "#ff79c6".to_string(),
            file_selector_border: "#ff79c6".to_string(),
            line_number: "#6272a4".to_string(),
            bracket_match: "#ff79c6".to_string(),
        }
    }
    
    pub fn solarized_dark() -> Self {
        ColorConfig {
            version: Some("2.0.0".to_string()),
            background: "#002b36".to_string(),
            foreground: "#839496".to_string(),
            cursor: "#2aa198".to_string(),
            selection: "#073642".to_string(),
            comment: "#586e75".to_string(),
            keyword: "#859900".to_string(),
            string: "#2aa198".to_string(),
            function: "#268bd2".to_string(),
            number: "#d33682".to_string(),
            minimap_highlight: "#073642".to_string(),
            minimap_background: "#00212b".to_string(),
            minimap_content: "#586e75".to_string(),
            minimap_border: "#073642".to_string(),
            tab_active: "#2aa198".to_string(),
            tab_inactive: "#586e75".to_string(),
            tab_background: "#002b36".to_string(),
            file_selector_background: "#073642".to_string(),
            file_selector_foreground: "#839496".to_string(),
            file_selector_highlight: "#2aa198".to_string(),
            file_selector_border: "#2aa198".to_string(),
            line_number: "#586e75".to_string(),
            bracket_match: "#2aa198".to_string(),
        }
    }
    
    pub fn nord() -> Self {
        ColorConfig {
            version: Some("2.0.0".to_string()),
            background: "#2e3440".to_string(),
            foreground: "#d8dee9".to_string(),
            cursor: "#88c0d0".to_string(),
            selection: "#434c5e".to_string(),
            comment: "#616e88".to_string(),
            keyword: "#81a1c1".to_string(),
            string: "#a3be8c".to_string(),
            function: "#88c0d0".to_string(),
            number: "#b48ead".to_string(),
            minimap_highlight: "#434c5e".to_string(),
            minimap_background: "#3b4252".to_string(),
            minimap_content: "#616e88".to_string(),
            minimap_border: "#434c5e".to_string(),
            tab_active: "#88c0d0".to_string(),
            tab_inactive: "#616e88".to_string(),
            tab_background: "#2e3440".to_string(),
            file_selector_background: "#434c5e".to_string(),
            file_selector_foreground: "#d8dee9".to_string(),
            file_selector_highlight: "#88c0d0".to_string(),
            file_selector_border: "#88c0d0".to_string(),
            line_number: "#616e88".to_string(),
            bracket_match: "#88c0d0".to_string(),
        }
    }
    
    pub fn monokai() -> Self {
        ColorConfig {
            version: Some("2.0.0".to_string()),
            background: "#272822".to_string(),
            foreground: "#f8f8f2".to_string(),
            cursor: "#f92672".to_string(),
            selection: "#49483e".to_string(),
            comment: "#75715e".to_string(),
            keyword: "#f92672".to_string(),
            string: "#e6db74".to_string(),
            function: "#a6e22e".to_string(),
            number: "#ae81ff".to_string(),
            minimap_highlight: "#49483e".to_string(),
            minimap_background: "#1e1f1c".to_string(),
            minimap_content: "#75715e".to_string(),
            minimap_border: "#49483e".to_string(),
            tab_active: "#f92672".to_string(),
            tab_inactive: "#75715e".to_string(),
            tab_background: "#272822".to_string(),
            file_selector_background: "#49483e".to_string(),
            file_selector_foreground: "#f8f8f2".to_string(),
            file_selector_highlight: "#f92672".to_string(),
            file_selector_border: "#f92672".to_string(),
            line_number: "#75715e".to_string(),
            bracket_match: "#f92672".to_string(),
        }
    }
    
    pub fn get_theme(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "gruvbox" => Self::gruvbox(),
            "one_dark" | "onedark" => Self::one_dark(),
            "dracula" => Self::dracula(),
            "solarized_dark" | "solarized-dark" => Self::solarized_dark(),
            "nord" => Self::nord(),
            "monokai" => Self::monokai(),
            _ => Self::one_dark(), // Default fallback
        }
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
    
    /// Merge user config with default config, adding any missing fields
    pub fn merge_with_defaults(mut self) -> Self {
        let defaults = Self::default();
        
        // For colors, we typically want to keep user values, but this could be extended
        // to add new color fields that didn't exist in older versions
        self.version = defaults.version;
        self
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Keybindings {
    pub version: Option<String>,
    pub normal_mode: HashMap<String, String>,
    pub insert_mode: HashMap<String, String>,
    pub visual_mode: HashMap<String, String>,
    pub command_mode: HashMap<String, String>,
    pub file_select_mode: HashMap<String, String>,
    pub search_mode: HashMap<String, String>,
    pub tab_mode: HashMap<String, String>,
}

impl Keybindings {
    pub fn default() -> Self {
        Keybindings {
            version: Some("2.0.0".to_string()),
            normal_mode: [
                ("dd".to_string(), "delete_line".to_string()),
                ("i".to_string(), "enter_insert_mode".to_string()),
                ("Insert".to_string(), "enter_insert_mode".to_string()),
                ("a".to_string(), "append".to_string()),
                ("o".to_string(), "open_line_below".to_string()),
                ("O".to_string(), "open_line_above".to_string()),
                ("yy".to_string(), "yank_line".to_string()),
                ("p".to_string(), "paste_after".to_string()),
                ("v".to_string(), "enter_visual_mode".to_string()),
                (":".to_string(), "enter_command_mode".to_string()),
                ("Ctrl+b".to_string(), "toggle_debug_menu".to_string()),
                ("Ctrl+e".to_string(), "toggle_sidebar".to_string()),
                ("Ctrl+g".to_string(), "toggle_agent_sidebar".to_string()),
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
                ("Left".to_string(), "move_cursor_left".to_string()),
                ("Down".to_string(), "move_cursor_down".to_string()),
                ("Up".to_string(), "move_cursor_up".to_string()),
                ("Right".to_string(), "move_cursor_right".to_string()),
                ("Home".to_string(), "move_cursor_start_of_line".to_string()),
                ("End".to_string(), "move_cursor_end_of_line".to_string()),
                ("PageUp".to_string(), "page_up".to_string()),
                ("PageDown".to_string(), "page_down".to_string()),
                ("Ctrl+l".to_string(), "toggle_line_numbers".to_string()),
                ("Ctrl+j".to_string(), "toggle_word_wrap".to_string()),
                ("Ctrl+i".to_string(), "toggle_auto_indent".to_string()),
                ("?".to_string(), "toggle_help".to_string()),
                ("Shift+T".to_string(), "cycle_theme".to_string()),
            ].iter().cloned().collect(),
            insert_mode: [
                ("Esc".to_string(), "exit_insert_mode".to_string()),
                ("Enter".to_string(), "insert_newline".to_string()),
                ("Backspace".to_string(), "backspace".to_string()),
                ("Delete".to_string(), "delete_char".to_string()),
                ("Left".to_string(), "move_cursor_left".to_string()),
                ("Down".to_string(), "move_cursor_down".to_string()),
                ("Up".to_string(), "move_cursor_up".to_string()),
                ("Right".to_string(), "move_cursor_right".to_string()),
            ].iter().cloned().collect(),
            visual_mode: [
                ("Esc".to_string(), "exit_visual_mode".to_string()),
                ("y".to_string(), "yank_selection".to_string()),
                ("d".to_string(), "delete_selection".to_string()),
                ("Left".to_string(), "move_cursor_left".to_string()),
                ("Down".to_string(), "move_cursor_down".to_string()),
                ("Up".to_string(), "move_cursor_up".to_string()),
                ("Right".to_string(), "move_cursor_right".to_string()),
            ].iter().cloned().collect(),
            command_mode: [
                ("Enter".to_string(), "execute_command".to_string()),
                ("Esc".to_string(), "exit_command_mode".to_string()),
            ].iter().cloned().collect(),
            file_select_mode: [
                ("Enter".to_string(), "select_file".to_string()),
                ("Esc".to_string(), "exit_file_select_mode".to_string()),
                ("Up".to_string(), "move_cursor_up".to_string()),
                ("Down".to_string(), "move_cursor_down".to_string()),
            ].iter().cloned().collect(),
            search_mode: [
                ("Enter".to_string(), "execute_search".to_string()),
                ("Esc".to_string(), "exit_search_mode".to_string()),
                ("Backspace".to_string(), "search_backspace".to_string()),
            ].iter().cloned().collect(),
            tab_mode: HashMap::new(),
        }
    }
    
    /// Merge user config with default config, adding any missing keys
    pub fn merge_with_defaults(mut self) -> Self {
        let defaults = Self::default();
        
        // Merge each mode's keybindings
        for (key, value) in defaults.normal_mode {
            self.normal_mode.entry(key).or_insert(value);
        }
        for (key, value) in defaults.insert_mode {
            self.insert_mode.entry(key).or_insert(value);
        }
        for (key, value) in defaults.visual_mode {
            self.visual_mode.entry(key).or_insert(value);
        }
        for (key, value) in defaults.command_mode {
            self.command_mode.entry(key).or_insert(value);
        }
        for (key, value) in defaults.file_select_mode {
            self.file_select_mode.entry(key).or_insert(value);
        }
        for (key, value) in defaults.search_mode {
            self.search_mode.entry(key).or_insert(value);
        }
        for (key, value) in defaults.tab_mode {
            self.tab_mode.entry(key).or_insert(value);
        }
        
        // Update version
        self.version = defaults.version;
        self
    }
} 
