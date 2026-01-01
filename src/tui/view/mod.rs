// view mod module responsible for centralization of view management
mod render;
mod mouse;
mod keyboard;
mod selection;
mod buffer;
mod clipboard;

pub use buffer::Buffer;

use crate::tui::{terminal::Terminal, caret::Caret};
use crate::core::selection::{Selection, TextPosition};
use std::io::Error;

pub struct View {
    pub buffer: Buffer,
    pub(in crate::tui::view) selection: Option<Selection>,
    pub(in crate::tui::view) is_dragging: bool,
    pub(in crate::tui::view) scroll_offset: usize,
    pub(in crate::tui::view) filename: Option<String>,
    pub(in crate::tui::view) show_shortcuts: bool,
}

impl View {
    pub fn new(buffer: Buffer) -> Self {
        Self {
            buffer,
            scroll_offset: 0,
            filename: None,
            show_shortcuts: false,
            selection: None,
            is_dragging: false,
        }
    }
    
    pub fn set_filename(&mut self, filename: String) {
        self.filename = Some(filename);
    }
    
    pub fn toggle_ctrl_shortcuts(&mut self) {
        self.show_shortcuts = !self.show_shortcuts;
    }
    
    // Rendering
    pub fn render(&self, caret: &Caret) -> Result<(), Error> {
        render::render_view(self, caret)
    }
    
    // Clipboard operations - Add these methods
    pub fn copy_selection(&self) -> Result<(), Error> {
        clipboard::copy_selection(self)
    }
    
    pub fn cut_selection(&mut self, caret: &mut Caret) -> Result<(), Error> {
        clipboard::cut_selection(self, caret)
    }
    
    pub fn paste_from_clipboard(&mut self, caret: &mut Caret) -> Result<(), Error> {
        clipboard::paste_from_clipboard(self, caret)
    }
    
    // Mouse operations
    pub fn handle_mouse_down(&mut self, x: u16, y: u16, caret: &mut Caret) -> Result<(), Error> {
        mouse::handle_down(self, x, y, caret)
    }
    
    pub fn handle_mouse_drag(&mut self, x: u16, y: u16, caret: &mut Caret) -> Result<(), Error> {
        mouse::handle_drag(self, x, y, caret)
    }
    
    pub fn handle_mouse_up(&mut self, x: u16, y: u16, caret: &mut Caret) -> Result<(), Error> {
        mouse::handle_up(self, x, y, caret)
    }
    
    pub fn handle_double_click(&mut self, x: u16, y: u16, caret: &mut Caret) -> Result<(), Error> {
        mouse::handle_double_click(self, x, y, caret)
    }
    
    pub fn handle_triple_click(&mut self, x: u16, y: u16, caret: &mut Caret) -> Result<(), Error> {
        mouse::handle_triple_click(self, x, y, caret)
    }
    
    // Keyboard operations
    pub fn type_character(&mut self, character: char, caret: &mut Caret) -> Result<(), Error> {
        keyboard::type_character(self, character, caret)
    }
    
    pub fn insert_newline(&mut self, caret: &mut Caret) -> Result<(), Error> {
        keyboard::insert_newline(self, caret)
    }
    
    pub fn delete_char(&mut self, caret: &mut Caret) -> Result<(), Error> {
        keyboard::delete_char(self, caret)
    }
    
    pub fn backspace(&mut self, caret: &mut Caret) -> Result<(), Error> {
        keyboard::backspace(self, caret)
    }
    
    // Movement operations
    pub fn move_up(&mut self, caret: &mut Caret) -> Result<(), Error> {
        self.move_without_selection("up", caret)
    }
    
    pub fn move_down(&mut self, caret: &mut Caret) -> Result<(), Error> {
        self.move_without_selection("down", caret)
    }
    
    pub fn move_left(&mut self, caret: &mut Caret) -> Result<(), Error> {
        self.move_without_selection("left", caret)
    }
    
    pub fn move_right(&mut self, caret: &mut Caret) -> Result<(), Error> {
        self.move_without_selection("right", caret)
    }
    
    pub fn move_top(&mut self, caret: &mut Caret) -> Result<(), Error> {
        self.move_without_selection("top", caret)
    }
    
    pub fn move_bottom(&mut self, caret: &mut Caret) -> Result<(), Error> {
        self.move_without_selection("bottom", caret)
    }
    
    pub fn move_max_left(&mut self, caret: &mut Caret) -> Result<(), Error> {
        self.move_without_selection("max_left", caret)
    }
    
    pub fn move_max_right(&mut self, caret: &mut Caret) -> Result<(), Error> {
        self.move_without_selection("max_right", caret)
    }
    
    // Selection operations
    pub fn move_with_selection(&mut self, direction: &str, caret: &mut Caret) -> Result<(), Error> {
        selection::move_with_selection(self, direction, caret)
    }
    
    pub fn move_without_selection(&mut self, direction: &str, caret: &mut Caret) -> Result<(), Error> {
        selection::move_without_selection(self, direction, caret)
    }
    
    pub fn select_all(&mut self, caret: &mut Caret) -> Result<(), Error> {
        selection::select_all(self, caret)
    }
    
    pub fn handle_resize(&mut self, caret: &mut Caret) -> Result<(), Error> {
        caret.clamp_to_bounds()?;
        self.render(caret)?;
        caret.move_to(caret.get_position())?;
        Ok(())
    }
    
    // Helper for clamping cursor to line length
    pub(in crate::tui::view) fn clamp_cursor_to_line(&self, caret: &mut Caret) -> Result<(), Error> {
        use crate::tui::caret::Position;
        
        let pos = caret.get_position();
        let buffer_line_idx = (pos.y.saturating_sub(Position::HEADER)) as usize + self.scroll_offset;
        
        if let Some(line) = self.buffer.lines.get(buffer_line_idx) {
            let line_end = Position::MARGIN + line.len() as u16;
            let size = Terminal::get_size()?;
            let max_x = line_end.min(size.width - 1);
            
            if pos.x > max_x {
                caret.move_to(Position { x: max_x, y: pos.y })?;
            } else {
                caret.move_to(pos)?;
            }
        } else {
            caret.move_to(pos)?;
        }
        
        Ok(())
    }
}

impl Default for View {
    fn default() -> Self {
        Self {
            buffer: Buffer::default(),
            scroll_offset: 0,
            filename: None,
            show_shortcuts: false,
            selection: None,
            is_dragging: false,
        }
    }
}

// Helper functions used across modules
pub(in crate::tui::view) mod helpers {
    use super::*;
    use crate::tui::caret::Position;
    
    pub fn screen_to_text_pos(view: &View, screen_x: u16, screen_y: u16) -> Result<TextPosition, Error> {
        let size = Terminal::get_size()?;
        
        // Clamp to valid screen area (don't include footer)
        let y = screen_y.min(size.height.saturating_sub(2));
        
        // Adjust for margin
        let x = if screen_x >= Position::MARGIN { 
            screen_x - Position::MARGIN 
        } else { 
            0 
        };
        
        // Convert screen Y to buffer line index (accounting for header and scroll)
        let line_idx = if y >= Position::HEADER {
            (y - Position::HEADER) as usize + view.scroll_offset
        } else {
            0
        };
        
        // Clamp to actual line length
        let max_col = if let Some(line) = view.buffer.lines.get(line_idx) {
            line.len()
        } else {
            0
        };
        
        Ok(TextPosition {
            line: line_idx,
            column: (x as usize).min(max_col),
        })
    }
    
    pub fn text_to_screen_pos(view: &View, pos: TextPosition) -> (u16, u16) {
        // Convert buffer line index to screen Y (accounting for header and scroll)
        let screen_y = if pos.line >= view.scroll_offset {
            Position::HEADER + (pos.line - view.scroll_offset) as u16
        } else {
            Position::HEADER
        };
        
        let screen_x = (pos.column as u16) + Position::MARGIN;
        (screen_x, screen_y)
    }
    
    pub fn get_current_text_pos(view: &View, caret: &Caret) -> TextPosition {
        let pos = caret.get_position();
        let buffer_line_idx = (pos.y.saturating_sub(Position::HEADER)) as usize + view.scroll_offset;
        let char_pos = (pos.x as usize).saturating_sub(Position::MARGIN as usize);
        
        TextPosition {
            line: buffer_line_idx,
            column: char_pos,
        }
    }
}