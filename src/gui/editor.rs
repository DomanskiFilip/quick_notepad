// src/gui/editor.rs - Editor with syntax highlighting
use super::state::EditorState;
use crate::core::graphemes::*;
use crate::core::selection::{Selection, TextPosition};
use crate::core::syntax::SyntaxHighlighter;
use egui::{Color32, FontId, Pos2, Rect, Response, Sense, Stroke, Ui};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

// Map a relative pixel X within the editor's available width to a grapheme column index for a line.
// Uses visual (column) widths of graphemes (unicode-width), and maps the pixel fraction onto the
// total visual width of the line. This avoids depending on a hard-coded per-column pixel width.
fn x_to_column_for_line(rel_x: f32, avail_px: f32, line: &str) -> usize {
    if avail_px <= 0.0 {
        return 0;
    }

    let total_cols = UnicodeWidthStr::width(line) as f32;
    if total_cols <= 0.0 {
        return 0;
    }

    let fraction = (rel_x / avail_px).clamp(0.0, 1.0);
    let target_cols = fraction * total_cols;

    let mut acc = 0.0f32;
    for (i, g) in line.graphemes(true).enumerate() {
        let w = UnicodeWidthStr::width(g) as f32;
        if acc + w > target_cols {
            return i;
        }
        acc += w;
    }

    // If we didn't hit target, place at end
    line.graphemes(true).count()
}

// Map a grapheme column to a pixel X offset within the available width for the line. Returns pixel offset from start (not including margin).
fn column_to_x_for_line(col: usize, avail_px: f32, line: &str) -> f32 {
    if avail_px <= 0.0 {
        return 0.0;
    }
    let total_cols = UnicodeWidthStr::width(line) as f32;
    if total_cols <= 0.0 {
        return 0.0;
    }

    let mut acc_cols = 0usize;
    for (i, g) in line.graphemes(true).enumerate() {
        if i >= col {
            break;
        }
        acc_cols += UnicodeWidthStr::width(g);
    }

    (acc_cols as f32 / total_cols) * avail_px
}

pub struct EditorPanel<'a> {
    state: &'a mut EditorState,
    accepts_input: bool,
}

impl<'a> EditorPanel<'a> {
    pub fn new(state: &'a mut EditorState, accepts_input: bool) -> Self {
        Self {
            state,
            accepts_input,
        }
    }

    pub fn show(&mut self, ui: &mut Ui) -> Response {
        let was_search_active = self.state.search_active;

        if self.state.search_active {
            self.show_search_bar(ui);
        }

        let available_rect = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(available_rect, Sense::click_and_drag());

        if was_search_active && !self.state.search_active {
            let search_id = egui::Id::new("search_bar_input");
            ui.ctx().memory_mut(|m| m.surrender_focus(search_id));
        }

        if self.accepts_input {
            self.handle_input(ui, &response);
        }

        self.render_content(ui, &response, available_rect);

        response
    }

    fn show_search_bar(&mut self, ui: &mut Ui) {
        let mut close_search = false;
        let mut do_search = false;
        let mut do_next = false;
        let mut do_prev = false;

        ui.horizontal(|ui| {
            ui.label("🔍");
            let response = egui::TextEdit::singleline(&mut self.state.search_query)
                .id(egui::Id::new("search_bar_input"))
                .show(ui)
                .response;

            // Auto-focus when search bar first appears
            if !response.has_focus() && self.state.search_results.is_empty() {
                response.request_focus();
            }

            // Enter = search / next match
            if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                do_search = true;
            }

            // Escape closes search (checked while focused)
            if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                close_search = true;
            }

            // Live search as user types
            if response.changed() {
                do_search = true;
            }

            if ui.button("Next").clicked() {
                do_next = true;
            }
            if ui.button("Prev").clicked() {
                do_prev = true;
            }
            if ui.button("X").clicked() {
                close_search = true;
            }
        });

        // Act on deferred flags (avoids borrow issues inside the closure)
        if close_search {
            self.state.clear_search();
            // Release focus back to the editor area on next frame
            ui.ctx().memory_mut(|m| m.stop_text_input());
        } else if do_search {
            self.state.perform_search();
        } else if do_next {
            self.state.next_search_match();
        } else if do_prev {
            self.state.prev_search_match();
        }

        ui.separator();
    }

    fn handle_input(&mut self, ui: &mut Ui, response: &Response) {
        // Handle clipboard events - support both egui and arboard
        let mut should_copy = false;
        let mut should_cut = false;
        let mut paste_text: Option<String> = None;

        ui.input(|i| {
            for event in &i.events {
                match event {
                    egui::Event::Paste(text) => {
                        paste_text = Some(text.clone());
                    }
                    egui::Event::Copy => {
                        should_copy = true;
                    }
                    egui::Event::Cut => {
                        should_cut = true;
                    }
                    _ => {}
                }
            }
        });

        // Handle copy
        if should_copy {
            self.state.copy_selection();
            if let Some(text) = self.state.get_clipboard_text() {
                ui.ctx().copy_text(text.to_string());
            }
        }

        // Handle cut
        if should_cut {
            self.state.cut_selection();
            if let Some(text) = self.state.get_clipboard_text() {
                ui.ctx().copy_text(text.to_string());
            }
        }

        // Handle paste
        if let Some(text) = paste_text {
            if let Some(selection) = self.state.selection.take() {
                if selection.is_active() {
                    let (start, _) = selection.get_range();
                    self.state.cursor_pos = start;
                }
            }

            // Normalize line endings
            let normalized = text.replace("\r\n", "\n").replace('\r', "\n");

            // Determine wrap width based on available editor area
            let available = ui.available_rect_before_wrap();
            let margin_width = 40.0;
            let char_width = 8.4_f32;
            let avail_cols = if available.width() > margin_width + char_width {
                ((available.width() - margin_width) / char_width).floor() as usize
            } else {
                80_usize
            };

            let wrapped = wrap_text_to_width(&normalized, avail_cols);
            let joined = wrapped.join("\n");

            self.state.insert_text(&joined);
        }

        // Handle text input - but NOT if modifiers are pressed
        ui.input(|i| {
            for event in &i.events {
                if let egui::Event::Text(text) = event {
                    if !i.modifiers.ctrl && !i.modifiers.alt && !i.modifiers.command {
                        if !text.chars().any(|c| c.is_control()) {
                            self.state.insert_text(text);
                        }
                    }
                }
            }
        });

        let has_ctrl = ui.input(|i| i.modifiers.ctrl);
        let has_shift = ui.input(|i| i.modifiers.shift);

        if ui.input(|i| i.key_pressed(egui::Key::Enter)) && !has_ctrl {
            self.state.insert_text("\n");
        }

        if ui.input(|i| i.key_pressed(egui::Key::Backspace)) && !has_ctrl {
            self.state.backspace();
        }

        if ui.input(|i| i.key_pressed(egui::Key::Delete)) && !has_ctrl {
            self.state.delete_at_cursor();
        }

        if ui.input(|i| i.key_pressed(egui::Key::Tab)) && !has_ctrl {
            self.state.insert_text("    ");
        }

        // Ensure cursor remains visible after any movement/paste
        let available = ui.available_rect_before_wrap();
        let visible_rows = (available.height() / 20.0) as usize + 1;
        if !self.state.is_dragging {
            let prev_scroll = self.state.scroll_offset.0;
            self.state.ensure_cursor_visible(Some(visible_rows));
            if self.state.scroll_offset.0 != prev_scroll {
                ui.ctx().request_repaint();
            }
        }

        // Arrow keys with optional shift for selection
        if ui.input(|i| i.key_pressed(egui::Key::ArrowLeft)) && !has_ctrl {
            if has_shift {
                self.move_cursor_with_selection(-1, 0);
            } else {
                self.state.move_cursor(-1, 0);
            }
        }

        if ui.input(|i| i.key_pressed(egui::Key::ArrowRight)) && !has_ctrl {
            if has_shift {
                self.move_cursor_with_selection(1, 0);
            } else {
                self.state.move_cursor(1, 0);
            }
        }

        if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) && !has_ctrl {
            if has_shift {
                self.move_cursor_with_selection(0, -1);
            } else {
                self.state.move_cursor(0, -1);
            }
        }

        if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) && !has_ctrl {
            if has_shift {
                self.move_cursor_with_selection(0, 1);
            } else {
                self.state.move_cursor(0, 1);
            }
        }

        // Home/End
        if ui.input(|i| i.key_pressed(egui::Key::Home)) && !has_ctrl {
            if has_shift {
                self.start_selection_if_needed();
                self.state.cursor_pos.column = 0;
                self.update_selection();
            } else {
                self.state.cursor_pos.column = 0;
                self.state.selection = None;
            }
        }

        if ui.input(|i| i.key_pressed(egui::Key::End)) && !has_ctrl {
            let line_len = self
                .state
                .current_buffer()
                .lines
                .get(self.state.cursor_pos.line)
                .map(|l| l.graphemes(true).count())
                .unwrap_or(0);

            if has_shift {
                self.start_selection_if_needed();
                self.state.cursor_pos.column = line_len;
                self.update_selection();
            } else {
                self.state.cursor_pos.column = line_len;
                self.state.selection = None;
            }
        }

        // Page Up/Down
        if ui.input(|i| i.key_pressed(egui::Key::PageUp)) {
            if has_shift {
                self.start_selection_if_needed();
                self.state.cursor_pos.line = self.state.cursor_pos.line.saturating_sub(20);
                self.update_selection();
            } else {
                self.state.cursor_pos.line = self.state.cursor_pos.line.saturating_sub(20);
                self.state.selection = None;
            }
        }

        if ui.input(|i| i.key_pressed(egui::Key::PageDown)) {
            let max_line = self.state.current_buffer().lines.len().saturating_sub(1);
            if has_shift {
                self.start_selection_if_needed();
                self.state.cursor_pos.line = (self.state.cursor_pos.line + 20).min(max_line);
                self.update_selection();
            } else {
                self.state.cursor_pos.line = (self.state.cursor_pos.line + 20).min(max_line);
                self.state.selection = None;
            }
        }

        // Mouse clicks and dragging
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                self.handle_click(ui, pos);
            }
        }

        if response.dragged() {
            if let Some(pos) = response.interact_pointer_pos() {
                self.handle_drag(ui, pos);
            }
        }

        if response.drag_stopped() {
            self.state.is_dragging = false;
        }
    }

    fn move_cursor_with_selection(&mut self, dx: isize, dy: isize) {
        self.start_selection_if_needed();
        self.move_cursor_internal(dx, dy);
        self.update_selection();
    }

    fn move_cursor_internal(&mut self, dx: isize, dy: isize) {
        if dx < 0 && self.state.cursor_pos.column > 0 {
            self.state.cursor_pos.column -= 1;
        } else if dx > 0 {
            let line_len = self
                .state
                .current_buffer()
                .lines
                .get(self.state.cursor_pos.line)
                .map(|l| l.graphemes(true).count())
                .unwrap_or(0);
            if self.state.cursor_pos.column < line_len {
                self.state.cursor_pos.column += 1;
            }
        }

        if dy < 0 && self.state.cursor_pos.line > 0 {
            self.state.cursor_pos.line -= 1;
            self.clamp_column();
        } else if dy > 0 && self.state.cursor_pos.line < self.state.current_buffer().lines.len() - 1
        {
            self.state.cursor_pos.line += 1;
            self.clamp_column();
        }
    }

    fn start_selection_if_needed(&mut self) {
        if self.state.selection.is_none() {
            self.state.selection = Some(Selection::new(self.state.cursor_pos));
        }
    }

    fn update_selection(&mut self) {
        if let Some(ref mut sel) = self.state.selection {
            sel.update_cursor(self.state.cursor_pos);
        }
    }

    fn clamp_column(&mut self) {
        let line_len = self
            .state
            .current_buffer()
            .lines
            .get(self.state.cursor_pos.line)
            .map(|l| l.graphemes(true).count())
            .unwrap_or(0);
        self.state.cursor_pos.column = self.state.cursor_pos.column.min(line_len);
    }

    fn handle_click(&mut self, ui: &Ui, pos: Pos2) {
        let text_pos = self.screen_pos_to_text_pos(ui, pos);
        self.state.cursor_pos = text_pos;
        self.state.selection = None;
        self.state.is_dragging = true;
    }

    fn handle_drag(&mut self, ui: &Ui, pos: Pos2) {
        if !self.state.is_dragging {
            return;
        }

        let text_pos = self.screen_pos_to_text_pos(ui, pos);

        if self.state.selection.is_none() {
            self.state.selection = Some(Selection::new(self.state.cursor_pos));
        }

        self.state.cursor_pos = text_pos;
        if let Some(ref mut sel) = self.state.selection {
            sel.update_cursor(text_pos);
        }
    }

    fn screen_pos_to_text_pos(&self, ui: &Ui, pos: Pos2) -> TextPosition {
        let row_height = 20.0;
        let margin_width = 40.0;

        let rect = ui.available_rect_before_wrap();

        // Compute line index
        let raw_line =
            ((pos.y - rect.top()) / row_height) as isize + self.state.scroll_offset.0 as isize;
        let mut line = if raw_line < 0 { 0 } else { raw_line as usize };
        let max_line = self.state.current_buffer().lines.len().saturating_sub(1);
        line = line.min(max_line);

        // Compute column by mapping the relative X into grapheme columns using visual widths
        let mut column = 0usize;
        if let Some(text_line) = self.state.current_buffer().lines.get(line) {
            let available_px = (rect.width() - margin_width).max(0.0);
            let rel_x = (pos.x - rect.left() - margin_width)
                .max(0.0)
                .min(available_px);
            column = x_to_column_for_line(rel_x, available_px, text_line);
        }

        TextPosition { line, column }
    }

    fn render_content(&mut self, ui: &mut Ui, response: &Response, rect: Rect) {
        let painter = ui.painter();
        let font_id = FontId::monospace(14.0);
        let row_height = 20.0;
        let margin_width = 40.0;
        let scroll_line = self.state.scroll_offset.0;

        // Handle Mouse Interaction
        if response.clicked() || response.dragged() {
            if let Some(mouse_pos) = response.interact_pointer_pos() {
                // local coords relative to our editor rect
                let local_pos = mouse_pos - rect.min;

                // Do not hold a long borrow to the buffer while we may mutate scroll_offset.
                let line_count = self.state.current_buffer().lines.len();
                // Save previous scroll so we can avoid jumping unless we intentionally auto-scrolled
                let prev_scroll = self.state.scroll_offset.0;

                // compute column using grapheme widths
                // Also implement incremental auto-scroll while dragging when pointer is near top/bottom edge.
                let mut col = 0usize;
                let mut scrolled = false;
                // auto-scroll when dragging near edges
                if response.dragged() || self.state.is_dragging {
                    let edge_px = row_height * 1.0; // 1 row height as edge zone
                    if local_pos.y < edge_px {
                        // scroll up by 1 line
                        if self.state.scroll_offset.0 > 0 {
                            self.state.scroll_offset.0 =
                                self.state.scroll_offset.0.saturating_sub(1);
                            scrolled = true;
                        }
                    } else if local_pos.y > rect.height() - edge_px {
                        // scroll down by 1 line
                        let visible_rows = (rect.height() / row_height) as usize;
                        let max_scroll = self
                            .state
                            .current_buffer()
                            .lines
                            .len()
                            .saturating_sub(visible_rows);
                        if self.state.scroll_offset.0 < max_scroll {
                            self.state.scroll_offset.0 =
                                (self.state.scroll_offset.0 + 1).min(max_scroll);
                            scrolled = true;
                        }
                    }
                }

                let available_px = (rect.width() - margin_width).max(0.0);
                // recompute clicked line using possibly-updated scroll_offset
                // clamp Y so pointer outside the editor doesn't produce huge jumps
                let y_clamped = local_pos.y.max(0.0).min(rect.height().max(1.0) - 0.01);
                let clicked_line = ((y_clamped / row_height) as usize + self.state.scroll_offset.0)
                    .min(line_count.saturating_sub(1));

                if let Some(line) = self.state.current_buffer().lines.get(clicked_line) {
                    let rel_x = (local_pos.x - margin_width).max(0.0).min(available_px);
                    col = x_to_column_for_line(rel_x, available_px, line);
                    col = col.min(grapheme_len(line));
                }

                let new_pos = TextPosition {
                    line: clicked_line,
                    column: col,
                };

                if response.clicked() {
                    self.state.cursor_pos = new_pos;
                    self.state.selection = Some(Selection {
                        anchor: new_pos,
                        cursor: new_pos,
                    });
                    self.state.is_dragging = true;
                    // If we did not auto-scroll, keep previous scroll (avoid jumping)
                    if !scrolled {
                        self.state.scroll_offset.0 = prev_scroll;
                    }
                } else if response.dragged() {
                    self.state.cursor_pos = new_pos;
                    // If drag started without a prior click that set selection, initialize it now
                    if self.state.selection.is_none() {
                        self.state.selection = Some(Selection {
                            anchor: new_pos,
                            cursor: new_pos,
                        });
                    } else if let Some(sel) = &mut self.state.selection {
                        sel.cursor = new_pos;
                    }
                }
            }
        }

        // Draw content
        let visible_rows = (rect.height() / row_height) as usize + 1;
        let end_line = (scroll_line + visible_rows).min(self.state.current_buffer().lines.len());

        // Draw margin background
        let margin_rect =
            Rect::from_min_size(rect.min, egui::Vec2::new(margin_width, rect.height()));
        painter.rect_filled(margin_rect, 0.0, Color32::from_rgb(38, 33, 28));

        let selection_range = self
            .state
            .selection
            .as_ref()
            .filter(|s| s.is_active())
            .map(|s| s.get_range());

        let buffer = self.state.current_buffer();

        // Get current filetype for syntax highlighting
        let filetype = self.state.tab_manager.current_tab().filetype.clone();
        let highlighter = SyntaxHighlighter::new(filetype);

        for (visual_idx, line_idx) in (scroll_line..end_line).enumerate() {
            let y_pos = rect.top() + visual_idx as f32 * row_height;

            // Line number
            painter.text(
                Pos2::new(rect.left() + 5.0, y_pos),
                egui::Align2::LEFT_TOP,
                format!("{:>3}", line_idx + 1),
                FontId::monospace(12.0),
                Color32::from_rgb(200, 160, 100),
            );

            // Line content with syntax highlighting and selection
            if let Some(line) = buffer.lines.get(line_idx) {
                let text_pos = Pos2::new(rect.left() + margin_width, y_pos);

                // Get syntax tokens for this line
                let tokens = highlighter.highlight_line(line);

                if let Some((start, end)) = selection_range {
                    if line_idx >= start.line && line_idx <= end.line {
                        // Line has selection - render with both syntax and selection highlighting
                        let sel_start = if line_idx == start.line {
                            start.column
                        } else {
                            0
                        };
                        let sel_end = if line_idx == end.line {
                            end.column
                        } else {
                            grapheme_len(line)
                        };

                        let available_px = rect.width() - margin_width;
                        let total_cols = line.as_str().width();
                        render_line_with_syntax_and_selection(
                            &painter,
                            &tokens,
                            text_pos,
                            available_px,
                            total_cols,
                            row_height,
                            sel_start,
                            sel_end,
                            &font_id,
                        );
                    } else {
                        // No selection on this line - just render with syntax highlighting
                        let available_px = rect.width() - margin_width;
                        let total_cols = line.as_str().width();
                        render_line_with_syntax(
                            &painter,
                            &tokens,
                            text_pos,
                            &font_id,
                            available_px,
                            total_cols,
                        );
                    }
                } else {
                    // No selection at all - just render with syntax highlighting
                    let available_px = rect.width() - margin_width;
                    let total_cols = line.as_str().width();
                    render_line_with_syntax(
                        &painter,
                        &tokens,
                        text_pos,
                        &font_id,
                        available_px,
                        total_cols,
                    );
                }
            }

            // Cursor
            if self.state.cursor_pos.line == line_idx {
                // Compute cursor X by mapping the cursor column proportionally to available pixel width.
                let available_px = rect.width() - margin_width;
                let mut cx = rect.left() + margin_width;
                if let Some(line_str) = buffer.lines.get(line_idx) {
                    let px =
                        column_to_x_for_line(self.state.cursor_pos.column, available_px, line_str);
                    cx += px;
                }
                painter.line_segment(
                    [Pos2::new(cx, y_pos), Pos2::new(cx, y_pos + row_height)],
                    Stroke::new(2.0, Color32::YELLOW),
                );
            }
        }
    }
}

// Helper function to render a line with syntax highlighting only
fn render_line_with_syntax(
    painter: &egui::Painter,
    tokens: &[crate::core::syntax::Token],
    mut pos: Pos2,
    font_id: &FontId,
    available_px: f32,
    total_cols: usize,
) {
    let total_cols_f = total_cols as f32;
    for token in tokens {
        let (r, g, b) = token.token_type.rgb();
        let color = Color32::from_rgb(r, g, b);
        painter.text(
            pos,
            egui::Align2::LEFT_TOP,
            &token.text,
            font_id.clone(),
            color,
        );

        // Compute token pixel width proportionally to its visual width
        let token_cols = token.text.as_str().width() as f32;
        let token_px = if total_cols_f > 0.0 {
            (token_cols / total_cols_f) * available_px
        } else {
            // fallback: approximate by glyph count
            (token.text.chars().count() as f32) * 8.0
        };

        pos.x += token_px;
    }
}

// Helper function to render a line with both syntax highlighting and selection
fn render_line_with_syntax_and_selection(
    painter: &egui::Painter,
    tokens: &[crate::core::syntax::Token],
    base_pos: Pos2,
    available_px: f32,
    total_cols: usize,
    row_height: f32,
    sel_start: usize,
    sel_end: usize,
    font_id: &FontId,
) {
    let total_cols_f = total_cols as f32;
    let mut char_cursor_pos = 0; // Tracks current grapheme position within the line
    let mut x_offset = 0.0f32;

    for token in tokens {
        let token_text = &token.text;
        let token_grapheme_count = token_text.as_str().graphemes(true).count();

        let (r, g, b) = token.token_type.rgb();
        let syntax_color = Color32::from_rgb(r, g, b);

        // For each grapheme in token, compute pixel width proportionally
        for (i, gstr) in token_text.as_str().graphemes(true).enumerate() {
            let current_grapheme_abs_pos = char_cursor_pos + i;
            let pos = Pos2::new(base_pos.x + x_offset, base_pos.y);
            let gcols = UnicodeWidthStr::width(gstr) as f32;
            let gpx = if total_cols_f > 0.0 {
                (gcols / total_cols_f) * available_px
            } else {
                0.0
            };

            if current_grapheme_abs_pos >= sel_start && current_grapheme_abs_pos < sel_end {
                // Character is selected
                let sel_rect = Rect::from_min_size(pos, egui::Vec2::new(gpx.max(1.0), row_height));
                painter.rect_filled(sel_rect, 0.0, Color32::from_rgb(50, 100, 200));
                painter.text(
                    pos,
                    egui::Align2::LEFT_TOP,
                    gstr,
                    font_id.clone(),
                    Color32::WHITE,
                );
            } else {
                // Character is not selected - use syntax color
                painter.text(
                    pos,
                    egui::Align2::LEFT_TOP,
                    gstr,
                    font_id.clone(),
                    syntax_color,
                );
            }

            x_offset += gpx;
        }

        char_cursor_pos += token_grapheme_count;
    }
}
