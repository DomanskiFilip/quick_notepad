// edit_history module - delta-based undo/redo system
use crate::tui::caret::Position;

// Represents a single atomic edit operation that can be undone/redone
#[derive(Clone, Debug)]
pub enum Edit {
    // Insert text at a position (line, column, text)
    InsertText {
        line: usize,
        column: usize,
        text: String,
    },
    // Delete text at a position (line, column, text, original_text)
    DeleteText {
        line: usize,
        column: usize,
        text: String, // The deleted text (needed for redo)
    },
    // Insert a new line at position (line, remaining_text)
    InsertLine {
        line: usize,
        remaining_text: String, // Text that moved to new line
    },
    // Delete a line and merge with previous (line, deleted_line_content)
    DeleteLine {
        line: usize,
        content: String,
        prev_line_end_len: usize, // Where cursor should go on undo
    },
    // Join lines (line_idx, second_line_content)
    JoinLines {
        line: usize,
        first_line_end: usize, // Position where join occurred
    },
    // Replace text in a range (for paste over selection)
    ReplaceRange {
        start_line: usize,
        start_column: usize,
        end_line: usize,
        end_column: usize,
        old_text: String,
        new_text: String,
    },
    // Replace a block of whole lines (used by the GUI, which diffs buffer snapshots)
    ReplaceLines {
        start: usize,
        old_lines: Vec<String>,
        new_lines: Vec<String>,
    },
}

// A complete edit operation with before/after cursor state
#[derive(Clone, Debug)]
pub struct EditOperation {
    pub edit: Edit,
    pub cursor_before: Position,
    pub cursor_after: Position,
    pub scroll_before: usize,
    pub scroll_after: usize,
}

#[derive(Clone)]
pub struct EditHistory {
    undo_stack: Vec<EditOperation>,
    redo_stack: Vec<EditOperation>,
    max_history: usize,
    
    // For grouping rapid edits (like continuous typing)
    last_edit_time: std::time::Instant,
    grouping_threshold_ms: u128,
}

impl EditHistory {
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history,
            last_edit_time: std::time::Instant::now(),
            grouping_threshold_ms: 500, // Group edits within 500ms
        }
    }
    
    // Push a new edit operation
    pub fn push(&mut self, operation: EditOperation) {
        // Clear redo stack when new edit is made
        self.redo_stack.clear();
        
        // Check if we should group this with the last edit
        let now = std::time::Instant::now();
        let should_group = self.can_group_with_last(&operation, now);
        
        if should_group {
            // Try to merge with last operation
            if let Some(mut last_op) = self.undo_stack.pop() {
                if Self::try_merge_operations(&mut last_op, &operation) {
                    // Merge successful, push back the merged operation
                    self.undo_stack.push(last_op);
                    self.last_edit_time = now;
                    return;
                } else {
                    // Merge failed, push back the old operation
                    self.undo_stack.push(last_op);
                }
            }
        }
        
        // Add as new operation
        self.undo_stack.push(operation);
        self.last_edit_time = now;
        
        // Limit stack size
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
    }
    
    // Check if we can group this edit with the previous one
    fn can_group_with_last(&self, operation: &EditOperation, now: std::time::Instant) -> bool {
        if self.undo_stack.is_empty() {
            return false;
        }
        
        let elapsed = now.duration_since(self.last_edit_time).as_millis();
        if elapsed > self.grouping_threshold_ms {
            return false;
        }
        
        // Only group similar operations on the same line
        if let Some(last_op) = self.undo_stack.last() {
            match (&last_op.edit, &operation.edit) {
                (Edit::InsertText { line: l1, .. }, Edit::InsertText { line: l2, .. }) => l1 == l2,
                (Edit::DeleteText { line: l1, .. }, Edit::DeleteText { line: l2, .. }) => l1 == l2,
                (Edit::ReplaceLines { start: s1, new_lines: n1, .. }, Edit::ReplaceLines { start: s2, old_lines: o2, .. }) => s1 == s2 && n1.len() == 1 && o2.len() == 1,
                _ => false,
            }
        } else {
            false
        }
    }
    
    // Try to merge two operations (for continuous typing/deleting)
    fn try_merge_operations(last: &mut EditOperation, new: &EditOperation) -> bool {
        match (&mut last.edit, &new.edit) {
            // Merge continuous character insertions
            (
                Edit::InsertText { line: l1, column: c1, text: t1 },
                Edit::InsertText { line: l2, column: c2, text: t2 }
            ) if l1 == l2 && *c1 + t1.len() == *c2 => {
                t1.push_str(t2);
                last.cursor_after = new.cursor_after;
                last.scroll_after = new.scroll_after;
                true
            },
            // Merge continuous backspace deletions
            (
                Edit::DeleteText { line: l1, column: c1, text: t1 },
                Edit::DeleteText { line: l2, column: c2, text: t2 }
            ) if l1 == l2 && *c2 == c1.saturating_sub(t2.len()) => {
                // Prepend the newly deleted text
                *t1 = format!("{}{}", t2, t1);
                *c1 = *c2;
                last.cursor_after = new.cursor_after;
                last.scroll_after = new.scroll_after;
                true
            },
            // Merge consecutive single-line changes (continuous typing in the GUI)
            (
                Edit::ReplaceLines { start: s1, new_lines: n1, .. },
                Edit::ReplaceLines { start: s2, old_lines: o2, new_lines: n2 }
            ) if s1 == s2 && n1 == o2 && n2.len() == 1 => {
                *n1 = n2.clone();
                last.cursor_after = new.cursor_after;
                last.scroll_after = new.scroll_after;
                true
            },
            _ => false,
        }
    }
    
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    // Get the next operation to undo
    pub fn undo(&mut self) -> Option<EditOperation> {
        if let Some(operation) = self.undo_stack.pop() {
            self.redo_stack.push(operation.clone());
            Some(operation)
        } else {
            None
        }
    }
    
    // Get the next operation to redo
    pub fn redo(&mut self) -> Option<EditOperation> {
        if let Some(operation) = self.redo_stack.pop() {
            self.undo_stack.push(operation.clone());
            Some(operation)
        } else {
            None
        }
    }
}

impl Edit {
    // Build a ReplaceLines edit from two buffer snapshots (None if nothing changed)
    pub fn diff_lines(before: &[String], after: &[String]) -> Option<Edit> {
        if before == after {
            return None;
        }

        // Skip the lines both snapshots share at the start and at the end
        let prefix = before
            .iter()
            .zip(after.iter())
            .take_while(|(a, b)| a == b)
            .count();
        let max_suffix = before.len().min(after.len()) - prefix;
        let suffix = before
            .iter()
            .rev()
            .zip(after.iter().rev())
            .take(max_suffix)
            .take_while(|(a, b)| a == b)
            .count();

        Some(Edit::ReplaceLines {
            start: prefix,
            old_lines: before[prefix..before.len() - suffix].to_vec(),
            new_lines: after[prefix..after.len() - suffix].to_vec(),
        })
    }

    pub fn apply(&self, buffer: &mut Vec<String>) {
        use unicode_segmentation::UnicodeSegmentation;
        
        match self {
            Edit::InsertText { line, column, text } => {
                if let Some(line_content) = buffer.get_mut(*line) {
                    // Convert grapheme index to byte index
                    let byte_idx = line_content
                        .grapheme_indices(true)
                        .nth(*column)
                        .map(|(idx, _)| idx)
                        .unwrap_or(line_content.len());
                    
                    line_content.insert_str(byte_idx, text);
                }
            },
            Edit::DeleteText { line, column, text } => {
                if let Some(line_content) = buffer.get_mut(*line) {
                    let grapheme_count: usize = line_content.graphemes(true).count();
                    
                    if *column < grapheme_count {
                        // Find byte range for this grapheme index
                        let byte_start = line_content
                            .grapheme_indices(true)
                            .nth(*column)
                            .map(|(idx, _)| idx)
                            .unwrap_or(line_content.len());
                        
                        let byte_end = byte_start + text.len();
                        
                        if byte_end <= line_content.len() {
                            line_content.drain(byte_start..byte_end);
                        }
                    }
                }
            },
            Edit::InsertLine { line, remaining_text } => {
                if *line < buffer.len() {
                    buffer.insert(line + 1, remaining_text.clone());
                } else {
                    buffer.push(remaining_text.clone());
                }
            },
            Edit::DeleteLine { line, .. } => {
                if *line < buffer.len() {
                    buffer.remove(*line);
                }
            },
            Edit::JoinLines { line, .. } => {
                if line + 1 < buffer.len() {
                    let next_line = buffer.remove(line + 1);
                    if let Some(current) = buffer.get_mut(*line) {
                        current.push_str(&next_line);
                    }
                }
            },
            Edit::ReplaceRange { start_line, start_column, end_line, end_column, new_text, .. } => {
                if start_line == end_line {
                    // Single line replacement
                    if let Some(line_content) = buffer.get_mut(*start_line) {
                        let byte_start = line_content
                            .grapheme_indices(true)
                            .nth(*start_column)
                            .map(|(idx, _)| idx)
                            .unwrap_or(line_content.len());
                        
                        let byte_end = line_content
                            .grapheme_indices(true)
                            .nth(*end_column)
                            .map(|(idx, _)| idx)
                            .unwrap_or(line_content.len());
                        
                        line_content.replace_range(byte_start..byte_end, new_text);
                    }
                } else {
                    // Multi-line - remove and insert
                    for _ in *start_line..=*end_line {
                        if *start_line < buffer.len() {
                            buffer.remove(*start_line);
                        }
                    }
                    let new_lines: Vec<&str> = new_text.split('\n').collect();
                    for (i, line) in new_lines.iter().enumerate() {
                        buffer.insert(start_line + i, line.to_string());
                    }
                }
            },
            Edit::ReplaceLines { start, old_lines, new_lines } => {
                let end = (start + old_lines.len()).min(buffer.len());
                let start = (*start).min(end);
                buffer.splice(start..end, new_lines.iter().cloned());
            },
        }
    }
    
    pub fn reverse(&self, buffer: &mut Vec<String>) {
        use unicode_segmentation::UnicodeSegmentation;
        
        match self {
            Edit::InsertText { line, column, text } => {
                // Remove the inserted text
                if let Some(line_content) = buffer.get_mut(*line) {
                    let byte_start = line_content
                        .grapheme_indices(true)
                        .nth(*column)
                        .map(|(idx, _)| idx)
                        .unwrap_or(line_content.len());
                    
                    let byte_end = byte_start + text.len();
                    
                    if byte_end <= line_content.len() {
                        line_content.drain(byte_start..byte_end);
                    }
                }
            },
            Edit::DeleteText { line, column, text } => {
                // Re-insert the deleted text
                if let Some(line_content) = buffer.get_mut(*line) {
                    let byte_idx = line_content
                        .grapheme_indices(true)
                        .nth(*column)
                        .map(|(idx, _)| idx)
                        .unwrap_or(line_content.len());
                    
                    line_content.insert_str(byte_idx, text);
                }
            },
            Edit::InsertLine { line, remaining_text } => {
                // Remove the inserted line and merge back
                if line + 1 < buffer.len() {
                    buffer.remove(line + 1);
                    if let Some(current) = buffer.get_mut(*line) {
                        current.push_str(remaining_text);
                    }
                }
            },
            Edit::DeleteLine { line, content, prev_line_end_len } => {
                // Re-insert the deleted line
                if *line > 0 && *line <= buffer.len() {
                    if let Some(prev) = buffer.get_mut(line - 1) {
                        // Convert grapheme index to byte index for split
                        let byte_idx = prev
                            .grapheme_indices(true)
                            .nth(*prev_line_end_len)
                            .map(|(idx, _)| idx)
                            .unwrap_or(prev.len());
                        
                        let split_content = prev.split_off(byte_idx);
                        buffer.insert(*line, format!("{}{}", content, split_content));
                    }
                }
            },
            Edit::JoinLines { line, first_line_end } => {
                // Split the line back
                if let Some(current) = buffer.get_mut(*line) {
                    let byte_idx = current
                        .grapheme_indices(true)
                        .nth(*first_line_end)
                        .map(|(idx, _)| idx)
                        .unwrap_or(current.len());
                    
                    let split_content = current.split_off(byte_idx);
                    buffer.insert(line + 1, split_content);
                }
            },
            Edit::ReplaceRange { start_line, start_column, end_line, end_column, old_text, .. } => {
                // Replace back with old text (reverse operation)
                if start_line == end_line {
                    if let Some(line_content) = buffer.get_mut(*start_line) {
                        let byte_start = line_content
                            .grapheme_indices(true)
                            .nth(*start_column)
                            .map(|(idx, _)| idx)
                            .unwrap_or(line_content.len());
                        
                        let byte_end = line_content
                            .grapheme_indices(true)
                            .nth(*end_column)
                            .map(|(idx, _)| idx)
                            .unwrap_or(line_content.len());
                        
                        line_content.replace_range(byte_start..byte_end, old_text);
                    }
                }
            },
            Edit::ReplaceLines { start, old_lines, new_lines } => {
                let end = (start + new_lines.len()).min(buffer.len());
                let start = (*start).min(end);
                buffer.splice(start..end, old_lines.iter().cloned());
            },
        }
    }
}