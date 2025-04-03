use std::error::Error;
use std::path::Path;
use std::env;
use copypasta::{ClipboardContext, ClipboardProvider};

mod config;

mod tab;
mod ui;

mod file_selector;
use file_selector::FileSelector;

mod mode;
use mode::Mode;

mod editor;
use editor::Editor;

enum ClipboardWrapper {
    Real(ClipboardContext),
    Dummy,
}

impl ClipboardWrapper {
    fn new() -> Self {
        match ClipboardContext::new() {
            Ok(clipboard) => ClipboardWrapper::Real(clipboard),
            Err(_) => ClipboardWrapper::Dummy,
        }
    }
}

impl ClipboardProvider for ClipboardWrapper {
    fn get_contents(&mut self) -> Result<String, Box<dyn Error + Send + Sync>> {
        match self {
            ClipboardWrapper::Real(clipboard) => clipboard.get_contents(),
            ClipboardWrapper::Dummy => Ok(String::new()),
        }
    }

    fn set_contents(&mut self, contents: String) -> Result<(), Box<dyn Error + Send + Sync>> {
        match self {
            ClipboardWrapper::Real(clipboard) => clipboard.set_contents(contents),
            ClipboardWrapper::Dummy => Ok(()),
        }
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