// clipboard module responsible for clipboard operations
use super::View;
use crate::tui::caret::{Caret, Position};
use crate::tui::terminal::Terminal;
use crate::core::selection::TextPosition;
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

pub fn cut_selection(view: &mut View, caret: &mut Caret) -> Result<(), Error> {
    // First copy
    copy_selection(view)?;
    
    // Then delete
    delete_selection(view, caret)?;
    
    Ok(())
}

pub fn paste_from_clipboard(view: &mut View, caret: &mut Caret) -> Result<(), Error> {
    // Delete selection first if it exists
    if view.selection.is_some() && view.selection.as_ref().unwrap().is_active() {
        delete_selection(view, caret)?;
    }
    
    // Get text from clipboard
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        if let Ok(text) = clipboard.get_text() {
            insert_text_at_cursor(view, caret, &text)?;
        }
    }
    
    Ok(())
}

pub fn delete_selection(view: &mut View, caret: &mut Caret) -> Result<(), Error> {
    if let Some(selection) = view.selection.take() {
        if !selection.is_active() {
            view.selection = Some(selection);
            return Ok(());
        }
        
        let (start, end) = selection.get_range();
        
        // Delete the selected text
        delete_range(view, start, end)?;
        
        // Move cursor to start of deleted range
        let (screen_x, screen_y) = super::helpers::text_to_screen_pos(view, start);
        if start.line < view.scroll_offset {
            view.scroll_offset = start.line;
        }
        caret.move_to(Position { x: screen_x, y: screen_y })?;
        
        view.needs_redraw = true;
        view.render(caret)?;
    }
    
    Ok(())
}

// Helper: Extract text from selection range
fn extract_text(view: &View, start: TextPosition, end: TextPosition) -> String {
    let mut result = String::new();
    
    if start.line == end.line {
        // Single line selection
        if let Some(line) = view.buffer.lines.get(start.line) {
            let chars: Vec<char> = line.chars().collect();
            let text: String = chars[start.column.min(chars.len())..end.column.min(chars.len())]
                .iter()
                .collect();
            result.push_str(&text);
        }
    } else {
        // Multi-line selection
        for line_idx in start.line..=end.line {
            if let Some(line) = view.buffer.lines.get(line_idx) {
                let chars: Vec<char> = line.chars().collect();
                
                if line_idx == start.line {
                    // First line: from start.column to end
                    let text: String = chars[start.column.min(chars.len())..].iter().collect();
                    result.push_str(&text);
                    result.push('\n');
                } else if line_idx == end.line {
                    // Last line: from beginning to end.column
                    let text: String = chars[..end.column.min(chars.len())].iter().collect();
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
            let mut chars: Vec<char> = line.chars().collect();
            chars.drain(start.column..end.column.min(chars.len()));
            *line = chars.into_iter().collect();
        }
    } else {
        // Multi-line deletion
        
        // Get the text before selection on first line
        let before_text = if let Some(line) = view.buffer.lines.get(start.line) {
            let chars: Vec<char> = line.chars().collect();
            chars[..start.column.min(chars.len())].iter().collect::<String>()
        } else {
            String::new()
        };
        
        // Get the text after selection on last line
        let after_text = if let Some(line) = view.buffer.lines.get(end.line) {
            let chars: Vec<char> = line.chars().collect();
            chars[end.column.min(chars.len())..].iter().collect::<String>()
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
    
    // Ensure the buffer is never truly empty
    if view.buffer.lines.is_empty() {
        view.buffer.lines.push(String::new());
    }
    
    Ok(())
}

// Helper: Insert text at cursor position
fn insert_text_at_cursor(view: &mut View, caret: &mut Caret, text: &str) -> Result<(), Error> {
    let pos = caret.get_position();
    let buffer_line_idx = (pos.y.saturating_sub(Position::HEADER)) as usize + view.scroll_offset;
    let char_pos = (pos.x as usize).saturating_sub(Position::MARGIN as usize);
    
    // Ensure line exists
    while view.buffer.lines.len() <= buffer_line_idx {
        view.buffer.lines.push(String::new());
    }
    
    // Check if text contains newlines
    if text.contains('\n') {
        // Multi-line paste - use split to preserve empty lines
        let lines: Vec<&str> = text.split('\n').collect();
        
        if lines.is_empty() {
            return Ok(());
        }
        
        // Split current line at cursor
        let current_line = view.buffer.lines[buffer_line_idx].clone();
        let before: String = current_line.chars().take(char_pos).collect();
        let after: String = current_line.chars().skip(char_pos).collect();
        
        // Update first line: before + first pasted line
        view.buffer.lines[buffer_line_idx] = format!("{}{}", before, lines[0]);
        
        // Insert all middle and last lines
        let mut insert_idx = buffer_line_idx + 1;
        for i in 1..lines.len() {
            let line_content = if i == lines.len() - 1 {
                // Last line gets the 'after' text appended
                format!("{}{}", lines[i], after)
            } else {
                // Middle lines inserted as-is
                lines[i].to_string()
            };
            
            // Ensure we don't exceed buffer capacity
            if insert_idx < view.buffer.lines.len() {
                view.buffer.lines.insert(insert_idx, line_content);
            } else {
                view.buffer.lines.push(line_content);
            }
            insert_idx += 1;
        }
        
        // Calculate final BUFFER position (not screen position yet)
        let final_buffer_line = buffer_line_idx + lines.len() - 1;
        let final_buffer_col = lines[lines.len() - 1].len();
        
        // Update scroll_offset to ensure cursor will be visible
        let size = Terminal::get_size()?;
        let visible_rows = size.height.saturating_sub(Position::HEADER + 1) as usize;
        
        if final_buffer_line >= view.scroll_offset + visible_rows {
            // Cursor would be below visible area - scroll down to center it
            view.scroll_offset = final_buffer_line.saturating_sub(visible_rows / 2);
        } else if final_buffer_line < view.scroll_offset {
            // Cursor would be above visible area - scroll up
            view.scroll_offset = final_buffer_line;
        }
        // else: cursor is already in visible range, keep current scroll_offset
        
        // Render the view with the new scroll_offset
        view.render(caret)?;
        
        // Use the helper function to convert buffer position to screen position
        let final_text_pos = TextPosition {
            line: final_buffer_line,
            column: final_buffer_col,
        };
        let (screen_x, screen_y) = super::helpers::text_to_screen_pos(view, final_text_pos);
        
        // STEP 5: Move caret to the validated screen position
        caret.move_to(Position { x: screen_x, y: screen_y })?;
        
    } else {
        // Single line paste
        let line = &mut view.buffer.lines[buffer_line_idx];
        
        // Insert text at cursor position
        if char_pos <= line.len() {
            line.insert_str(char_pos, text);
        } else {
            // Pad with spaces if cursor is beyond line end
            line.push_str(&" ".repeat(char_pos - line.len()));
            line.push_str(text);
        }
        
        // Render updated view
        view.render(caret)?;
        
        // Move cursor forward by pasted text length
        let size = Terminal::get_size()?;
        let new_x = (pos.x + text.len() as u16).min(size.width.saturating_sub(1));
        caret.move_to(Position { x: new_x, y: pos.y })?;
    }
    
    Ok(())
}