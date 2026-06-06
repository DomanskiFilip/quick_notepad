// module handling graphemes
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

// Convert grapheme index to byte offset
pub fn grapheme_to_byte_idx(s: &str, grapheme_idx: usize) -> usize {
    s.grapheme_indices(true)
        .nth(grapheme_idx)
        .map(|(idx, _)| idx)
        .unwrap_or(s.len())
}

// Get grapheme at specific index
pub fn grapheme_at(s: &str, grapheme_idx: usize) -> Option<&str> {
    s.graphemes(true).nth(grapheme_idx)
}

// Count graphemes in a string
pub fn grapheme_len(s: &str) -> usize {
    s.graphemes(true).count()
}

// Get visual width of string (accounts for wide characters like emojis)
pub fn visual_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

// Extract substring by grapheme indices
pub fn grapheme_slice(s: &str, start: usize, end: usize) -> String {
    s.graphemes(true)
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
}

// Insert string at grapheme position
pub fn insert_at_grapheme(s: &mut String, grapheme_idx: usize, text: &str) {
    let byte_idx = grapheme_to_byte_idx(s, grapheme_idx);
    s.insert_str(byte_idx, text);
}

// Remove grapheme at position
pub fn remove_grapheme_at(s: &mut String, grapheme_idx: usize) -> Option<String> {
    let byte_start = grapheme_to_byte_idx(s, grapheme_idx);
    let byte_end = grapheme_to_byte_idx(s, grapheme_idx + 1);

    if byte_start < s.len() {
        let removed: String = s.drain(byte_start..byte_end).collect();
        Some(removed)
    } else {
        None
    }
}

// Split string at grapheme position
pub fn split_at_grapheme(s: &str, grapheme_idx: usize) -> (&str, &str) {
    let byte_idx = grapheme_to_byte_idx(s, grapheme_idx);
    s.split_at(byte_idx)
}

// Wrap a single logical line into visual lines, given a max column width (in terminal columns)
// Uses grapheme boundaries and Unicode visual width to avoid splitting combined characters.
pub fn wrap_line_to_width(line: &str, max_cols: usize) -> Vec<String> {
    if max_cols == 0 {
        return vec![line.to_string()];
    }

    let mut parts: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_width: usize = 0;

    for g in line.graphemes(true) {
        let w = UnicodeWidthStr::width(g);
        if current_width + w > max_cols && !current.is_empty() {
            parts.push(current);
            current = String::new();
            current_width = 0;
        }
        current.push_str(g);
        current_width += w;
    }

    parts.push(current);
    parts
}

// Wrap multi-line text (may contain newlines) into a sequence of lines fit for a given max_cols.
// Preserves explicit newline breaks and blank lines.
pub fn wrap_text_to_width(text: &str, max_cols: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in text.split('\n') {
        if line.is_empty() {
            // preserve blank lines
            out.push(String::new());
            continue;
        }
        let wrapped = wrap_line_to_width(line, max_cols);
        out.extend(wrapped);
    }
    out
}
