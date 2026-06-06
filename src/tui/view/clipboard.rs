// clipboard module with EditOperation tracking
use super::View;
use crate::tui::caret::{Caret, Position};
use crate::tui::terminal::Terminal;
use crate::core::selection::TextPosition;
use crate::core::graphemes::*;
use crate::core::edit_history::{Edit, EditOperation};
use std::io::Error;

pub fn copy_selection(view: &View) -> Result<(), Error> {
    if let Some(ref selection) = view.selection {
        if !selection.is_active() {
            return Ok(());
        }
        
        let (start, end) = selection.get_range();
        let selected_text = extract_text(view, start, end);
        
        // Copy to clipboard
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(selected_text);
        }
    }
    Ok(())
}

pub fn cut_selection(view: &mut View, caret: &mut Caret) -> Result<Option<EditOperation>, Error> {
    // First copy
    copy_selection(view)?;
    
    // Then delete and return the operation
    delete_selection(view, caret)
}

// normalisation for pasteing
fn normalize_line_endings(text: &str) -> String {
    // \r\n first, then bare \r — order matters
    text.replace("\r\n", "\n").replace('\r', "\n")
}

pub fn paste_from_clipboard(view: &mut View, caret: &mut Caret) -> Result<Option<EditOperation>, Error> {
    // Delete selection first if it exists
    if view.selection.is_some() && view.selection.as_ref().unwrap().is_active() {
        delete_selection(view, caret)?;
    }
    
    // Get text from clipboard
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        if let Ok(text) = clipboard.get_text() {
            let normalized = normalize_line_endings(&text);
            return insert_text_at_cursor(view, caret, &normalized);
        }
    }
    
    Ok(None)
}

pub fn delete_selection(view: &mut View, caret: &mut Caret) -> Result<Option<EditOperation>, Error> {
    if let Some(selection) = view.selection.take() {
        if !selection.is_active() {
            view.selection = Some(selection);
            return Ok(None);
        }
        
        let (start, end) = selection.get_range();
        let cursor_before = caret.get_position();
        let scroll_before = view.scroll_offset;
        
        // Extract the text that will be deleted (for undo)
        let deleted_text = extract_text(view, start, end);
        
        // Delete the selected text
        delete_range(view, start, end)?;
        
        // Move cursor to start of deleted range
        let (screen_x, screen_y) = super::helpers::text_to_screen_pos(view, start);
        caret.move_to(Position { x: screen_x, y: screen_y })?;
        
        view.render(caret)?;
        
        // Create edit operation
        let operation = EditOperation {
            edit: Edit::ReplaceRange {
                start_line: start.line,
                start_column: start.column,
                end_line: end.line,
                end_column: end.column,
                old_text: deleted_text,
                new_text: String::new(),
            },
            cursor_before,
            cursor_after: caret.get_position(),
            scroll_before,
            scroll_after: view.scroll_offset,
        };
        
        return Ok(Some(operation));
    }
    
    Ok(None)
}

// Helper: Extract text from selection range
fn extract_text(view: &View, start: TextPosition, end: TextPosition) -> String {
    let mut result = String::new();
    
    if start.line == end.line {
        // Single line selection
        if let Some(line) = view.buffer.lines.get(start.line) {
            let grapheme_count = grapheme_len(line);
            let text = grapheme_slice(
                line,
                start.column.min(grapheme_count),
                end.column.min(grapheme_count)
            );
            result.push_str(&text);
        }
    } else {
        // Multi-line selection
        for line_idx in start.line..=end.line {
            if let Some(line) = view.buffer.lines.get(line_idx) {
                let grapheme_count = grapheme_len(line);
                
                if line_idx == start.line {
                    // First line: from start.column to end
                    let text = grapheme_slice(line, start.column.min(grapheme_count), grapheme_count);
                    result.push_str(&text);
                    result.push('\n');
                } else if line_idx == end.line {
                    // Last line: from beginning to end.column
                    let text = grapheme_slice(line, 0, end.column.min(grapheme_count));
                    result.push_str(&text);
                } else {
                    // Middle lines: entire line
                    result.push_str(line);
                    result.push('\n');
                }
            }
        }
    }
    
    result
}

// Helper: Delete text in range
fn delete_range(view: &mut View, start: TextPosition, end: TextPosition) -> Result<(), Error> {
    if start.line == end.line {
        // Single line deletion
        if let Some(line) = view.buffer.lines.get_mut(start.line) {
            let grapheme_count = grapheme_len(line);
            let start_col = start.column.min(grapheme_count);
            let end_col = end.column.min(grapheme_count);
            
            // Convert to byte indices and remove
            let byte_start = grapheme_to_byte_idx(line, start_col);
            let byte_end = grapheme_to_byte_idx(line, end_col);
            line.drain(byte_start..byte_end);
        }
    } else {
        // Multi-line deletion
        // Get the text before selection on first line
        let before_text = if let Some(line) = view.buffer.lines.get(start.line) {
            grapheme_slice(line, 0, start.column.min(grapheme_len(line)))
        } else {
            String::new()
        };
        
        // Get the text after selection on last line
        let after_text = if let Some(line) = view.buffer.lines.get(end.line) {
            let grapheme_count = grapheme_len(line);
            grapheme_slice(line, end.column.min(grapheme_count), grapheme_count)
        } else {
            String::new()
        };
        
        // Merge before and after text
        let merged_line = format!("{}{}", before_text, after_text);
        
        // Remove all lines in range
        for _ in start.line..=end.line {
            if start.line < view.buffer.lines.len() {
                view.buffer.lines.remove(start.line);
            }
        }
        
        // Insert merged line
        view.buffer.lines.insert(start.line, merged_line);
    }
    
    Ok(())
}

// Helper: Insert text at cursor position and return EditOperation
fn insert_text_at_cursor(view: &mut View, caret: &mut Caret, text: &str) -> Result<Option<EditOperation>, Error> {
    use crate::tui::caret::Position;
    
    let pos = caret.get_position();
    let cursor_before = pos;
    let scroll_before = view.scroll_offset;
    
    let buffer_line_idx = (pos.y.saturating_sub(Position::HEADER)) as usize + view.scroll_offset;
    let char_pos = (pos.x as usize).saturating_sub(Position::MARGIN as usize);
    
    // Ensure line exists
    while view.buffer.lines.len() <= buffer_line_idx {
        view.buffer.lines.push(String::new());
    }
    
    // Check if text contains newlines
    if text.contains('\n') {
        // Multi-line paste
        let lines: Vec<&str> = text.split('\n').collect();
        
        if lines.is_empty() {
            return Ok(None);
        }
        
        // Split current line at cursor (using grapheme position)
        let current_line = &view.buffer.lines[buffer_line_idx];
        let grapheme_count = grapheme_len(current_line);
        let grapheme_pos = char_pos.min(grapheme_count);
        
        let (before, after) = split_at_grapheme(current_line, grapheme_pos);
        let before = before.to_string();
        let after = after.to_string();
        
        // Update first line: before + first pasted line
        view.buffer.lines[buffer_line_idx] = format!("{}{}", before, lines[0]);
        
        // Insert all middle and last lines
        let mut insert_idx = buffer_line_idx + 1;
        for i in 1..lines.len() {
            let line_content = if i == lines.len() - 1 {
                format!("{}{}", lines[i], after)
            } else {
                lines[i].to_string()
            };
            
            if insert_idx < view.buffer.lines.len() {
                view.buffer.lines.insert(insert_idx, line_content);
            } else {
                view.buffer.lines.push(line_content);
            }
            insert_idx += 1;
        }
        
        // Calculate final position (in graphemes)
        let final_buffer_line = buffer_line_idx + lines.len() - 1;
        let final_buffer_col = grapheme_len(lines[lines.len() - 1]);
        
        // Update scroll_offset
        let size = Terminal::get_size()?;
        let visible_rows = size.height.saturating_sub(Position::HEADER + 1) as usize;
        
        if final_buffer_line >= view.scroll_offset + visible_rows {
            view.scroll_offset = final_buffer_line.saturating_sub(visible_rows / 2);
        } else if final_buffer_line < view.scroll_offset {
            view.scroll_offset = final_buffer_line;
        }
        
        view.render(caret)?;
        
        let final_text_pos = TextPosition {
            line: final_buffer_line,
            column: final_buffer_col,
        };
        let (screen_x, screen_y) = super::helpers::text_to_screen_pos(view, final_text_pos);
        caret.move_to(Position { x: screen_x, y: screen_y })?;
        
        // Create edit operation for multi-line paste
        Ok(Some(EditOperation {
            edit: Edit::ReplaceRange {
                start_line: buffer_line_idx,
                start_column: grapheme_pos,
                end_line: buffer_line_idx,
                end_column: grapheme_pos,
                old_text: String::new(),
                new_text: text.to_string(),
            },
            cursor_before,
            cursor_after: caret.get_position(),
            scroll_before,
            scroll_after: view.scroll_offset,
        }))
    } else {
        // Single line paste
        let line = &mut view.buffer.lines[buffer_line_idx];
        let grapheme_count = grapheme_len(line);
        let grapheme_pos = char_pos.min(grapheme_count);
        
        insert_at_grapheme(line, grapheme_pos, text);
        
        view.render(caret)?;
        
        let size = Terminal::get_size()?;
        let text_grapheme_len = grapheme_len(text);
        let new_x = (pos.x + text_grapheme_len as u16).min(size.width.saturating_sub(1));
        caret.move_to(Position { x: new_x, y: pos.y })?;
        
        // Create edit operation for single-line paste
        Ok(Some(EditOperation {
            edit: Edit::InsertText {
                line: buffer_line_idx,
                column: grapheme_pos,
                text: text.to_string(),
            },
            cursor_before,
            cursor_after: caret.get_position(),
            scroll_before,
            scroll_after: view.scroll_offset,
        }))
    }
}
