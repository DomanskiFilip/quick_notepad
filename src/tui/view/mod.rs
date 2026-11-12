// view mod module with corrected EditOperation returns
mod clipboard;
mod keyboard;
mod mouse;
mod render;
mod search;
mod selection;

pub use crate::core::buffer::Buffer;
use crate::core::{
    edit_history::EditOperation,
    selection::{Selection, TextPosition},
    wrap::{self, RowPos},
};
use crate::tui::{caret::Caret, terminal::Terminal};
pub use crate::core::search::SearchState;
use std::io::Error;

// Prompt kind describes the intent of the footer prompt.
pub(crate) enum PromptKind {
    SaveAs,
    Error,
    Search,
    SearchInfo,
}

// Prompt state shown in the footer when active.
pub(in crate::tui) struct Prompt {
    pub kind: PromptKind,
    pub message: String,
    pub input: String,
}

pub struct View {
    pub buffer: Buffer,
    pub selection: Option<Selection>,
    pub is_dragging: bool,
    // first visible buffer line, and how many of its wrapped rows are scrolled past
    pub scroll_offset: usize,
    pub scroll_row: usize,
    // text area size the lines are wrapped to (updated on resize)
    pub wrap_cols: usize,
    pub wrap_rows: usize,
    pub filename: Option<String>,
    pub filetype: Option<String>,
    pub prompt_since: Option<std::time::Instant>,
    pub show_shortcuts: bool,
    pub needs_redraw: bool,
    pub search_state: Option<SearchState>,
    pub(in crate::tui) prompt: Option<Prompt>,
    #[allow(dead_code)]
    // clipboard for wayland must be here even tho rust warns its unused - its not!
    clipboard: Option<arboard::Clipboard>,
}

impl View {
    pub fn new(buffer: Buffer) -> Self {
        Self {
            buffer,
            scroll_offset: 0,
            scroll_row: 0,
            wrap_cols: text_area_size().0,
            wrap_rows: text_area_size().1,
            filename: None,
            filetype: None,
            prompt_since: None,
            show_shortcuts: false,
            selection: None,
            is_dragging: false,
            prompt: None,
            needs_redraw: true,
            search_state: None,
            clipboard: try_clipboard(),
        }
    }

    pub fn set_filename_and_filetype(
        &mut self,
        filename: Option<String>,
        filetype: Option<String>,
    ) {
        self.filename = filename;
        self.filetype = filetype;
    }

    // Prompt helpers - used to show a special footer prompt (errors, save-as, etc.)
    pub fn show_prompt(&mut self, kind: PromptKind, message: String) {
        self.prompt = Some(Prompt {
            kind,
            message,
            input: String::new(),
        });
        // Record when the prompt was shown so UI can auto-clear it after a timeout.
        self.prompt_since = Some(std::time::Instant::now());
        self.needs_redraw = true;
    }

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
        self.prompt_since = None;
        self.needs_redraw = true;
    }

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

    pub fn paste_from_clipboard(
        &mut self,
        caret: &mut Caret,
    ) -> Result<Option<EditOperation>, Error> {
        let result = clipboard::paste_from_clipboard(self, caret)?;
        if result.is_some() {
            self.needs_redraw = true;
        }
        Ok(result)
    }

    // Search in text
    pub fn search(&mut self, caret: &mut Caret) -> Result<(), Error> {
        search::search(self, caret)?;
        self.needs_redraw = true;
        Ok(())
    }

    pub fn set_search_state(&mut self, state: Option<SearchState>) {
        self.search_state = state;
    }

    pub fn set_current_match(&mut self, idx: usize) {
        if let Some(ref mut state) = self.search_state {
            state.current_match_idx = idx;
        }
    }

    pub fn next_search_match(&mut self, caret: &mut Caret) -> Result<(), Error> {
        search::next_search_match(self, caret)?;
        Ok(())
    }

    pub fn prev_search_match(&mut self, caret: &mut Caret) -> Result<(), Error> {
        search::prev_search_match(self, caret)?;
        Ok(())
    }

    pub fn clear_search(&mut self) {
        search::clear_search(self);
    }

    pub fn is_search_active(&self) -> bool {
        self.search_state.is_some()
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

    // Handle mouse wheel scroll
    pub fn handle_mouse_wheel(&mut self, delta: f32, caret: &mut Caret) -> Result<(), Error> {
        let lines_to_scroll = (delta.abs() * 3.0) as usize; // Adjust scroll sensitivity as needed
                                                            // Positive delta -> scroll UP (same semantic as crossterm MouseEventKind::ScrollUp)
        if delta > 0.0 {
            mouse::scroll_up(self, caret, lines_to_scroll)?;
        } else {
            mouse::scroll_down(self, caret, lines_to_scroll)?;
        }
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
        // Work out where the caret is with the old layout, then re-wrap and put it back
        let pos = helpers::get_current_text_pos(self, caret);
        // Keep the same text at the top of the screen
        let top_col = wrap::row_starts(self.line(self.scroll_offset), self.wrap_cols)
            .get(self.scroll_row)
            .copied()
            .unwrap_or(0);
        let (cols, rows) = text_area_size();
        self.wrap_cols = cols;
        self.wrap_rows = rows;
        let top_row = wrap::row_of(&wrap::row_starts(self.line(self.scroll_offset), cols), top_col);
        self.set_top(RowPos {
            line: self.scroll_offset,
            row: top_row,
        });
        helpers::place_caret(self, caret, pos)?;
        Terminal::clear_screen()?;
        self.needs_redraw = true;
        self.render_if_needed(caret, is_dirty)?;
        caret.move_to(caret.get_position())?;
        Ok(())
    }

    // Viewport top as a visual row
    pub(in crate::tui) fn top(&self) -> RowPos {
        RowPos {
            line: self.scroll_offset,
            row: self.scroll_row,
        }
    }

    pub(in crate::tui) fn set_top(&mut self, top: RowPos) {
        let top = wrap::clamp_row(&self.buffer.lines, self.wrap_cols, top);
        self.scroll_offset = top.line;
        self.scroll_row = top.row;
    }

    // Scroll so a buffer line is the first one on screen (used by undo/redo and tabs)
    pub fn set_scroll_line(&mut self, line: usize) {
        self.scroll_offset = line;
        self.scroll_row = 0;
    }

    pub(in crate::tui) fn line(&self, idx: usize) -> &str {
        self.buffer.lines.get(idx).map(String::as_str).unwrap_or("")
    }
}

// Columns and rows available for text. One column is kept free on the right
// so the caret has room after the last character of a full row.
fn text_area_size() -> (usize, usize) {
    use crate::tui::caret::Position;
    let size = Terminal::get_size().unwrap_or(crate::tui::terminal::Size {
        width: 80,
        height: 24,
    });
    let cols = size.width.saturating_sub(Position::MARGIN + 1).max(1) as usize;
    let rows = size.height.saturating_sub(Position::HEADER + 1).max(1) as usize;
    (cols, rows)
}

impl Default for View {
    fn default() -> Self {
        Self {
            buffer: Buffer::default(),
            scroll_offset: 0,
            scroll_row: 0,
            wrap_cols: text_area_size().0,
            wrap_rows: text_area_size().1,
            filename: None,
            filetype: None,
            prompt_since: None,
            show_shortcuts: false,
            selection: None,
            is_dragging: false,
            prompt: None,
            needs_redraw: true,
            search_state: None,
            clipboard: try_clipboard(),
        }
    }
}

// Helper functions used across modules.
// The caret lives in screen coordinates; these map it to buffer positions through the wrapped layout.
pub mod helpers {
    use super::*;
    use crate::tui::caret::Position;
    use unicode_segmentation::UnicodeSegmentation;

    // Column on visual row `rp` under screen column `x` (relative to the text area).
    pub fn col_at_x(view: &View, rp: RowPos, x: usize) -> usize {
        wrap::col_at_x(view.line(rp.line), view.wrap_cols, rp.row, x)
    }

    pub fn text_pos_at(view: &View, screen_x: u16, screen_y: u16) -> TextPosition {
        let y = (screen_y.saturating_sub(Position::HEADER) as usize).min(view.wrap_rows - 1);
        let rp = wrap::step_forward(&view.buffer.lines, view.wrap_cols, view.top(), y);
        let x = screen_x.saturating_sub(Position::MARGIN) as usize;
        TextPosition {
            line: rp.line,
            column: col_at_x(view, rp, x),
        }
    }

    pub fn screen_to_text_pos(
        view: &View,
        screen_x: u16,
        screen_y: u16,
    ) -> Result<TextPosition, Error> {
        Ok(text_pos_at(view, screen_x, screen_y))
    }

    pub fn text_to_screen_pos(view: &View, pos: TextPosition) -> (u16, u16) {
        let (row, x) = wrap::locate(view.line(pos.line), view.wrap_cols, pos.column);
        let target = RowPos {
            line: pos.line,
            row,
        };

        let rows = view.wrap_rows;
        let y = match wrap::rows_between(&view.buffer.lines, view.wrap_cols, view.top(), target, rows) {
            Some(n) if n < rows => n,
            Some(_) => rows - 1,
            None => 0,
        };

        (
            Position::MARGIN + x.min(view.wrap_cols) as u16,
            Position::HEADER + y as u16,
        )
    }

    pub fn get_current_text_pos(view: &View, caret: &Caret) -> TextPosition {
        let pos = caret.get_position();
        text_pos_at(view, pos.x, pos.y)
    }

    // Put the caret on a buffer position, scrolling just enough to keep it on screen.
    pub fn place_caret(view: &mut View, caret: &mut Caret, pos: TextPosition) -> Result<(), Error> {
        let line = pos.line.min(view.buffer.lines.len().saturating_sub(1));
        let pos = TextPosition {
            line,
            column: pos.column.min(view.line(line).graphemes(true).count()),
        };
        let target = wrap::row_pos_of(&view.buffer.lines, view.wrap_cols, pos);
        let top = wrap::ensure_visible(
            &view.buffer.lines,
            view.wrap_cols,
            view.top(),
            view.wrap_rows,
            target,
        );
        if top != view.top() {
            view.set_top(top);
            view.needs_redraw = true;
        }
        let (x, y) = text_to_screen_pos(view, pos);
        caret.move_to(Position { x, y })
    }
}

fn try_clipboard() -> Option<arboard::Clipboard> {
    // catch_unwind requires the closure to be UnwindSafe.
    // arboard::Clipboard is not marked UnwindSafe, so we use AssertUnwindSafe.
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        arboard::Clipboard::new().ok()
    }))
    .unwrap_or(None)
}
