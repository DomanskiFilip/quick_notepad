// mouse module responsible for handling mouse events
use super::{
    helpers::{screen_to_text_pos, text_to_screen_pos},
    View,
};
use crate::core::graphemes::*;
use crate::core::selection::{Selection, TextPosition};
use crate::tui::caret::{Caret, Position};
use crate::tui::terminal::Terminal;
use std::io::Error;

pub fn handle_down(
    view: &mut View,
    screen_x: u16,
    screen_y: u16,
    caret: &mut Caret,
) -> Result<(), Error> {
    let pos = screen_to_text_pos(view, screen_x, screen_y)?;

    // Start new selection
    view.selection = Some(Selection::new(pos));
    view.is_dragging = true;

    let (sx, sy) = text_to_screen_pos(view, pos);
    caret.move_to(Position { x: sx, y: sy })?;

    view.render(caret)?;
    Ok(())
}

pub fn handle_drag(
    view: &mut View,
    screen_x: u16,
    screen_y: u16,
    caret: &mut Caret,
) -> Result<(), Error> {
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

pub fn handle_up(
    view: &mut View,
    _screen_x: u16,
    _screen_y: u16,
    caret: &mut Caret,
) -> Result<(), Error> {
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

pub fn handle_double_click(
    view: &mut View,
    screen_x: u16,
    screen_y: u16,
    caret: &mut Caret,
) -> Result<(), Error> {
    let pos = screen_to_text_pos(view, screen_x, screen_y)?;

    if let Some(line) = view.buffer.lines.get(pos.line) {
        let (start, end) = find_word_boundaries(line, pos.column);

        let start_pos = TextPosition {
            line: pos.line,
            column: start,
        };
        let end_pos = TextPosition {
            line: pos.line,
            column: end,
        };

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

pub fn handle_triple_click(
    view: &mut View,
    screen_x: u16,
    screen_y: u16,
    caret: &mut Caret,
) -> Result<(), Error> {
    let pos = screen_to_text_pos(view, screen_x, screen_y)?;

    if let Some(line) = view.buffer.lines.get(pos.line) {
        let start_pos = TextPosition {
            line: pos.line,
            column: 0,
        };
        let end_pos = TextPosition {
            line: pos.line,
            column: line.len(),
        };

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
    let grapheme_count = grapheme_len(line);

    if col >= grapheme_count || grapheme_count == 0 {
        return (col, col);
    }

    let is_word_char = |g: &str| g.chars().all(|c| c.is_alphanumeric() || c == '_');

    let current = grapheme_at(line, col).unwrap_or("");
    if !is_word_char(current) {
        return (col, col + 1);
    }

    let mut start = col;
    while start > 0 {
        if let Some(g) = grapheme_at(line, start - 1) {
            if is_word_char(g) {
                start -= 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    let mut end = col;
    while end < grapheme_count {
        if let Some(g) = grapheme_at(line, end) {
            if is_word_char(g) {
                end += 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    (start, end)
}

// Scroll the view UP
pub fn scroll_up(view: &mut View, caret: &mut Caret, lines: usize) -> Result<(), Error> {
    if view.scroll_offset == 0 {
        return Ok(());
    }
    view.scroll_offset = view.scroll_offset.saturating_sub(lines);
    view.needs_redraw = true;
    view.render_if_needed(caret, false)?;
    // Keep caret clamped to the visible area
    caret.clamp_to_bounds()?;
    Terminal::execute()?;
    Ok(())
}

// Scroll the view DOWN
pub fn scroll_down(view: &mut View, caret: &mut Caret, lines: usize) -> Result<(), Error> {
    let size = Terminal::get_size()?;
    let visible_rows = size.height.saturating_sub(Position::HEADER + 1) as usize;
    let max_scroll = view.buffer.lines.len().saturating_sub(visible_rows);

    if view.scroll_offset >= max_scroll {
        return Ok(());
    }
    view.scroll_offset = (view.scroll_offset + lines).min(max_scroll);
    view.needs_redraw = true;
    view.render_if_needed(caret, false)?;
    caret.clamp_to_bounds()?;
    Terminal::execute()?;
    Ok(())
}
