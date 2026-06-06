// keyboard logic with edit tracking for undo/redo
use super::View;
use crate::tui::{
    terminal::Terminal, 
    caret::{Caret, Position}
};
use crate::core::graphemes::*;
use crate::core::edit_history::{Edit, EditOperation};
use std::io::Error;

pub fn type_character(view: &mut View, character: char, caret: &mut Caret) -> Result<Option<EditOperation>, Error> {    
    // Delete selection first if it exists
    if view.selection.is_some() && view.selection.as_ref().unwrap().is_active() {
        super::clipboard::delete_selection(view, caret)?;
    }
    
    let size = Terminal::get_size()?;
    let position = caret.get_position();
    
    if position.y >= size.height - 1 {
        return Ok(None);
    }
    
    let buffer_line_idx = (position.y.saturating_sub(Position::HEADER)) as usize + view.scroll_offset;
    let char_pos = (position.x as usize).saturating_sub(Position::MARGIN as usize);
    
    let cursor_before = position;
    let scroll_before = view.scroll_offset;

    while view.buffer.lines.len() <= buffer_line_idx {
        view.buffer.lines.push(String::new());
    }

    // If at end of screen width, wrap to next line
    if position.x >= size.width - 1 {
        insert_newline(view, caret)?;
        return type_character(view, character, caret);
    }

    // Insert the character at grapheme position
    let line = &mut view.buffer.lines[buffer_line_idx];
    let grapheme_count = grapheme_len(line);
    let grapheme_pos = char_pos.min(grapheme_count);
    
    insert_at_grapheme(line, grapheme_pos, &character.to_string());

    view.render(caret)?;
    
    let new_offset = caret.move_right(view.scroll_offset, view.buffer.lines.len())?;
    view.scroll_offset = new_offset;
    
    Ok(Some(EditOperation {
        edit: Edit::InsertText {
            line: buffer_line_idx,
            column: grapheme_pos,
            text: character.to_string(),
        },
        cursor_before,
        cursor_after: caret.get_position(),
        scroll_before,
        scroll_after: view.scroll_offset,
    }))
}

pub fn insert_newline(view: &mut View, caret: &mut Caret) -> Result<Option<EditOperation>, Error> {
    if view.selection.is_some() && view.selection.as_ref().unwrap().is_active() {
        super::clipboard::delete_selection(view, caret)?;
    }
    
    let size = Terminal::get_size()?;
    let position = caret.get_position();
    
    if position.y >= size.height - 1 {
        return Ok(None);
    }
    
    let cursor_before = position;
    let scroll_before = view.scroll_offset;
    
    let buffer_line_idx = (position.y.saturating_sub(Position::HEADER)) as usize + view.scroll_offset;
    let char_pos = (position.x as usize).saturating_sub(Position::MARGIN as usize);

    while view.buffer.lines.len() <= buffer_line_idx {
        view.buffer.lines.push(String::new());
    }
    
    // Split at grapheme position
    let current_line = &view.buffer.lines[buffer_line_idx];
    let grapheme_count = grapheme_len(current_line);
    let grapheme_pos = char_pos.min(grapheme_count);
    
    let (before, remaining) = split_at_grapheme(current_line, grapheme_pos);
    let remaining_text = remaining.to_string();
    
    // Update current line to only contain text before split
    view.buffer.lines[buffer_line_idx] = before.to_string();
    
    // Insert new line with remaining text
    view.buffer.lines.insert(buffer_line_idx + 1, remaining_text.clone());
    
    // Render first, then move cursor
    view.render(caret)?;
    
    // Check if we need to scroll
    if position.y < size.height - 2 {
        // Room to move down on screen
        caret.next_line()?;
    } else {
        // Need to scroll
        view.scroll_offset += 1;
        view.render(caret)?;
        caret.next_line()?;
    }
    
    Ok(Some(EditOperation {
        edit: Edit::InsertLine {
            line: buffer_line_idx,
            remaining_text,
        },
        cursor_before,
        cursor_after: caret.get_position(),
        scroll_before,
        scroll_after: view.scroll_offset,
    }))
}

pub fn delete_char(view: &mut View, caret: &mut Caret) -> Result<Option<EditOperation>, Error> {
    if view.selection.is_some() && view.selection.as_ref().unwrap().is_active() {
        return super::clipboard::delete_selection(view, caret);
    }
    
    let pos = caret.get_position();
    let cursor_before = pos;
    let scroll_before = view.scroll_offset;
    
    let buffer_line_idx = (pos.y.saturating_sub(Position::HEADER)) as usize + view.scroll_offset;
    let char_pos = (pos.x as usize).saturating_sub(Position::MARGIN as usize);
    
    if buffer_line_idx >= view.buffer.lines.len() {
        return Ok(None);
    }
    
    let line = &mut view.buffer.lines[buffer_line_idx];
    let grapheme_count = grapheme_len(line);
    
    if char_pos < grapheme_count {
        // Delete grapheme at cursor
        let deleted = remove_grapheme_at(line, char_pos).unwrap_or_default();
        view.render(caret)?;
        caret.move_to(pos)?;
        
        Ok(Some(EditOperation {
            edit: Edit::DeleteText {
                line: buffer_line_idx,
                column: char_pos,
                text: deleted,
            },
            cursor_before,
            cursor_after: pos,
            scroll_before,
            scroll_after: view.scroll_offset,
        }))
    } else if buffer_line_idx + 1 < view.buffer.lines.len() {
        // At end of line, merge with next line
        let next_line = view.buffer.lines.remove(buffer_line_idx + 1);
        let first_line_end = grapheme_len(&view.buffer.lines[buffer_line_idx]);
        view.buffer.lines[buffer_line_idx].push_str(&next_line);
        view.render(caret)?;
        caret.move_to(pos)?;
        
        Ok(Some(EditOperation {
            edit: Edit::JoinLines {
                line: buffer_line_idx,
                first_line_end,
            },
            cursor_before,
            cursor_after: pos,
            scroll_before,
            scroll_after: view.scroll_offset,
        }))
    } else {
        Ok(None)
    }
}

pub fn backspace(view: &mut View, caret: &mut Caret) -> Result<Option<EditOperation>, Error> {
    if view.selection.is_some() && view.selection.as_ref().unwrap().is_active() {
        return super::clipboard::delete_selection(view, caret);
    }
    
    let pos = caret.get_position();
    let cursor_before = pos;
    let scroll_before = view.scroll_offset;
    
    let buffer_line_idx = (pos.y.saturating_sub(Position::HEADER)) as usize + view.scroll_offset;
    let char_pos = (pos.x as usize).saturating_sub(Position::MARGIN as usize);
    
    if char_pos > 0 {
        // Delete grapheme before cursor
        if let Some(line) = view.buffer.lines.get_mut(buffer_line_idx) {
            let grapheme_count = grapheme_len(line);
            if char_pos <= grapheme_count {
                let deleted = remove_grapheme_at(line, char_pos - 1).unwrap_or_default();
                view.render(caret)?;
                let new_offset = caret.move_left(view.scroll_offset)?;
                view.scroll_offset = new_offset;
                
                return Ok(Some(EditOperation {
                    edit: Edit::DeleteText {
                        line: buffer_line_idx,
                        column: char_pos - 1,
                        text: deleted,
                    },
                    cursor_before,
                    cursor_after: caret.get_position(),
                    scroll_before,
                    scroll_after: view.scroll_offset,
                }));
            }
        }
    } else if buffer_line_idx > 0 {
        // At beginning of line, merge with previous line
        let prev_line_len = grapheme_len(&view.buffer.lines[buffer_line_idx - 1]);
        let current_line_content = view.buffer.lines[buffer_line_idx].clone();
        
        view.buffer.lines[buffer_line_idx - 1].push_str(&current_line_content);
        view.buffer.lines.remove(buffer_line_idx);
        
        // Move cursor to end of previous line
        if pos.y > Position::HEADER {
            view.render(caret)?;
            caret.move_to(Position { 
                x: Position::MARGIN + prev_line_len as u16, 
                y: pos.y - 1 
            })?;
        } else if view.scroll_offset > 0 {
            view.scroll_offset -= 1;
            view.render(caret)?;
            caret.move_to(Position { 
                x: Position::MARGIN + prev_line_len as u16, 
                y: Position::HEADER 
            })?;
        }
        
        return Ok(Some(EditOperation {
            edit: Edit::DeleteLine {
                line: buffer_line_idx,
                content: current_line_content,
                prev_line_end_len: prev_line_len,
            },
            cursor_before,
            cursor_after: caret.get_position(),
            scroll_before,
            scroll_after: view.scroll_offset,
        }));
    }
    
    Ok(None)
}