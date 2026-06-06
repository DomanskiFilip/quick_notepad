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