
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
    pub filename: Option<String>,
    pub scroll_offset: usize,
    pub cursor_pos: Position,
    pub has_unsaved_changes: bool,
    pub edit_history: EditHistory,
}

impl Tab {
    pub fn new(buffer: Buffer, filename: Option<String>) -> Self {
        Self {
            buffer,
            filename,
            scroll_offset: 0,
            cursor_pos: Position::default(),
            has_unsaved_changes: false,
            edit_history: EditHistory::new(500),
        }
    }

    pub fn from_file(path: &str) -> Result<Self, Error> {
        // Convert to absolute path immediately
        let absolute_path = std::fs::canonicalize(path)
            .map(|p| p.to_string_lossy().into_owned())
            // Fallback to original if canonicalize fails (e.g., file doesn't exist yet)
            .unwrap_or_else(|_| path.to_string());

        let content = fs::read_to_string(&absolute_path)?;
        let buffer = Buffer::from_string(content);
        
        Ok(Self::new(buffer, Some(absolute_path)))
    }
}

// Serializable tab info for persistence
#[derive(Serialize, Deserialize, Debug, Clone)]
struct TabInfo {
    filename: Option<String>,
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
    pub fn new(initial_buffer: Buffer, filename: Option<String>) -> Self {
        let session_file = Self::get_session_file_path();
        
        // Try to load previous session
        let manager = if let Ok(session) = Self::load_session(&session_file) {
            Self::from_session(session)
        } else {
            let initial_tab = Tab::new(initial_buffer, filename);
            Self {
                tabs: vec![initial_tab],
                active_tab_index: 0,
                max_tabs: 10,
                session_file,
            }
        };
        
        manager
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
            let tab = if let Some(ref filename) = tab_info.filename {
                // Try to load the file
                match Tab::from_file(filename) {
                    Ok(mut t) => {
                        t.scroll_offset = tab_info.scroll_offset;
                        t.cursor_pos = Position {
                            x: tab_info.cursor_col,
                            y: tab_info.cursor_line,
                        };
                        t
                    }
                    Err(e) => {
                        eprintln!("Could not load file {}: {}", filename, e);
                        // Create empty tab as fallback
                        Tab::new(Buffer::default(), None)
                    }
                }
            } else {
                Tab::new(Buffer::default(), None)
            };
            
            tabs.push(tab);
        }
        
        // Ensure at least one tab exists
        if tabs.is_empty() {
            tabs.push(Tab::new(Buffer::default(), None));
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
        let new_tab = Tab::new(Buffer::default(), None);
        
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
        // Check if file is already open
        for (i, tab) in self.tabs.iter().enumerate() {
            if let Some(ref filename) = tab.filename {
                if filename == path {
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