// selection module responsible for handling selection logic shared between mouse and keyboard
use super::{View, helpers};
use crate::tui::caret::{Caret, Position};
use crate::core::wrap;
use crate::core::selection::{Selection, TextPosition};
use crate::core::graphemes::*;
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
    let pos = helpers::get_current_text_pos(view, caret);
    let line_len = |view: &View, line: usize| grapheme_len(view.line(line));
    let lines = &view.buffer.lines;
    let cols = view.wrap_cols;

    let target = match direction {
        "left" if pos.column > 0 => TextPosition { line: pos.line, column: pos.column - 1 },
        "left" if pos.line > 0 => TextPosition { line: pos.line - 1, column: line_len(view, pos.line - 1) },
        "right" if pos.column < line_len(view, pos.line) => {
            TextPosition { line: pos.line, column: pos.column + 1 }
        }
        "right" if pos.line + 1 < lines.len() => TextPosition { line: pos.line + 1, column: 0 },
        // Up/down move one visual row, so they walk through wrapped lines row by row
        "up" | "down" => {
            let here = wrap::row_pos_of(lines, cols, pos);
            let next = if direction == "up" {
                wrap::step_back(lines, cols, here, 1)
            } else {
                wrap::step_forward(lines, cols, here, 1)
            };
            if next == here {
                return Ok(());
            }
            let x = caret.get_position().x.saturating_sub(Position::MARGIN) as usize;
            TextPosition { line: next.line, column: helpers::col_at_x(view, next, x) }
        }
        "top" => TextPosition { line: 0, column: 0 },
        "bottom" => {
            let last_line = lines.iter().rposition(|line| !line.is_empty()).unwrap_or(0);
            TextPosition { line: last_line, column: line_len(view, last_line) }
        }
        "max_left" => TextPosition { line: pos.line, column: 0 },
        "max_right" => TextPosition { line: pos.line, column: line_len(view, pos.line) },
        _ => return Ok(()),
    };

    helpers::place_caret(view, caret, target)
}

pub fn select_all(view: &mut View, caret: &mut Caret) -> Result<(), Error> {
    // Find last non-empty line
    let last_line_idx = view.buffer.lines.iter()
        .rposition(|line| !line.is_empty())
        .unwrap_or(0);
    
    let last_line_len = view.buffer.lines.get(last_line_idx)
        .map(|l| grapheme_len(l))
        .unwrap_or(0);
    
    let start = TextPosition { line: 0, column: 0 };
    let end = TextPosition { line: last_line_idx, column: last_line_len };
    
    view.selection = Some(Selection {
        anchor: start,
        cursor: end,
    });
    
    helpers::place_caret(view, caret, end)?;
    
    view.render(caret)?;
    Ok(())
}