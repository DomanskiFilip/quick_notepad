// selection module for text selection data structures
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextPosition {
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug)]
pub struct Selection {
    pub anchor: TextPosition,  // Where selection started
    pub cursor: TextPosition,  // Current cursor position
}

impl Selection {
    pub fn new(pos: TextPosition) -> Self {
        Self {
            anchor: pos,
            cursor: pos,
        }
    }
    
    pub fn is_active(&self) -> bool {
        self.anchor != self.cursor
    }
    
    // Get ordered start and end (anchor might be after cursor)
    pub fn get_range(&self) -> (TextPosition, TextPosition) {
        if self.anchor.line < self.cursor.line 
            || (self.anchor.line == self.cursor.line && self.anchor.column < self.cursor.column) {
            (self.anchor, self.cursor)
        } else {
            (self.cursor, self.anchor)
        }
    }
    
    pub fn clear(&mut self) {
        self.cursor = self.anchor;
    }
    
    pub fn update_cursor(&mut self, new_pos: TextPosition) {
        self.cursor = new_pos;
    }
    
    pub fn reset_anchor(&mut self, pos: TextPosition) {
        self.anchor = pos;
        self.cursor = pos;
    }
}