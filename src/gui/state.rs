// state - adapter between core logic and GUI with Wayland-safe clipboard handling
use crate::core::{
    buffer::Buffer,
    edit_history::{Edit, EditHistory, EditOperation},
    search::{find_all_occurrences, find_closest_match, SearchState},
    selection::{Selection, TextPosition},
    tabs::TabManager,
};

use crate::core::graphemes::*;
use crate::tui::caret::Position;

pub struct EditorState {
    pub tab_manager: TabManager,
    pub selection: Option<Selection>,
    pub cursor_pos: TextPosition,
    pub scroll_offset: (usize, usize), // (line, column)
    pub search_query: String,
    pub search_active: bool,
    pub search_state: Option<SearchState>,
    pub search_focus_requested: bool,
    pub scroll_to_cursor: bool, // editor scrolls the cursor into view on the next frame
    pub is_dragging: bool,
    clipboard_text: Option<String>,
}

impl EditorState {
    pub fn new(file_path: Option<String>) -> Self {
        let mut tab_manager = if let Some(path) = file_path {
            let mut tm = TabManager::new(Buffer::default(), None, None);
            if let Err(e) = tm.open_file_in_new_tab(&path) {
                eprintln!("Failed to open file: {}", e);
            }
            tm
        } else {
            TabManager::new(Buffer::default(), None, None)
        };

        // Restored session tabs store the tui caret (screen position), convert it to line/column
        for tab in &mut tab_manager.tabs {
            tab.gui_cursor = TextPosition {
                line: tab.scroll_offset + tab.cursor_pos.y.saturating_sub(Position::HEADER) as usize,
                column: tab.cursor_pos.x.saturating_sub(Position::MARGIN) as usize,
            };
        }

        let mut state = Self {
            tab_manager,
            selection: None,
            cursor_pos: TextPosition { line: 0, column: 0 },
            scroll_offset: (0, 0),
            search_query: String::new(),
            search_active: false,
            is_dragging: false,
            clipboard_text: None,
            search_state: None,
            search_focus_requested: false,
            scroll_to_cursor: false,
        };
        state.load_tab_view();
        state
    }

    // Tabs - each tab remembers its own cursor and scroll position
    pub fn switch_to_tab(&mut self, tab_number: usize) {
        if tab_number == 0 || tab_number > self.tab_manager.tabs.len() {
            return;
        }
        self.store_tab_view();
        let _ = self.tab_manager.switch_to_tab(tab_number);
        self.load_tab_view();
    }

    pub fn new_tab(&mut self) {
        self.store_tab_view();
        self.tab_manager.new_tab();
        self.load_tab_view();
    }

    fn store_tab_view(&mut self) {
        let (cursor, scroll) = (self.cursor_pos, self.scroll_offset.0);
        let tab = self.tab_manager.current_tab_mut();
        tab.gui_cursor = cursor;
        tab.scroll_offset = scroll;
    }

    fn load_tab_view(&mut self) {
        let tab = self.tab_manager.current_tab();
        self.cursor_pos = tab.gui_cursor;
        self.scroll_offset = (tab.scroll_offset, 0);
        self.selection = None;
        self.is_dragging = false;
        if self.current_buffer().lines.is_empty() {
            self.current_buffer_mut().lines.push(String::new());
        }
        self.clamp_cursor();
        self.scroll_to_cursor = true;
        if self.search_active {
            self.perform_search();
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

    pub fn can_undo(&self) -> bool {
        self.tab_manager.current_tab().edit_history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.tab_manager.current_tab().edit_history.can_redo()
    }

    pub fn has_selection(&self) -> bool {
        self.selection.as_ref().is_some_and(|s| s.is_active())
    }

    pub fn has_unsaved_changes(&self) -> bool {
        self.tab_manager.current_tab().has_unsaved_changes
    }

    pub fn mark_dirty(&mut self) {
        self.tab_manager.current_tab_mut().has_unsaved_changes = true;
    }

    pub fn current_filename(&self) -> Option<&str> {
        self.tab_manager.current_tab().filename.as_deref()
    }

    // Undo - every edit runs through with_undo, which records it as a line diff
    fn with_undo(&mut self, edit: impl FnOnce(&mut Self)) {
        let before = self.current_buffer().lines.clone();
        let cursor_before = self.cursor_pos;
        let scroll_before = self.scroll_offset.0;

        edit(self);

        if let Some(edit) = Edit::diff_lines(&before, &self.current_buffer().lines) {
            let operation = EditOperation {
                edit,
                cursor_before: to_history_pos(cursor_before),
                cursor_after: to_history_pos(self.cursor_pos),
                scroll_before,
                scroll_after: self.scroll_offset.0,
            };
            self.current_edit_history().push(operation);
            self.mark_dirty();
            self.scroll_to_cursor = true;
        }
    }

    pub fn undo(&mut self) {
        if let Some(op) = self.current_edit_history().undo() {
            op.edit.reverse(&mut self.current_buffer_mut().lines);
            self.cursor_pos = from_history_pos(op.cursor_before);
            self.after_undo_redo();
        }
    }

    pub fn redo(&mut self) {
        if let Some(op) = self.current_edit_history().redo() {
            op.edit.apply(&mut self.current_buffer_mut().lines);
            self.cursor_pos = from_history_pos(op.cursor_after);
            self.after_undo_redo();
        }
    }

    fn after_undo_redo(&mut self) {
        if self.current_buffer().lines.is_empty() {
            self.current_buffer_mut().lines.push(String::new());
        }
        self.selection = None;
        self.clamp_cursor();
        self.mark_dirty();
        self.scroll_to_cursor = true;
    }

    // Insert text at cursor position, replacing the selection if there is one
    pub fn insert_text(&mut self, text: &str) {
        self.with_undo(|state| {
            if let Some(selection) = state.selection.take().filter(|s| s.is_active()) {
                state.delete_selection(selection);
            }
            state.insert_text_raw(text);
        });
    }

    // Insert text at cursor position (grapheme-aware)
    fn insert_text_raw(&mut self, text: &str) {
        let pos = self.cursor_pos;
        let buffer = self.current_buffer_mut();

        // Ensure line exists
        while buffer.lines.len() <= pos.line {
            buffer.lines.push(String::new());
        }

        // Normalize line endings already handled by caller if needed
        // Handle multi-line insert
        if text.contains('\n') {
            let parts: Vec<&str> = text.split('\n').collect();

            // Work with a cloned current line so split_at_grapheme lifetimes are safe
            let current_line = buffer.lines[pos.line].clone();
            let grapheme_count = grapheme_len(&current_line);
            let split_at = pos.column.min(grapheme_count);
            let (before, after) = split_at_grapheme(&current_line, split_at);

            // Replace current line with before + first part
            buffer.lines[pos.line] = format!("{}{}", before, parts[0]);

            // Insert middle/last parts
            for (i, part) in parts.iter().enumerate().skip(1) {
                let mut line_content = part.to_string();
                if i == parts.len() - 1 {
                    // append the remainder of the original line
                    line_content.push_str(after);
                }

                let insert_idx = pos.line + i;
                // Double-check bounds safety dynamically as the vector grows
                if insert_idx <= buffer.lines.len() {
                    buffer.lines.insert(insert_idx, line_content);
                } else {
                    buffer.lines.push(line_content);
                }
            }

            // Set cursor to end of last inserted line (grapheme count)
            let final_line_idx = pos.line + parts.len() - 1;
            let final_col = grapheme_len(&buffer.lines[final_line_idx]);
            self.cursor_pos = TextPosition {
                line: final_line_idx,
                column: final_col,
            };
        } else {
            // Single line insert - use grapheme insert
            let line = &mut buffer.lines[pos.line];
            let grapheme_count = grapheme_len(line);
            let insert_at = pos.column.min(grapheme_count);
            insert_at_grapheme(line, insert_at, text);
            self.cursor_pos = TextPosition {
                line: pos.line,
                column: insert_at + grapheme_len(text),
            };
        }

        self.mark_dirty();
    }

    pub fn move_cursor(&mut self, dx: isize, dy: isize) {
        let line_count = self.current_buffer().lines.len();

        let mut new_line = self.cursor_pos.line as isize + dy;
        new_line = new_line.clamp(0, line_count.saturating_sub(1) as isize);

        self.cursor_pos.line = new_line as usize;

        let line_len = grapheme_len(&self.current_buffer().lines[self.cursor_pos.line]);
        let mut new_col = self.cursor_pos.column as isize + dx;
        new_col = new_col.clamp(0, line_len as isize);

        self.cursor_pos.column = new_col as usize;
    }

    pub fn clamp_cursor(&mut self) {
        let line_count = self.current_buffer().lines.len();
        self.cursor_pos.line = self.cursor_pos.line.min(line_count.saturating_sub(1));
        let line_len = grapheme_len(&self.current_buffer().lines[self.cursor_pos.line]);
        self.cursor_pos.column = self.cursor_pos.column.min(line_len);
    }

    pub fn delete_at_cursor(&mut self) {
        self.with_undo(|state| state.delete_at_cursor_raw());
    }

    pub fn backspace(&mut self) {
        self.with_undo(|state| state.backspace_raw());
    }

    // Delete selection or character at cursor (grapheme-aware)
    fn delete_at_cursor_raw(&mut self) {
        if let Some(selection) = self.selection.take() {
            self.delete_selection(selection);
        } else {
            let pos = self.cursor_pos;
            let buffer = self.current_buffer_mut();
            if pos.line < buffer.lines.len() {
                let line = &mut buffer.lines[pos.line];
                let grapheme_count = grapheme_len(line);
                if pos.column < grapheme_count {
                    let _ = remove_grapheme_at(line, pos.column);
                    self.clamp_cursor();
                    self.mark_dirty();
                } else if pos.line + 1 < buffer.lines.len() {
                    // At the end of the line - join the next line
                    let next_line = buffer.lines.remove(pos.line + 1);
                    buffer.lines[pos.line].push_str(&next_line);
                    self.mark_dirty();
                }
            }
        }
    }

    // Backspace - delete character before cursor
    fn backspace_raw(&mut self) {
        if let Some(selection) = self.selection.take() {
            self.delete_selection(selection);
        } else if self.cursor_pos.column > 0 {
            let pos = self.cursor_pos;
            let buffer = self.current_buffer_mut();
            if pos.line < buffer.lines.len() {
                let line = &mut buffer.lines[pos.line];
                let grapheme_count = grapheme_len(line);
                if pos.column <= grapheme_count && pos.column > 0 {
                    let _ = remove_grapheme_at(line, pos.column - 1);
                    self.cursor_pos.column -= 1;
                    self.mark_dirty();
                }
            }
        } else if self.cursor_pos.line > 0 {
            let pos = self.cursor_pos;
            let buffer = self.current_buffer_mut();
            let current_line = buffer.lines[pos.line].clone();
            let prev_line_len = grapheme_len(&buffer.lines[pos.line - 1]);
            // Merge lines
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
            // Single line deletion (grapheme-aware)
            if let Some(line) = buffer.lines.get_mut(start.line) {
                let line_graphemes = grapheme_len(line);
                let start_col = start.column.min(line_graphemes);
                let end_col = end.column.min(line_graphemes);
                let byte_start = grapheme_to_byte_idx(line, start_col);
                let byte_end = grapheme_to_byte_idx(line, end_col);
                line.drain(byte_start..byte_end);
            }
        } else {
            // Multi-line deletion (grapheme-aware)
            let before_text = if let Some(line) = buffer.lines.get(start.line) {
                grapheme_slice(line, 0, start.column.min(grapheme_len(line)))
            } else {
                String::new()
            };

            let after_text = if let Some(line) = buffer.lines.get(end.line) {
                let gcount = grapheme_len(line);
                grapheme_slice(line, end.column.min(gcount), gcount)
            } else {
                String::new()
            };

            // Remove lines in range
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

    // Save current file (false if it has no file yet and needs save as)
    pub fn save(&mut self) -> Result<bool, std::io::Error> {
        let index = self.tab_manager.active_tab_index;
        self.tab_manager.save_tab(index)
    }

    // Save as new file
    pub fn save_as(&mut self, path: &str) -> Result<(), std::io::Error> {
        let index = self.tab_manager.active_tab_index;
        self.tab_manager.save_tab_as(index, path)
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

    pub fn selected_text(&self) -> Option<String> {
        let selection = self.selection.as_ref().filter(|s| s.is_active())?;
        let (start, end) = selection.get_range();
        Some(self.extract_text_range(start, end))
    }

    // Get the text from last copy operation
    pub fn get_clipboard_text(&self) -> Option<&str> {
        self.clipboard_text.as_deref()
    }

    // Cut selection to clipboard
    pub fn cut_selection(&mut self) {
        self.copy_selection();
        self.with_undo(|state| {
            if let Some(selection) = state.selection.take() {
                if selection.is_active() {
                    state.delete_selection(selection);
                }
            }
        });
    }

    // Paste from clipboard using arboard
    pub fn paste_from_clipboard(&mut self) {
        // Try arboard first
        let text = if let Ok(mut clipboard) = arboard::Clipboard::new() {
            clipboard.get_text().ok()
        } else {
            None
        };

        // Fall back to internal clipboard if arboard fails
        let text = text.or_else(|| self.clipboard_text.clone());

        if let Some(mut text) = text {
            // Normalize line endings for consistent pasting
            text = text.replace("\r\n", "\n").replace('\r', "\n");
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
            .map(|l| l.chars().count())
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
        let matches = find_all_occurrences(&self.current_buffer().lines, &self.search_query);
        if matches.is_empty() {
            self.search_state = None;
            return;
        }

        // Find the match closest to current cursor (or to the start of the current match)
        let cur = self
            .selection
            .as_ref()
            .filter(|s| s.is_active())
            .map(|s| s.get_range().0)
            .unwrap_or(self.cursor_pos);
        let idx = find_closest_match(&matches, cur.line, cur.column);

        let mut search_state = SearchState::new(self.search_query.clone(), matches);
        search_state.current_match_idx = idx;
        self.search_state = Some(search_state);
        self.jump_to_current_match();
    }

    pub fn next_search_match(&mut self) {
        match self.search_state.as_mut() {
            Some(search_state) => search_state.next_match(),
            None => return self.perform_search(),
        }
        self.jump_to_current_match();
    }

    pub fn prev_search_match(&mut self) {
        match self.search_state.as_mut() {
            Some(search_state) => search_state.prev_match(),
            None => return self.perform_search(),
        }
        self.jump_to_current_match();
    }

    fn jump_to_current_match(&mut self) {
        let Some(m) = self.search_state.as_ref().and_then(|s| s.current_match()) else {
            return;
        };
        let (line, col, end_col) = (m.line, m.column, m.column + m.length);
        self.cursor_pos = TextPosition {
            line,
            column: end_col,
        };
        self.selection = Some(Selection {
            anchor: TextPosition { line, column: col },
            cursor: TextPosition {
                line,
                column: end_col,
            },
        });
        // Scroll to keep match visible
        self.scroll_to_cursor = true;
    }

    pub fn clear_search(&mut self) {
        self.search_active = false;
        self.search_query.clear();
        self.search_state = None;
        self.selection = None;
    }

    // Ensure cursor is visible within the current viewport height (in lines).
    // visible_rows should be the number of text rows visible in the editor area.
    pub(in crate::gui) fn ensure_cursor_visible(&mut self, visible_rows: Option<usize>) {
        let line = self.cursor_pos.line;
        // If cursor above viewport, jump viewport up
        if line < self.scroll_offset.0 {
            self.scroll_offset.0 = line;
            return;
        }

        // If caller provided visible_rows, ensure bottom bound as well
        if let Some(visible_rows) = visible_rows {
            if visible_rows == 0 {
                return;
            }

            if line >= self.scroll_offset.0 + visible_rows {
                // place cursor roughly in middle of viewport
                self.scroll_offset.0 = line.saturating_sub(visible_rows / 2);
            }
        }
    }
}

// The edit history stores tui caret positions, the gui keeps line/column in them
fn to_history_pos(pos: TextPosition) -> Position {
    Position {
        x: pos.column.min(u16::MAX as usize) as u16,
        y: pos.line.min(u16::MAX as usize) as u16,
    }
}

fn from_history_pos(pos: Position) -> TextPosition {
    TextPosition {
        line: pos.y as usize,
        column: pos.x as usize,
    }
}
