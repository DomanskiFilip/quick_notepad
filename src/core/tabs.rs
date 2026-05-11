use crate::tui::view::Buffer;
use crate::tui::caret::Position;
use crate::core::edit_history::EditHistory;
use std::fs;
use std::io::Error;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

#[derive(Clone)]
pub struct Tab {
    pub buffer: Buffer,
    pub filename: Option<String>,  // Display name only
    pub filepath: Option<String>,  // Full path for saving
    pub filetype: Option<String>,
    pub scroll_offset: usize,
    pub cursor_pos: Position,
    pub has_unsaved_changes: bool,
    pub edit_history: EditHistory,
}

impl Tab {
    pub fn new(buffer: Buffer, filename: Option<String>, filepath: Option<String>, filetype: Option<String>) -> Self {
        Self {
            buffer,
            filename,
            filepath,
            filetype,
            scroll_offset: 0,
            cursor_pos: Position::default(),
            has_unsaved_changes: false,
            edit_history: EditHistory::new(500),
        }
    }

    pub fn blank() -> Self {
        Self::new(Buffer::default(), None, None, None)
    }

    pub fn from_file(path: &str) -> Result<Self, Error> {
        let path_buf = std::fs::canonicalize(path)
            .unwrap_or_else(|_| std::path::PathBuf::from(path));

        let display_name = path_buf
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path_buf.to_string_lossy().into_owned());

        let full_path = path_buf.to_string_lossy().into_owned();
        let raw_ext = path_buf.extension().map(|ext| ext.to_string_lossy().into_owned());
        let friendly_filetype = get_friendly_filetype(raw_ext);

        let content = std::fs::read_to_string(&path_buf)?;
        let buffer = Buffer::from_string(content);

        Ok(Self::new(buffer, Some(display_name), Some(full_path), friendly_filetype))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct TabInfo {
    filename: Option<String>,
    filepath: Option<String>,
    filetype: Option<String>,
    scroll_offset: usize,
    cursor_line: u16,
    cursor_col: u16,
}

#[derive(Serialize, Deserialize, Debug)]
struct TabSession {
    tabs: Vec<TabInfo>,
    active_tab_index: usize,
}

pub struct TabManager {
    pub tabs: Vec<Tab>,
    pub active_tab_index: usize,
    pub max_tabs: usize,
    session_file: PathBuf,
}

impl TabManager {
    // User ran `quick` with no arguments — restore last session or start with one blank tab.
    pub fn restore_or_blank() -> Self {
        let session_file = Self::get_session_file_path();
        if let Ok(session) = Self::load_session(&session_file) {
            return Self::from_session(session, session_file);
        }
        Self {
            tabs: vec![Tab::blank()],
            active_tab_index: 0,
            max_tabs: 10,
            session_file,
        }
    }

    // User ran `quick somefile.txt` — that file is tab 1, prior session tabs follow.
    pub fn with_file(path: &str) -> Result<Self, Error> {
        let session_file = Self::get_session_file_path();

        let first_tab = Tab::from_file(path)?;
        let first_filepath = first_tab.filepath.clone();
        let mut tabs: Vec<Tab> = vec![first_tab];

        if let Ok(session) = Self::load_session(&session_file) {
            for tab_info in session.tabs {
                if tabs.len() >= 10 {
                    break;
                }
                // Skip the file we already opened as tab 1
                if tab_info.filepath.as_deref() == first_filepath.as_deref() {
                    continue;
                }
                // Skip blank session tabs when opening a specific file
                let Some(ref filepath) = tab_info.filepath else { continue };
                match Tab::from_file(filepath) {
                    Ok(mut t) => {
                        t.filetype = tab_info.filetype;
                        t.scroll_offset = tab_info.scroll_offset;
                        t.cursor_pos = Position {
                            x: tab_info.cursor_col,
                            y: tab_info.cursor_line,
                        };
                        tabs.push(t);
                    }
                    Err(_) => continue, // Skip files that no longer exist on disk
                }
            }
        }

        Ok(Self {
            tabs,
            active_tab_index: 0,
            max_tabs: 10,
            session_file,
        })
    }

    // Compatibility shim — prefer restore_or_blank() or with_file().
    pub fn new(initial_buffer: Buffer, filename: Option<String>, filetype: Option<String>) -> Self {
        if filename.is_none() {
            return Self::restore_or_blank();
        }
        let initial_tab = Tab::new(initial_buffer, filename, None, filetype);
        Self {
            tabs: vec![initial_tab],
            active_tab_index: 0,
            max_tabs: 10,
            session_file: Self::get_session_file_path(),
        }
    }

    fn get_session_file_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let mut path = PathBuf::from(home);
        path.push(".quicknotepad");
        if let Err(e) = fs::create_dir_all(&path) {
            eprintln!("Warning: Could not create .quicknotepad directory: {}", e);
        }
        path.push("tabs.json");
        path
    }

    fn load_session(path: &PathBuf) -> Result<TabSession, Error> {
        let content = fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| Error::new(std::io::ErrorKind::InvalidData, e))
    }

    fn from_session(session: TabSession, session_file: PathBuf) -> Self {
        let mut tabs = Vec::new();

        for tab_info in session.tabs {
            if tabs.len() >= 10 {
                break;
            }
            let tab = if let Some(ref filepath) = tab_info.filepath {
                match Tab::from_file(filepath) {
                    Ok(mut t) => {
                        t.filetype = tab_info.filetype.clone();
                        t.scroll_offset = tab_info.scroll_offset;
                        t.cursor_pos = Position {
                            x: tab_info.cursor_col,
                            y: tab_info.cursor_line,
                        };
                        t
                    }
                    Err(e) => {
                        eprintln!("Could not load file {}: {}", filepath, e);
                        continue; // Skip missing files instead of inserting blank
                    }
                }
            } else {
                Tab::blank()
            };
            tabs.push(tab);
        }

        if tabs.is_empty() {
            tabs.push(Tab::blank());
        }

        let active_index = session.active_tab_index.min(tabs.len() - 1);

        Self {
            tabs,
            active_tab_index: active_index,
            max_tabs: 10,
            session_file,
        }
    }

    pub fn save_session(&self) -> Result<(), Error> {
        let tab_infos: Vec<TabInfo> = self.tabs.iter().map(|tab| TabInfo {
            filename: tab.filename.clone(),
            filepath: tab.filepath.clone(),
            filetype: tab.filetype.clone(),
            scroll_offset: tab.scroll_offset,
            cursor_line: tab.cursor_pos.y,
            cursor_col: tab.cursor_pos.x,
        }).collect();

        let session = TabSession {
            tabs: tab_infos,
            active_tab_index: self.active_tab_index,
        };

        let json = serde_json::to_string_pretty(&session)
            .map_err(|e| Error::new(std::io::ErrorKind::InvalidData, e))?;

        fs::write(&self.session_file, json)?;
        Ok(())
    }

    pub fn current_tab(&self) -> &Tab {
        &self.tabs[self.active_tab_index]
    }

    pub fn current_tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active_tab_index]
    }

    pub fn switch_to_tab(&mut self, tab_number: usize) -> Result<(), Error> {
        if tab_number < 1 {
            return Ok(());
        }
        let tab_index = tab_number - 1;
        if tab_index >= self.tabs.len() {
            return Ok(());
        }
        self.active_tab_index = tab_index;
        let _ = self.save_session();
        Ok(())
    }

    // Open a new blank tab, append at end, switch to it.
    pub fn new_tab(&mut self) -> usize {
        if self.tabs.len() >= self.max_tabs {
            // Remove the oldest tab (last element) when the limit is reached
            self.tabs.pop();
        }
        // Insert the new blank tab at the beginning (index 0)
        self.tabs.insert(0, Tab::blank());
        self.active_tab_index = 0; // The new tab is now the active one
        let _ = self.save_session();
        self.active_tab_index
    }

    // Open a file. If already open, switch to it. Otherwise append and switch.
    pub fn open_file_in_new_tab(&mut self, path: &str) -> Result<usize, Error> {
        for (i, tab) in self.tabs.iter().enumerate() {
            if let Some(ref filepath) = tab.filepath {
                if filepath == path {
                    self.active_tab_index = i;
                    let _ = self.save_session();
                    return Ok(i);
                }
            }
        }
        if self.tabs.len() >= self.max_tabs {
            self.tabs.pop();
        }
        let new_tab = Tab::from_file(path)?;
        self.tabs.push(new_tab);
        self.active_tab_index = self.tabs.len() - 1;
        let _ = self.save_session();
        Ok(self.active_tab_index)
    }

    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }
}

impl Drop for TabManager {
    fn drop(&mut self) {
        let _ = self.save_session();
    }
}

pub fn get_friendly_filetype(extension: Option<String>) -> Option<String> {
    extension.map(|ext| {
        match ext.to_lowercase().as_str() {
            "rs" => "Rust".to_string(),
            "py" | "pyw" => "Python".to_string(),
            "js" | "mjs" => "JavaScript".to_string(),
            "ts" | "mts" => "TypeScript".to_string(),
            "c" => "C".to_string(),
            "cpp" | "cc" | "cxx" | "hpp" => "C++".to_string(),
            "cs" => "C#".to_string(),
            "java" | "jar" => "Java".to_string(),
            "go" => "Go".to_string(),
            "rb" => "Ruby".to_string(),
            "php" => "PHP".to_string(),
            "swift" => "Swift".to_string(),
            "kt" | "kts" => "Kotlin".to_string(),
            "dart" => "Dart".to_string(),
            "lua" => "Lua".to_string(),
            "pl" | "pm" => "Perl".to_string(),
            "r" => "R".to_string(),
            "scala" => "Scala".to_string(),
            "hs" => "Haskell".to_string(),
            "zig" => "Zig".to_string(),
            "nim" => "Nim".to_string(),
            "html" | "htm" => "HTML".to_string(),
            "css" => "CSS".to_string(),
            "scss" | "sass" => "Sass".to_string(),
            "jsx" => "React JSX".to_string(),
            "tsx" => "React TSX".to_string(),
            "vue" => "Vue".to_string(),
            "json" => "JSON".to_string(),
            "toml" => "TOML".to_string(),
            "yaml" | "yml" => "YAML".to_string(),
            "xml" => "XML".to_string(),
            "ini" | "conf" | "cfg" => "Config".to_string(),
            "sql" => "SQL Query".to_string(),
            "env" => "Environment".to_string(),
            "sh" => "Shell Script".to_string(),
            "bash" => "Bash Script".to_string(),
            "zsh" => "Zsh Script".to_string(),
            "ps1" => "PowerShell".to_string(),
            "bat" | "cmd" => "Batch File".to_string(),
            "make" | "mak" => "Makefile".to_string(),
            "txt" => "Text File".to_string(),
            "md" | "markdown" => "Markdown".to_string(),
            "log" => "Log File".to_string(),
            "csv" => "CSV Data".to_string(),
            "tex" => "LaTeX".to_string(),
            _ => ext.to_uppercase(),
        }
    })
}
