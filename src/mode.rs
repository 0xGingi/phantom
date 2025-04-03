use std::fmt;

#[derive(PartialEq, Clone, Copy)]
pub enum Mode {
    Normal,
    Insert,
    Command,
    Visual,
    FileSelect,
    DirectoryNav,
    Search,
    SidebarActive,
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