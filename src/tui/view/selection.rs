// selection module responsible for handling selection logic shared between mouse and keyboard
use super::{View, helpers};
use crate::tui::{terminal::Terminal, caret::{Caret, Position}};
use crate::core::selection::Selection;
use std::io::Error;

pub fn move_with_selection(view: &mut View, direction: &str, caret: &mut Caret) -> Result<(), Error> {
    let current_pos = helpers::get_current_text_pos(view, caret);
    
    // Initialize selection if it doesn't exist
    if view.selection.is_none() {
        view.selection = Some(Selection::new(current_pos));
    }
    
    // Perform the movement
    perform_movement(view, direction, caret)?;
    
    // Update selection cursor to new position
    let new_pos = helpers::get_current_text_pos(view, caret);
    if let Some(ref mut selection) = view.selection {
        selection.update_cursor(new_pos);
    }
    
    view.render(caret)?;
    Ok(())
}

pub fn move_without_selection(view: &mut View, direction: &str, caret: &mut Caret) -> Result<(), Error> {
    // Clear selection
    view.selection = None;
    
    // Perform movement
    perform_movement(view, direction, caret)?;
    
    view.render(caret)?;
    Ok(())
}

fn perform_movement(view: &mut View, direction: &str, caret: &mut Caret) -> Result<(), Error> {
    match direction {
        "left" => {
            let pos = caret.get_position();
            let buffer_line_idx = (pos.y.saturating_sub(Position::HEADER)) as usize + view.scroll_offset;
            
            // If at beginning of line content, try to move to end of previous line
            if pos.x <= Position::MARGIN && buffer_line_idx > 0 {
                let prev_line_len = view.buffer.lines.get(buffer_line_idx - 1)
                    .map(|l| l.len())
                    .unwrap_or(0);
                
                if pos.y > Position::HEADER {
                    // Move to end of previous line
                    caret.move_to(Position { 
                        x: Position::MARGIN + prev_line_len as u16, 
                        y: pos.y - 1 
                    })?;
                } else if view.scroll_offset > 0 {
                    // Scroll up
                    view.scroll_offset -= 1;
                    view.render(caret)?;
                    caret.move_to(Position { 
                        x: Position::MARGIN + prev_line_len as u16, 
                        y: Position::HEADER 
                    })?;
                }
            } else {
                // Normal left movement using caret
                let new_offset = caret.move_left(view.scroll_offset)?;
                view.scroll_offset = new_offset;
            }
        },
        "right" => {
            let pos = caret.get_position();
            let buffer_line_idx = (pos.y.saturating_sub(Position::HEADER)) as usize + view.scroll_offset;
            
            // Check if we can move right on current line
            if let Some(line) = view.buffer.lines.get(buffer_line_idx) {
                let char_pos = (pos.x as usize).saturating_sub(Position::MARGIN as usize);
                let line_end = Position::MARGIN + line.len() as u16;
                let size = Terminal::get_size()?;
                
                // If we're within the line content, use caret's move_right
                if pos.x < line_end && pos.x < size.width - 1 {
                    let new_offset = caret.move_right(view.scroll_offset, view.buffer.lines.len())?;
                    view.scroll_offset = new_offset;
                    return Ok(());
                }
                
                // If at end of line content, try to move to next line
                if char_pos >= line.len() && buffer_line_idx + 1 < view.buffer.lines.len() {
                    if pos.y < size.height - 2 {
                        // Move to start of next line
                        caret.move_to(Position { x: Position::MARGIN, y: pos.y + 1 })?;
                    } else {
                        // Scroll down and stay at same y position
                        view.scroll_offset += 1;
                        view.render(caret)?;
                        caret.move_to(Position { x: Position::MARGIN, y: pos.y })?;
                    }
                }
            }
        },
        "up" => {
            let new_offset = caret.move_up(view.scroll_offset)?;
            view.scroll_offset = new_offset;
            view.render(caret)?;
            view.clamp_cursor_to_line(caret)?;
        },
        "down" => {
            let new_offset = caret.move_down(view.scroll_offset, view.buffer.lines.len())?;
            view.scroll_offset = new_offset;
            view.render(caret)?;
            view.clamp_cursor_to_line(caret)?;
        },
        "top" => {
            let new_offset = caret.move_top()?;
            view.scroll_offset = new_offset;
            view.render(caret)?;
            caret.move_to(Position { x: Position::MARGIN, y: Position::HEADER })?;
        },
        "bottom" => {
            let size = Terminal::get_size()?;
            let visible_rows = (size.height.saturating_sub(Position::HEADER + 1)) as usize;
            
            // Find last non-empty line
            let last_line = view.buffer.lines.iter()
                .rposition(|line| !line.is_empty())
                .unwrap_or(0);
            
            // Calculate scroll offset to show last line
            if last_line >= visible_rows {
                view.scroll_offset = last_line - visible_rows + 1;
            } else {
                view.scroll_offset = 0;
            }
            
            view.render(caret)?;
            
            // Use caret's move_bottom to position at bottom of visible area
            caret.move_bottom()?;
            
            // Then clamp to actual line length
            view.clamp_cursor_to_line(caret)?;
        },
        "max_left" => {
            caret.move_max_left()?;
        },
        "max_right" => {
            let pos = caret.get_position();
            let buffer_line_idx = (pos.y.saturating_sub(Position::HEADER)) as usize + view.scroll_offset;
            
            // Get the actual line length to determine proper max right position
            if let Some(line) = view.buffer.lines.get(buffer_line_idx) {
                let size = Terminal::get_size()?;
                let line_end = Position::MARGIN + line.len() as u16;
                let max_x = line_end.min(size.width - 1);
                caret.move_to(Position { x: max_x, y: pos.y })?;
            } else {
                // If no line content, just use caret's move_max_right
                caret.move_max_right()?;
            }
        },
        _ => {}
    }
    Ok(())
}