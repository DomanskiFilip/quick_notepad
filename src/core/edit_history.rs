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
            _ => false,
        }
    }
    
    /// Get the next operation to undo
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
    // Apply this edit to the buffer (for redo)
    pub fn apply(&self, buffer: &mut Vec<String>) {
        match self {
            Edit::InsertText { line, column, text } => {
                if let Some(line_content) = buffer.get_mut(*line) {
                    let chars: Vec<char> = line_content.chars().collect();
                    let mut new_chars = chars[..*column].to_vec();
                    new_chars.extend(text.chars());
                    new_chars.extend(&chars[*column..]);
                    *line_content = new_chars.into_iter().collect();
                }
            },
            Edit::DeleteText { line, column, text } => {
                if let Some(line_content) = buffer.get_mut(*line) {
                    let chars: Vec<char> = line_content.chars().collect();
                    let mut new_chars = chars[..*column].to_vec();
                    new_chars.extend(&chars[column + text.len()..]);
                    *line_content = new_chars.into_iter().collect();
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
                // This is complex - handle single vs multi-line replacements
                if start_line == end_line {
                    // Single line replacement
                    if let Some(line_content) = buffer.get_mut(*start_line) {
                        let chars: Vec<char> = line_content.chars().collect();
                        let mut new_chars = chars[..*start_column].to_vec();
                        new_chars.extend(new_text.chars());
                        new_chars.extend(&chars[*end_column..]);
                        *line_content = new_chars.into_iter().collect();
                    }
                } else {
                    // Multi-line replacement
                    for _ in *start_line..=*end_line {
                        if *start_line < buffer.len() {
                            buffer.remove(*start_line);
                        }
                    }
                    // Insert new text (could be multi-line)
                    let new_lines: Vec<&str> = new_text.split('\n').collect();
                    for (i, line) in new_lines.iter().enumerate() {
                        buffer.insert(start_line + i, line.to_string());
                    }
                }
            },
        }
    }
    
    // Reverse this edit (for undo)
    pub fn reverse(&self, buffer: &mut Vec<String>) {
        match self {
            Edit::InsertText { line, column, text } => {
                // Remove the inserted text
                if let Some(line_content) = buffer.get_mut(*line) {
                    let chars: Vec<char> = line_content.chars().collect();
                    let mut new_chars = chars[..*column].to_vec();
                    new_chars.extend(&chars[column + text.len()..]);
                    *line_content = new_chars.into_iter().collect();
                }
            },
            Edit::DeleteText { line, column, text } => {
                // Re-insert the deleted text
                if let Some(line_content) = buffer.get_mut(*line) {
                    let chars: Vec<char> = line_content.chars().collect();
                    let mut new_chars = chars[..*column].to_vec();
                    new_chars.extend(text.chars());
                    new_chars.extend(&chars[*column..]);
                    *line_content = new_chars.into_iter().collect();
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
                    // Split the previous line back
                    if let Some(prev) = buffer.get_mut(line - 1) {
                        let split_content = prev.split_off(*prev_line_end_len);
                        buffer.insert(*line, format!("{}{}", content, split_content));
                    }
                }
            },
            Edit::JoinLines { line, first_line_end } => {
                // Split the line back
                if let Some(current) = buffer.get_mut(*line) {
                    let split_content = current.split_off(*first_line_end);
                    buffer.insert(line + 1, split_content);
                }
            },
            Edit::ReplaceRange { start_line, start_column, end_line, end_column, old_text, .. } => {
                // Replace back with old text (reverse operation)
                if start_line == end_line {
                    if let Some(line_content) = buffer.get_mut(*start_line) {
                        let chars: Vec<char> = line_content.chars().collect();
                        let mut new_chars = chars[..*start_column].to_vec();
                        new_chars.extend(old_text.chars());
                        new_chars.extend(&chars[*end_column..]);
                        *line_content = new_chars.into_iter().collect();
                    }
                }
            },
        }
    }
}