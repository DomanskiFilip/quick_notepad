// state - adapter between core logic and GUI with Wayland-safe clipboard handling
use crate::core::{
    buffer::Buffer,
    edit_history::EditHistory,
    selection::{Selection, TextPosition},
    tabs::TabManager,
};

pub struct EditorState {
    pub tab_manager: TabManager,
    pub selection: Option<Selection>,
    pub cursor_pos: TextPosition,
    pub scroll_offset: (usize, usize), // (line, column)
    pub search_query: String,
    pub search_active: bool,
    pub is_dragging: bool,
    // Dual clipboard approach:
    // - arboard handles actual system clipboard (works on X11 and most Wayland)
    // - internal clipboard_text as fallback for edge cases
    // - egui events provide Wayland compatibility layer
    clipboard_text: Option<String>,
}

impl EditorState {
    pub fn new(file_path: Option<String>) -> Self {
        let tab_manager = if let Some(path) = file_path {
            let mut tm = TabManager::new(Buffer::default(), None, None);
            if let Err(e) = tm.open_file_in_new_tab(&path) {
                eprintln!("Failed to open file: {}", e);
            }
            tm
        } else {
            TabManager::new(Buffer::default(), None, None)
        };

        Self {
            tab_manager,
            selection: None,
            cursor_pos: TextPosition { line: 0, column: 0 },
            scroll_offset: (0, 0),
            search_query: String::new(),
            search_active: false,
            is_dragging: false,
            clipboard_text: None,
        }
    }

    pub fn current_buffer(&self) -> &Buffer {
        &self.tab_manager.current_tab().buffer
    }

    pub fn current_buffer_mut(&mut self) -> &mut Buffer {
        &mut self.tab_manager.current_tab_mut().buffer
    }

    pub fn current_edit_history(&mut self) -> &mut EditHistory {
        &mut self.tab_manager.current_tab_mut().edit_history
    }

    pub fn has_unsaved_changes(&self) -> bool {
        self.tab_manager.current_tab().has_unsaved_changes
    }

    pub fn mark_dirty(&mut self) {
        self.tab_manager.current_tab_mut().has_unsaved_changes = true;
    }

    pub fn mark_clean(&mut self) {
        self.tab_manager.current_tab_mut().has_unsaved_changes = false;
    }

    pub fn current_filename(&self) -> Option<&str> {
        self.tab_manager.current_tab().filename.as_deref()
    }

    pub fn set_filename(&mut self, filename: String) {
        self.tab_manager.current_tab_mut().filename = Some(filename);
    }

    // Insert text at cursor position
    pub fn insert_text(&mut self, text: &str) {
        let pos = self.cursor_pos;
        let buffer = self.current_buffer_mut();

        // Ensure line exists
        while buffer.lines.len() <= pos.line {
            buffer.lines.push(String::new());
        }

        // Handle newlines
        if text.contains('\n') {
            let lines: Vec<&str> = text.split('\n').collect();
            let current_line = buffer.lines[pos.line].clone();
            let before: String = current_line.chars().take(pos.column).collect();
            let after: String = current_line.chars().skip(pos.column).collect();

            buffer.lines[pos.line] = format!("{}{}", before, lines[0]);

            for i in 1..lines.len() {
                let line_content = if i == lines.len() - 1 {
                    format!("{}{}", lines[i], after)
                } else {
                    lines[i].to_string()
                };
                buffer.lines.insert(pos.line + i, line_content);
            }

            self.cursor_pos = TextPosition {
                line: pos.line + lines.len() - 1,
                column: lines[lines.len() - 1].len(),
            };
        } else {
            // Single line insert
            let line = &mut buffer.lines[pos.line];
            if pos.column <= line.len() {
                line.insert_str(pos.column, text);
            } else {
                line.push_str(text);
            }
            self.cursor_pos = TextPosition {
                line: pos.line,
                column: pos.column + text.len(),
            };
        }

        self.mark_dirty();
    }
    
    pub fn move_cursor(&mut self, dx: isize, dy: isize) {
        let line_count = self.current_buffer().lines.len();
        
        let mut new_line = self.cursor_pos.line as isize + dy;
        new_line = new_line.clamp(0, line_count.saturating_sub(1) as isize);
        
        self.cursor_pos.line = new_line as usize;
    
        let line_len = self.current_buffer().lines[self.cursor_pos.line].len();
        let mut new_col = self.cursor_pos.column as isize + dx;
        new_col = new_col.clamp(0, line_len as isize);
        
        self.cursor_pos.column = new_col as usize;
    }
    
    pub fn clamp_cursor(&mut self) {
        let line_count = self.current_buffer().lines.len();
        self.cursor_pos.line = self.cursor_pos.line.min(line_count.saturating_sub(1));
        let line_len = self.current_buffer().lines[self.cursor_pos.line].len();
        self.cursor_pos.column = self.cursor_pos.column.min(line_len);
    }

    // Delete selection or character at cursor
    pub fn delete_at_cursor(&mut self) {
        if let Some(selection) = self.selection.take() {
            self.delete_selection(selection);
        } else {
            let pos = self.cursor_pos;
            let buffer = self.current_buffer_mut();
            if pos.line < buffer.lines.len() {
                let line = &mut buffer.lines[pos.line];
                if pos.column < line.len() {
                    line.remove(pos.column);
                    self.clamp_cursor();
                    self.mark_dirty();
                }
            }
        }
    }

    // Backspace - delete character before cursor
    pub fn backspace(&mut self) {
        if let Some(selection) = self.selection.take() {
            self.delete_selection(selection);
        } else if self.cursor_pos.column > 0 {
            let pos = self.cursor_pos;
            let buffer = self.current_buffer_mut();
            if pos.line < buffer.lines.len() {
                let line = &mut buffer.lines[pos.line];
                if pos.column <= line.len() {
                    line.remove(pos.column - 1);
                    self.cursor_pos.column -= 1;
                    self.mark_dirty();
                }
            }
        } else if self.cursor_pos.line > 0 {
            let pos = self.cursor_pos;
            let buffer = self.current_buffer_mut();
            let current_line = buffer.lines[pos.line].clone();
            let prev_line_len = buffer.lines[pos.line - 1].len();
            buffer.lines[pos.line - 1].push_str(&current_line);
            buffer.lines.remove(pos.line);
            self.cursor_pos = TextPosition {
                line: self.cursor_pos.line - 1,
                column: prev_line_len,
            };
            self.clamp_cursor();
            self.mark_dirty();
        }
    }

    fn delete_selection(&mut self, selection: Selection) {
        let (start, end) = selection.get_range();
        let buffer = self.current_buffer_mut();

        if start.line == end.line {
            // Single line deletion
            let line = &mut buffer.lines[start.line];
            let chars: Vec<char> = line.chars().collect();
            let before: String = chars[..start.column].iter().collect();
            let after: String = chars[end.column..].iter().collect();
            *line = format!("{}{}", before, after);
        } else {
            // Multi-line deletion
            let before_text = buffer.lines[start.line]
                .chars()
                .take(start.column)
                .collect::<String>();
            let after_text = buffer.lines[end.line]
                .chars()
                .skip(end.column)
                .collect::<String>();

            for _ in start.line..=end.line {
                if start.line < buffer.lines.len() {
                    buffer.lines.remove(start.line);
                }
            }

            buffer
                .lines
                .insert(start.line, format!("{}{}", before_text, after_text));
        }

        self.cursor_pos = start;
        self.mark_dirty();
    }

    // Save current file
    pub fn save(&mut self) -> Result<(), std::io::Error> {
        if let Some(filepath) = self.tab_manager.current_tab().filepath.clone() {
            self.save_as(&filepath)?;
        }
        Ok(())
    }
    
    // Save as new file
    pub fn save_as(&mut self, path: &str) -> Result<(), std::io::Error> {
        use std::fs;
    
        let path_buf = std::fs::canonicalize(path)
            .unwrap_or_else(|_| {
                // File doesn't exist yet — build absolute path manually
                let mut current_dir = std::env::current_dir().unwrap_or_default();
                current_dir.push(path);
                current_dir
            });
    
        let full_path = path_buf.to_string_lossy().into_owned();
        let display_name = path_buf
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| full_path.clone());
    
        let buffer = self.current_buffer();
        let last_line = buffer
            .lines
            .iter()
            .rposition(|line| !line.is_empty())
            .unwrap_or(0);
        let content_lines: Vec<String> = buffer.lines.iter().take(last_line + 1).cloned().collect();
        let content = content_lines.join("\n");
    
        fs::write(&full_path, content)?;
    
        // Update BOTH filepath (full path for saving) and filename (display name)
        self.tab_manager.current_tab_mut().filepath = Some(full_path);
        self.tab_manager.current_tab_mut().filename = Some(display_name);
        self.mark_clean();
    
        let _ = self.tab_manager.save_session();
    
        Ok(())
    }

    // Copy selection to clipboard using arboard
    pub fn copy_selection(&mut self) {
        if let Some(ref selection) = self.selection {
            if !selection.is_active() {
                return;
            }

            let (start, end) = selection.get_range();
            let text = self.extract_text_range(start, end);
            
            // Try to use arboard (works on X11 and most Wayland compositors)
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(&text);
            }
            
            // Also store internally as fallback
            self.clipboard_text = Some(text);
        }
    }
    
    // Get the text from last copy operation
    pub fn get_clipboard_text(&self) -> Option<&str> {
        self.clipboard_text.as_deref()
    }

    // Cut selection to clipboard
    pub fn cut_selection(&mut self) {
        self.copy_selection();
        if let Some(selection) = self.selection.take() {
            if selection.is_active() {
                self.delete_selection(selection);
            }
        }
    }

    // Paste from clipboard using arboard
    pub fn paste_from_clipboard(&mut self) {
        // Delete selection first if active
        if let Some(selection) = self.selection.take() {
            if selection.is_active() {
                self.delete_selection(selection);
            }
        }

        // Try arboard first
        let text = if let Ok(mut clipboard) = arboard::Clipboard::new() {
            clipboard.get_text().ok()
        } else {
            None
        };
        
        // Fall back to internal clipboard if arboard fails
        let text = text.or_else(|| self.clipboard_text.clone());
        
        if let Some(text) = text {
            self.insert_text(&text);
        }
    }

    // Select all text
    pub fn select_all(&mut self) {
        let last_line = self
            .current_buffer()
            .lines
            .iter()
            .rposition(|line| !line.is_empty())
            .unwrap_or(0);
        let last_col = self
            .current_buffer()
            .lines
            .get(last_line)
            .map(|l| l.len())
            .unwrap_or(0);

        self.selection = Some(Selection {
            anchor: TextPosition { line: 0, column: 0 },
            cursor: TextPosition {
                line: last_line,
                column: last_col,
            },
        });
        self.cursor_pos = TextPosition {
            line: last_line,
            column: last_col,
        };
    }

    fn extract_text_range(&self, start: TextPosition, end: TextPosition) -> String {
        let mut result = String::new();
        let buffer = self.current_buffer();

        if start.line == end.line {
            // Single line
            if let Some(line) = buffer.lines.get(start.line) {
                let chars: Vec<char> = line.chars().collect();
                let text: String = chars
                    [start.column.min(chars.len())..end.column.min(chars.len())]
                    .iter()
                    .collect();
                result.push_str(&text);
            }
        } else {
            // Multi-line
            for line_idx in start.line..=end.line {
                if let Some(line) = buffer.lines.get(line_idx) {
                    let chars: Vec<char> = line.chars().collect();

                    if line_idx == start.line {
                        let text: String = chars[start.column.min(chars.len())..].iter().collect();
                        result.push_str(&text);
                        result.push('\n');
                    } else if line_idx == end.line {
                        let text: String = chars[..end.column.min(chars.len())].iter().collect();
                        result.push_str(&text);
                    } else {
                        result.push_str(line);
                        result.push('\n');
                    }
                }
            }
        }

        result
    }

    // Search functionality
    pub fn perform_search(&mut self) {
        if self.search_query.is_empty() {
            return;
        }

        let query = self.search_query.to_lowercase();
        let mut matches = Vec::new();

        for (line_idx, line) in self.current_buffer().lines.iter().enumerate() {
            let line_lower = line.to_lowercase();
            let mut start = 0;

            while let Some(pos) = line_lower[start..].find(&query) {
                matches.push(TextPosition {
                    line: line_idx,
                    column: start + pos,
                });
                start += pos + 1;
            }
        }

        if !matches.is_empty() {
            let first = matches[0];
            self.cursor_pos = first;
            self.selection = Some(Selection {
                anchor: first,
                cursor: TextPosition {
                    line: first.line,
                    column: first.column + self.search_query.len(),
                },
            });
        }
    }

    pub fn next_search_match(&mut self) {
        self.perform_search();
    }

    pub fn prev_search_match(&mut self) {
        self.perform_search();
    }
}