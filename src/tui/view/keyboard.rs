// keyboard logic with edit tracking for undo/redo
use super::{helpers, View};
use crate::core::edit_history::{Edit, EditOperation};
use crate::core::graphemes::*;
use crate::core::selection::TextPosition;
use crate::tui::caret::Caret;
use std::io::Error;

fn has_active_selection(view: &View) -> bool {
    view.selection.as_ref().is_some_and(|s| s.is_active())
}

// Buffer position under the caret, with the line guaranteed to exist
fn cursor_pos(view: &mut View, caret: &Caret) -> TextPosition {
    let pos = helpers::get_current_text_pos(view, caret);
    while view.buffer.lines.len() <= pos.line {
        view.buffer.lines.push(String::new());
    }
    let column = pos.column.min(grapheme_len(&view.buffer.lines[pos.line]));
    TextPosition {
        line: pos.line,
        column,
    }
}

pub fn type_character(
    view: &mut View,
    character: char,
    caret: &mut Caret,
) -> Result<Option<EditOperation>, Error> {
    // Delete selection first if it exists
    if has_active_selection(view) {
        super::clipboard::delete_selection(view, caret)?;
    }

    let cursor_before = caret.get_position();
    let scroll_before = view.scroll_offset;
    let pos = cursor_pos(view, caret);

    // Insert the character; long lines are soft wrapped on screen, never split in the buffer
    let text = character.to_string();
    let line = &mut view.buffer.lines[pos.line];
    let byte_idx = grapheme_to_byte_idx(line, pos.column);
    line.insert_str(byte_idx, &text);
    // A combining mark joins the previous grapheme, so count instead of adding one
    let new_column = grapheme_len(&line[..byte_idx + text.len()]);

    helpers::place_caret(
        view,
        caret,
        TextPosition {
            line: pos.line,
            column: new_column,
        },
    )?;
    view.render(caret)?;

    Ok(Some(EditOperation {
        edit: Edit::InsertText {
            line: pos.line,
            column: pos.column,
            text,
        },
        cursor_before,
        cursor_after: caret.get_position(),
        scroll_before,
        scroll_after: view.scroll_offset,
    }))
}

pub fn insert_newline(view: &mut View, caret: &mut Caret) -> Result<Option<EditOperation>, Error> {
    if has_active_selection(view) {
        super::clipboard::delete_selection(view, caret)?;
    }

    let cursor_before = caret.get_position();
    let scroll_before = view.scroll_offset;
    let pos = cursor_pos(view, caret);

    // Split at grapheme position
    let current_line = &view.buffer.lines[pos.line];
    let (before, remaining) = split_at_grapheme(current_line, pos.column);
    let (before, remaining_text) = (before.to_string(), remaining.to_string());

    view.buffer.lines[pos.line] = before;
    view.buffer.lines.insert(pos.line + 1, remaining_text.clone());

    helpers::place_caret(
        view,
        caret,
        TextPosition {
            line: pos.line + 1,
            column: 0,
        },
    )?;
    view.render(caret)?;

    Ok(Some(EditOperation {
        edit: Edit::InsertLine {
            line: pos.line,
            remaining_text,
        },
        cursor_before,
        cursor_after: caret.get_position(),
        scroll_before,
        scroll_after: view.scroll_offset,
    }))
}

pub fn delete_char(view: &mut View, caret: &mut Caret) -> Result<Option<EditOperation>, Error> {
    if has_active_selection(view) {
        return super::clipboard::delete_selection(view, caret);
    }

    let cursor_before = caret.get_position();
    let scroll_before = view.scroll_offset;
    let pos = cursor_pos(view, caret);

    let grapheme_count = grapheme_len(&view.buffer.lines[pos.line]);

    let edit = if pos.column < grapheme_count {
        // Delete grapheme at cursor
        let deleted =
            remove_grapheme_at(&mut view.buffer.lines[pos.line], pos.column).unwrap_or_default();
        Edit::DeleteText {
            line: pos.line,
            column: pos.column,
            text: deleted,
        }
    } else if pos.line + 1 < view.buffer.lines.len() {
        // At end of line, merge with next line
        let next_line = view.buffer.lines.remove(pos.line + 1);
        view.buffer.lines[pos.line].push_str(&next_line);
        Edit::JoinLines {
            line: pos.line,
            first_line_end: grapheme_count,
        }
    } else {
        return Ok(None);
    };

    helpers::place_caret(view, caret, pos)?;
    view.render(caret)?;

    Ok(Some(EditOperation {
        edit,
        cursor_before,
        cursor_after: caret.get_position(),
        scroll_before,
        scroll_after: view.scroll_offset,
    }))
}

pub fn backspace(view: &mut View, caret: &mut Caret) -> Result<Option<EditOperation>, Error> {
    if has_active_selection(view) {
        return super::clipboard::delete_selection(view, caret);
    }

    let cursor_before = caret.get_position();
    let scroll_before = view.scroll_offset;
    let pos = cursor_pos(view, caret);

    let (edit, new_pos) = if pos.column > 0 {
        // Delete grapheme before cursor
        let deleted = remove_grapheme_at(&mut view.buffer.lines[pos.line], pos.column - 1)
            .unwrap_or_default();
        (
            Edit::DeleteText {
                line: pos.line,
                column: pos.column - 1,
                text: deleted,
            },
            TextPosition {
                line: pos.line,
                column: pos.column - 1,
            },
        )
    } else if pos.line > 0 {
        // At beginning of line, merge with previous line
        let prev_line_len = grapheme_len(&view.buffer.lines[pos.line - 1]);
        let current_line_content = view.buffer.lines.remove(pos.line);
        view.buffer.lines[pos.line - 1].push_str(&current_line_content);
        (
            Edit::DeleteLine {
                line: pos.line,
                content: current_line_content,
                prev_line_end_len: prev_line_len,
            },
            TextPosition {
                line: pos.line - 1,
                column: prev_line_len,
            },
        )
    } else {
        return Ok(None);
    };

    helpers::place_caret(view, caret, new_pos)?;
    view.render(caret)?;

    Ok(Some(EditOperation {
        edit,
        cursor_before,
        cursor_after: caret.get_position(),
        scroll_before,
        scroll_after: view.scroll_offset,
    }))
}
