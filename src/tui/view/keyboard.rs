// keyboard logic responsible for handling keyboard input and text editing
use super::View;
use crate::tui::{terminal::Terminal, caret::{Caret, Position}};
use std::io::Error;

pub fn type_character(view: &mut View, character: char, caret: &mut Caret) -> Result<(), Error> {
    let size = Terminal::get_size()?;
    let position = caret.get_position();
    
    // Clear selection when typing
    view.selection = None;
    
    // Don't allow typing in footer
    if position.y >= size.height - 1 {
        return Ok(());
    }
    
    // Adjust Y coordinate to Buffer Index
    let buffer_line_idx = (position.y.saturating_sub(Position::HEADER)) as usize + view.scroll_offset;

    while view.buffer.lines.len() <= buffer_line_idx {
        view.buffer.lines.push(String::new());
    }

    // Adjust X coordinate to Character Index
    let char_pos = (position.x as usize).saturating_sub(Position::MARGIN as usize);
    
    // If at end of screen width, wrap to next line
    if position.x >= size.width - 1 {
        insert_newline(view, caret)?;
        return type_character(view, character, caret);
    }

    let line = &mut view.buffer.lines[buffer_line_idx];
    if char_pos <= line.len() {
        line.insert(char_pos, character);
    } else {
        line.push(character);
    }

    view.render(caret)?;
    
    // Use caret's move_right to handle cursor movement
    let new_offset = caret.move_right(view.scroll_offset, view.buffer.lines.len())?;
    view.scroll_offset = new_offset;
    
    Ok(())
}

pub fn insert_newline(view: &mut View, caret: &mut Caret) -> Result<(), Error> {
    let size = Terminal::get_size()?;
    let position = caret.get_position();
    
    // Clear selection
    view.selection = None;
    
    // Prevent typing in the absolute last row of the terminal (footer)
    if position.y >= size.height - 1 {
        return Ok(());
    }
    
    let buffer_line_idx = (position.y.saturating_sub(Position::HEADER)) as usize + view.scroll_offset;
    let char_pos = (position.x as usize).saturating_sub(Position::MARGIN as usize);

    // Ensure buffer is large enough
    while view.buffer.lines.len() <= buffer_line_idx {
        view.buffer.lines.push(String::new());
    }
    
    // Split the current line
    let current_line = &mut view.buffer.lines[buffer_line_idx];
    let new_line_content = if char_pos < current_line.len() {
        current_line.split_off(char_pos)
    } else {
        String::new()
    };
    
    // Insert the new line into the buffer
    view.buffer.lines.insert(buffer_line_idx + 1, new_line_content);
    
    // Update the view (must render before moving caret so terminal state matches)
    view.render(caret)?;
    
    // This will handle incrementing scroll_offset if the cursor is at the bottom of the visible area
    view.scroll_offset = caret.move_down(view.scroll_offset, view.buffer.lines.len())?;
    
    // Final render to place cursor correctly
    view.render(caret)?;
    caret.next_line()?;
    Ok(())
}

pub fn delete_char(view: &mut View, caret: &mut Caret) -> Result<(), Error> {
    let pos = caret.get_position();
    let buffer_line_idx = (pos.y.saturating_sub(Position::HEADER)) as usize + view.scroll_offset;
    let char_pos = (pos.x as usize).saturating_sub(Position::MARGIN as usize);
    
    // Clear selection
    view.selection = None;
    
    // Check if we're in a valid line
    if buffer_line_idx >= view.buffer.lines.len() {
        return Ok(());
    }
    
    let line_len = view.buffer.lines[buffer_line_idx].len();
    
    if char_pos < line_len {
        // Delete character at cursor
        view.buffer.lines[buffer_line_idx].remove(char_pos);
        view.render(caret)?;
        caret.move_to(pos)?;
    } else if buffer_line_idx + 1 < view.buffer.lines.len() {
        // At end of line, merge with next line
        let next_line = view.buffer.lines.remove(buffer_line_idx + 1);
        view.buffer.lines[buffer_line_idx].push_str(&next_line);
        view.render(caret)?;
        caret.move_to(pos)?;
    }
    
    Ok(())
}

pub fn backspace(view: &mut View, caret: &mut Caret) -> Result<(), Error> {
    let pos = caret.get_position();
    let buffer_line_idx = (pos.y.saturating_sub(Position::HEADER)) as usize + view.scroll_offset;
    let char_pos = (pos.x as usize).saturating_sub(Position::MARGIN as usize);
    
    // Clear selection
    view.selection = None;
    
    if char_pos > 0 {
        // Delete character before cursor
        if let Some(line) = view.buffer.lines.get_mut(buffer_line_idx) {
            if char_pos <= line.len() {
                line.remove(char_pos - 1);
                view.render(caret)?;
                // Use caret's move_left
                let new_offset = caret.move_left(view.scroll_offset)?;
                view.scroll_offset = new_offset;
            }
        }
    } else if buffer_line_idx > 0 {
        // At beginning of line, merge with previous line
        let prev_line_len = view.buffer.lines[buffer_line_idx - 1].len();
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
    }
    Ok(())
}