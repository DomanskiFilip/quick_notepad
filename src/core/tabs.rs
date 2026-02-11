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

    pub fn from_file(path: &str) -> Result<Self, Error> {
        let path_buf = std::fs::canonicalize(path)
            .unwrap_or_else(|_| std::path::PathBuf::from(path));
    
        // Get the filename for display
        let display_name = path_buf
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path_buf.to_string_lossy().into_owned());
    
        // Store the FULL PATH for saving
        let full_path = path_buf.to_string_lossy().into_owned();
    
        // Extract the file extension
        let raw_ext = path_buf.extension().map(|ext| ext.to_string_lossy().into_owned());
        let friendly_filetype = get_friendly_filetype(raw_ext);
    
        let content = std::fs::read_to_string(&path_buf)?;
        let buffer = Buffer::from_string(content);
        
        Ok(Self::new(buffer, Some(display_name), Some(full_path), friendly_filetype))
    }
}

// Serializable tab info for persistence
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
    pub fn new(initial_buffer: Buffer, filename: Option<String>, filetype: Option<String>) -> Self {
        let session_file = Self::get_session_file_path();
        
        // Check if we are starting truly "fresh" (no filename provided)
        // If we have a filename, we should skip session loading to prioritize the new file
        if filename.is_none() {
            if let Ok(session) = Self::load_session(&session_file) {
                return Self::from_session(session);
            }
        }
    
        // Default behavior if session load fails or a file was specified
        let initial_tab = Tab::new(initial_buffer, filename, None, filetype);
        Self {
            tabs: vec![initial_tab],
            active_tab_index: 0,
            max_tabs: 10,
            session_file,
        }
    }

    fn get_session_file_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let mut path = PathBuf::from(home);
        path.push(".quicknotepad");
        
        // Create directory if it doesn't exist
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

    fn from_session(session: TabSession) -> Self {
        let mut tabs = Vec::new();
        
        for tab_info in session.tabs {
            let tab = if let Some(ref filepath) = tab_info.filepath {
                // Use the FULL PATH to load the file
                match Tab::from_file(filepath) {
                    Ok(mut t) => {
                        // Apply the saved metadata
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
                        Tab::new(Buffer::default(), None, None, None)
                    }
                }
            } else {
                // Fallback for old sessions without filepath
                Tab::new(Buffer::default(), tab_info.filename.clone(), None, None)
            };
            
            tabs.push(tab);
        }
        
        // Ensure at least one tab exists
        if tabs.is_empty() {
            tabs.push(Tab::new(Buffer::default(), None, None, None));
        }
        
        let active_index = session.active_tab_index.min(tabs.len() - 1);
        
        Self {
            tabs,
            active_tab_index: active_index,
            max_tabs: 10,
            session_file: Self::get_session_file_path(),
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
        if tab_number < 1 || tab_number > self.max_tabs {
            return Ok(());
        }

        let tab_index = tab_number - 1;

        if tab_index >= self.tabs.len() {
            return Ok(());
        }

        self.active_tab_index = tab_index;
        
        // Save session after switching
        let _ = self.save_session();
        Ok(())
    }

    // Create new tab at position 1, shift everything else down
    pub fn new_tab(&mut self) -> usize {
        // If at max capacity, remove the last tab (oldest/least used)
        if self.tabs.len() >= self.max_tabs {
            self.tabs.pop();
        }

        // Create new tab
        let new_tab = Tab::new(Buffer::default(), None, None, None);
        
        // Insert at position 0 (tab 1)
        self.tabs.insert(0, new_tab);
        
        // New tab becomes active (at index 0)
        self.active_tab_index = 0;
        
        // Save session after creating new tab
        let _ = self.save_session();
        
        0
    }

    // Open file in tab 1, push everything else down
    pub fn open_file_in_new_tab(&mut self, path: &str) -> Result<usize, Error> {
        // Check if file is already open (compare by filepath)
        for (i, tab) in self.tabs.iter().enumerate() {
            if let Some(ref filepath) = tab.filepath {
                if filepath == path {
                    self.active_tab_index = i;
                    let _ = self.save_session();
                    return Ok(i);
                }
            }
        }

        // If at max capacity, remove last tab
        if self.tabs.len() >= self.max_tabs {
            self.tabs.pop();
        }

        // Load file
        let new_tab = Tab::from_file(path)?;
        
        // Insert at position 0 (tab 1)
        self.tabs.insert(0, new_tab);
        self.active_tab_index = 0;
        
        // Save session
        let _ = self.save_session();
        
        Ok(0)
    }
}

// Save session on drop
impl Drop for TabManager {
    fn drop(&mut self) {
        let _ = self.save_session();
    }
}

// utility for filetypes
pub fn get_friendly_filetype(extension: Option<String>) -> Option<String> {
    extension.map(|ext| {
        match ext.to_lowercase().as_str() {
            // Programming Languages
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

            // Web Technologies
            "html" | "htm" => "HTML".to_string(),
            "css" => "CSS".to_string(),
            "scss" | "sass" => "Sass".to_string(),
            "jsx" => "React JSX".to_string(),
            "tsx" => "React TSX".to_string(),
            "vue" => "Vue".to_string(),

            // Configuration & Data
            "json" => "JSON".to_string(),
            "toml" => "TOML".to_string(),
            "yaml" | "yml" => "YAML".to_string(),
            "xml" => "XML".to_string(),
            "ini" | "conf" | "cfg" => "Config".to_string(),
            "sql" => "SQL Query".to_string(),
            "env" => "Environment".to_string(),
            
            // Shell & Scripts
            "sh" => "Shell Script".to_string(),
            "bash" => "Bash Script".to_string(),
            "zsh" => "Zsh Script".to_string(),
            "ps1" => "PowerShell".to_string(),
            "bat" | "cmd" => "Batch File".to_string(),
            "make" | "mak" => "Makefile".to_string(),

            // Documentation & Text
            "txt" => "Text File".to_string(),
            "md" | "markdown" => "Markdown".to_string(),
            "log" => "Log File".to_string(),
            "csv" => "CSV Data".to_string(),
            "tex" => "LaTeX".to_string(),
            
            // Fallback
            _ => ext.to_uppercase(),
        }
    })
}