// mouse module responsible for handling mouse events
use super::{
    View,
    helpers::{screen_to_text_pos, text_to_screen_pos},
};
use crate::tui::caret::{Caret, Position};
use crate::core::selection::{Selection, TextPosition};
use std::io::Error;

pub fn handle_down(view: &mut View, screen_x: u16, screen_y: u16, caret: &mut Caret) -> Result<(), Error> {
    let pos = screen_to_text_pos(view, screen_x, screen_y)?;
    
    // Start new selection
    view.selection = Some(Selection::new(pos));
    view.is_dragging = true;
    
    let (sx, sy) = text_to_screen_pos(view, pos);
    caret.move_to(Position { x: sx, y: sy })?;
    
    view.render(caret)?;
    Ok(())
}

pub fn handle_drag(view: &mut View, screen_x: u16, screen_y: u16, caret: &mut Caret) -> Result<(), Error> {
    if !view.is_dragging {
        return Ok(());
    }
    
    let pos = screen_to_text_pos(view, screen_x, screen_y)?;
    
    if let Some(ref mut selection) = view.selection {
        selection.update_cursor(pos);
    }
    
    let (sx, sy) = text_to_screen_pos(view, pos);
    caret.move_to(Position { x: sx, y: sy })?;
    
    view.render(caret)?;
    Ok(())
}

pub fn handle_up(view: &mut View, _screen_x: u16, _screen_y: u16, caret: &mut Caret) -> Result<(), Error> {
    view.is_dragging = false;
    
    // If selection is empty (just a click), clear it
    if let Some(ref selection) = view.selection {
        if !selection.is_active() {
            view.selection = None;
        }
    }
    
    view.render(caret)?;
    Ok(())
}

pub fn handle_double_click(view: &mut View, screen_x: u16, screen_y: u16, caret: &mut Caret) -> Result<(), Error> {
    let pos = screen_to_text_pos(view, screen_x, screen_y)?;
    
    if let Some(line) = view.buffer.lines.get(pos.line) {
        let (start, end) = find_word_boundaries(line, pos.column);
        
        let start_pos = TextPosition { line: pos.line, column: start };
        let end_pos = TextPosition { line: pos.line, column: end };
        
        view.selection = Some(Selection {
            anchor: start_pos,
            cursor: end_pos,
        });
        
        let (sx, sy) = text_to_screen_pos(view, end_pos);
        caret.move_to(Position { x: sx, y: sy })?;
    }
    
    view.render(caret)?;
    Ok(())
}

pub fn handle_triple_click(view: &mut View, screen_x: u16, screen_y: u16, caret: &mut Caret) -> Result<(), Error> {
    let pos = screen_to_text_pos(view, screen_x, screen_y)?;
    
    if let Some(line) = view.buffer.lines.get(pos.line) {
        let start_pos = TextPosition { line: pos.line, column: 0 };
        let end_pos = TextPosition { line: pos.line, column: line.len() };
        
        view.selection = Some(Selection {
            anchor: start_pos,
            cursor: end_pos,
        });
        
        let (sx, sy) = text_to_screen_pos(view, end_pos);
        caret.move_to(Position { x: sx, y: sy })?;
    }
    
    view.render(caret)?;
    Ok(())
}

fn find_word_boundaries(line: &str, col: usize) -> (usize, usize) {
    let chars: Vec<char> = line.chars().collect();
    
    if col >= chars.len() || chars.is_empty() {
        return (col, col);
    }
    
    let is_word_char = |c: char| c.is_alphanumeric() || c == '_';
    
    if !is_word_char(chars[col]) {
        return (col, col + 1);
    }
    
    let mut start = col;
    while start > 0 && is_word_char(chars[start - 1]) {
        start -= 1;
    }
    
    let mut end = col;
    while end < chars.len() && is_word_char(chars[end]) {
        end += 1;
    }
    
    (start, end)
}