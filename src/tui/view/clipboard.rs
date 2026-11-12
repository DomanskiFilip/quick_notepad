// clipboard module with EditOperation tracking
use super::View;
use crate::core::edit_history::{Edit, EditOperation};
use crate::core::graphemes::*;
use crate::core::selection::TextPosition;
use crate::tui::caret::Caret;
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

pub fn paste_from_clipboard(
    view: &mut View,
    caret: &mut Caret,
) -> Result<Option<EditOperation>, Error> {
    // Delete selection first if it exists
    if view.selection.is_some() && view.selection.as_ref().unwrap().is_active() {
        delete_selection(view, caret)?;
    }

    // Get text from clipboard. Long lines are soft wrapped on screen, so paste them as they are.
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        if let Ok(text) = clipboard.get_text() {
            return insert_text_at_cursor(view, caret, &normalize_line_endings(&text));
        }
    }

    Ok(None)
}

pub fn delete_selection(
    view: &mut View,
    caret: &mut Caret,
) -> Result<Option<EditOperation>, Error> {
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
        super::helpers::place_caret(view, caret, start)?;

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
                end.column.min(grapheme_count),
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
                    let text =
                        grapheme_slice(line, start.column.min(grapheme_count), grapheme_count);
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
fn insert_text_at_cursor(
    view: &mut View,
    caret: &mut Caret,
    text: &str,
) -> Result<Option<EditOperation>, Error> {
    if text.is_empty() {
        return Ok(None);
    }

    let cursor_before = caret.get_position();
    let scroll_before = view.scroll_offset;
    let pos = super::helpers::get_current_text_pos(view, caret);

    // Ensure line exists
    while view.buffer.lines.len() <= pos.line {
        view.buffer.lines.push(String::new());
    }

    let current_line = &view.buffer.lines[pos.line];
    let grapheme_pos = pos.column.min(grapheme_len(current_line));
    let (before, after) = split_at_grapheme(current_line, grapheme_pos);
    let (before, after) = (before.to_string(), after.to_string());

    // Split the pasted text into buffer lines, the remainder of the cursor line goes after it
    let pasted: Vec<&str> = text.split('\n').collect();
    let last = pasted.len() - 1;
    let mut new_lines: Vec<String> = pasted.iter().map(|l| l.to_string()).collect();
    new_lines[0].insert_str(0, &before);
    let final_col = grapheme_len(&new_lines[last]);
    new_lines[last].push_str(&after);

    view.buffer
        .lines
        .splice(pos.line..=pos.line, new_lines);

    let final_pos = TextPosition {
        line: pos.line + last,
        column: final_col,
    };
    super::helpers::place_caret(view, caret, final_pos)?;
    view.render(caret)?;

    let edit = if last == 0 {
        Edit::InsertText {
            line: pos.line,
            column: grapheme_pos,
            text: text.to_string(),
        }
    } else {
        Edit::ReplaceRange {
            start_line: pos.line,
            start_column: grapheme_pos,
            end_line: pos.line,
            end_column: grapheme_pos,
            old_text: String::new(),
            new_text: text.to_string(),
        }
    };

    Ok(Some(EditOperation {
        edit,
        cursor_before,
        cursor_after: caret.get_position(),
        scroll_before,
        scroll_after: view.scroll_offset,
    }))
}
