// view mod module with corrected EditOperation returns
mod buffer;
mod clipboard;
mod keyboard;
mod mouse;
mod render;
mod selection;

pub use buffer::Buffer;

use crate::core::edit_history::EditOperation;
use crate::core::selection::{Selection, TextPosition};
use crate::tui::{caret::Caret, terminal::Terminal};
use std::io::Error;

// Prompt kind describes the intent of the footer prompt.
pub(crate) enum PromptKind {
    SaveAs,
    Error,
    // Input,
    // Other,
}

// Prompt state shown in the footer when active.
pub(in crate::tui) struct Prompt {
    pub kind: PromptKind,
    pub message: String,
    pub input: String,
}

pub struct View {
    pub buffer: Buffer,
    pub(in crate::tui::view) selection: Option<Selection>,
    pub(in crate::tui::view) is_dragging: bool,
    pub(in crate::tui) scroll_offset: usize,
    pub(in crate::tui) filename: Option<String>,
    pub(in crate::tui::view) show_shortcuts: bool,
    pub(in crate::tui) prompt: Option<Prompt>,
    pub(in crate::tui) needs_redraw: bool,
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
            prompt: None,
            needs_redraw: true,
        }
    }

    pub fn set_filename(&mut self, filename: String) {
        self.filename = Some(filename);
    }

    // Prompt helpers - used to show a special footer prompt (errors, save-as, etc.)
    pub fn show_prompt(&mut self, kind: PromptKind, message: String) {
        self.prompt = Some(Prompt {
            kind,
            message,
            input: String::new(),
        });
        self.needs_redraw = true;
    }

    // Replace current prompt input (useful when setting initial input programmatically).
    // pub fn update_prompt_input(&mut self, input: String) {
    //     if let Some(p) = &mut self.prompt {
    //         p.input = input;
    //         self.needs_redraw = true;
    //     }
    // }

    // Append a character to the current prompt input (for in-UI typing).
    pub fn append_prompt_char(&mut self, ch: char) {
        if let Some(p) = &mut self.prompt {
            p.input.push(ch);
            self.needs_redraw = true;
        }
    }

    // Backspace in the prompt input.
    pub fn backspace_prompt(&mut self) {
        if let Some(p) = &mut self.prompt {
            p.input.pop();
            self.needs_redraw = true;
        }
    }

    // Clear any active prompt.
    pub fn clear_prompt(&mut self) {
        self.prompt = None;
        self.needs_redraw = true;
    }

    // Return whether a prompt is active.
    // pub fn prompt_active(&self) -> bool {
    //     self.prompt.is_some()
    // }

    //Get a reference to the prompt if active.
    pub fn get_prompt(&self) -> Option<(&PromptKind, &str, &str)> {
        self.prompt
            .as_ref()
            .map(|p| (&p.kind, p.message.as_str(), p.input.as_str()))
    }

    pub fn toggle_ctrl_shortcuts(&mut self) {
        self.show_shortcuts = !self.show_shortcuts;
        self.needs_redraw = true;
    }

    // Rendering
    pub fn render(&self, caret: &Caret) -> Result<(), Error> {
        render::render_view(self, caret, false)
    }

    // render only if needed and clear the flag
    pub fn render_if_needed(&mut self, caret: &Caret, is_dirty: bool) -> Result<(), Error> {
        if self.needs_redraw {
            render::render_view(self, caret, is_dirty)?;
            self.needs_redraw = false;
        }
        Ok(())
    }

    // Clipboard operations - Return Option<EditOperation>
    pub fn copy_selection(&self) -> Result<(), Error> {
        clipboard::copy_selection(self)
    }

    pub fn cut_selection(&mut self, caret: &mut Caret) -> Result<Option<EditOperation>, Error> {
        let result = clipboard::cut_selection(self, caret)?;
        if result.is_some() {
            self.needs_redraw = true;
        }
        Ok(result)
    }

    pub fn paste_from_clipboard(&mut self, caret: &mut Caret,) -> Result<Option<EditOperation>, Error> {
        let result = clipboard::paste_from_clipboard(self, caret)?;
        if result.is_some() {
            self.needs_redraw = true;
        }
        Ok(result)
    }

    // Search in text
    pub fn search(&mut self) -> Result<(), Error> {
        // search::search(self)?;
        self.needs_redraw = true;
        Ok(())
    }

    // Mouse operations
    pub fn handle_mouse_down(&mut self, x: u16, y: u16, caret: &mut Caret) -> Result<(), Error> {
        mouse::handle_down(self, x, y, caret)?;
        self.needs_redraw = true;
        Ok(())
    }

    pub fn handle_mouse_drag(&mut self, x: u16, y: u16, caret: &mut Caret) -> Result<(), Error> {
        mouse::handle_drag(self, x, y, caret)?;
        self.needs_redraw = true;
        Ok(())
    }

    pub fn handle_mouse_up(&mut self, x: u16, y: u16, caret: &mut Caret) -> Result<(), Error> {
        mouse::handle_up(self, x, y, caret)?;
        self.needs_redraw = true;
        Ok(())
    }

    pub fn handle_double_click(&mut self, x: u16, y: u16, caret: &mut Caret) -> Result<(), Error> {
        mouse::handle_double_click(self, x, y, caret)?;
        self.needs_redraw = true;
        Ok(())
    }

    pub fn handle_triple_click(&mut self, x: u16, y: u16, caret: &mut Caret) -> Result<(), Error> {
        mouse::handle_triple_click(self, x, y, caret)?;
        self.needs_redraw = true;
        Ok(())
    }

    // Keyboard operations - Return Option<EditOperation>
    pub fn type_character(
        &mut self,
        character: char,
        caret: &mut Caret,
    ) -> Result<Option<EditOperation>, Error> {
        let result = keyboard::type_character(self, character, caret)?;
        if result.is_some() {
            self.needs_redraw = true;
        }
        Ok(result)
    }

    pub fn insert_newline(&mut self, caret: &mut Caret) -> Result<Option<EditOperation>, Error> {
        let result = keyboard::insert_newline(self, caret)?;
        if result.is_some() {
            self.needs_redraw = true;
        }
        Ok(result)
    }

    pub fn delete_char(&mut self, caret: &mut Caret) -> Result<Option<EditOperation>, Error> {
        let result = keyboard::delete_char(self, caret)?;
        if result.is_some() {
            self.needs_redraw = true;
        }
        Ok(result)
    }

    pub fn backspace(&mut self, caret: &mut Caret) -> Result<Option<EditOperation>, Error> {
        let result = keyboard::backspace(self, caret)?;
        if result.is_some() {
            self.needs_redraw = true;
        }
        Ok(result)
    }

    // Movement operations - only mark dirty if scroll changes or selection changes
    pub fn move_up(&mut self, caret: &mut Caret) -> Result<(), Error> {
        let old_offset = self.scroll_offset;
        let had_selection = self.selection.is_some();
        self.move_without_selection("up", caret)?;
        if self.scroll_offset != old_offset || had_selection {
            self.needs_redraw = true;
        }
        Ok(())
    }

    pub fn move_down(&mut self, caret: &mut Caret) -> Result<(), Error> {
        let old_offset = self.scroll_offset;
        let had_selection = self.selection.is_some();
        self.move_without_selection("down", caret)?;
        if self.scroll_offset != old_offset || had_selection {
            self.needs_redraw = true;
        }
        Ok(())
    }

    pub fn move_left(&mut self, caret: &mut Caret) -> Result<(), Error> {
        let old_offset = self.scroll_offset;
        let had_selection = self.selection.is_some();
        self.move_without_selection("left", caret)?;
        if self.scroll_offset != old_offset || had_selection {
            self.needs_redraw = true;
        }
        Ok(())
    }

    pub fn move_right(&mut self, caret: &mut Caret) -> Result<(), Error> {
        let old_offset = self.scroll_offset;
        let had_selection = self.selection.is_some();
        self.move_without_selection("right", caret)?;
        if self.scroll_offset != old_offset || had_selection {
            self.needs_redraw = true;
        }
        Ok(())
    }

    pub fn move_top(&mut self, caret: &mut Caret) -> Result<(), Error> {
        self.move_without_selection("top", caret)?;
        self.needs_redraw = true;
        Ok(())
    }

    pub fn move_bottom(&mut self, caret: &mut Caret) -> Result<(), Error> {
        self.move_without_selection("bottom", caret)?;
        self.needs_redraw = true;
        Ok(())
    }

    pub fn move_max_left(&mut self, caret: &mut Caret) -> Result<(), Error> {
        let had_selection = self.selection.is_some();
        self.move_without_selection("max_left", caret)?;
        if had_selection {
            self.needs_redraw = true;
        }
        Ok(())
    }

    pub fn move_max_right(&mut self, caret: &mut Caret) -> Result<(), Error> {
        let had_selection = self.selection.is_some();
        self.move_without_selection("max_right", caret)?;
        if had_selection {
            self.needs_redraw = true;
        }
        Ok(())
    }

    // Selection operations - always need redraw
    pub fn move_with_selection(&mut self, direction: &str, caret: &mut Caret) -> Result<(), Error> {
        selection::move_with_selection(self, direction, caret)?;
        self.needs_redraw = true;
        Ok(())
    }

    pub fn move_without_selection(
        &mut self,
        direction: &str,
        caret: &mut Caret,
    ) -> Result<(), Error> {
        selection::move_without_selection(self, direction, caret)
    }

    pub fn select_all(&mut self, caret: &mut Caret) -> Result<(), Error> {
        selection::select_all(self, caret)?;
        self.needs_redraw = true;
        Ok(())
    }

    pub fn handle_resize(&mut self, caret: &mut Caret, is_dirty: bool) -> Result<(), Error> {
        caret.clamp_to_bounds()?;
        self.needs_redraw = true;
        self.render_if_needed(caret, is_dirty)?;
        caret.move_to(caret.get_position())?;
        Ok(())
    }

    // Helper for clamping cursor to line length
    pub(in crate::tui::view) fn clamp_cursor_to_line(
        &self,
        caret: &mut Caret,
    ) -> Result<(), Error> {
        use crate::tui::caret::Position;

        let pos = caret.get_position();
        let buffer_line_idx =
            (pos.y.saturating_sub(Position::HEADER)) as usize + self.scroll_offset;

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
            prompt: None,
            needs_redraw: true,
        }
    }
}

// Helper functions used across modules
pub(in crate::tui::view) mod helpers {
    use super::*;
    use crate::tui::caret::Position;

    pub fn screen_to_text_pos(
        view: &View,
        screen_x: u16,
        screen_y: u16,
    ) -> Result<TextPosition, Error> {
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
        let buffer_line_idx =
            (pos.y.saturating_sub(Position::HEADER)) as usize + view.scroll_offset;
        let char_pos = (pos.x as usize).saturating_sub(Position::MARGIN as usize);

        TextPosition {
            line: buffer_line_idx,
            column: char_pos,
        }
    }
}
