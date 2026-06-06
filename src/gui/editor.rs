// editor.rs responsible for
use super::state::EditorState;
use crate::core::graphemes::wrap_text_to_width;
use crate::core::selection::{Selection, TextPosition};
use crate::core::syntax::SyntaxHighlighter;
use egui::{
    text::{LayoutJob, TextFormat},
    Color32, FontFamily, FontId, Pos2, Rect, Response, Sense, Stroke, Ui,
};
use unicode_width::UnicodeWidthChar;

const ROW_HEIGHT: f32 = 20.0;
const MARGIN_WIDTH: f32 = 40.0;
const FONT_SIZE: f32 = 14.0;

fn monospace() -> FontId {
    FontId::new(FONT_SIZE, FontFamily::Monospace)
}

fn build_line_galley(
    line: &str,
    tokens: &[crate::core::syntax::Token],
    sel_start: Option<usize>, // character indices into the line
    sel_end: Option<usize>,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = f32::INFINITY; // never wrap inside the editor

    let sel_bg = Color32::from_rgb(50, 100, 200);
    let sel_fg = Color32::WHITE;

    // We need byte ranges for LayoutSection.
    // Build a mapping: character_index → byte_offset in `line`
    let char_byte_offsets: Vec<usize> = {
        let mut v: Vec<usize> = line.char_indices().map(|(byte_idx, _)| byte_idx).collect();
        v.push(line.len()); // sentinel: one past the end
        v
    };

    let byte_for_char = |c: usize| -> usize { *char_byte_offsets.get(c).unwrap_or(&line.len()) };

    let sel_start_byte = sel_start.map(|c| byte_for_char(c));
    let sel_end_byte = sel_end.map(|c| byte_for_char(c));

    // Walk tokens, splitting each token's byte range against the selection range.
    let mut byte_pos = 0usize;
    for token in tokens {
        let token_len = token.text.len(); // byte length
        let token_end = byte_pos + token_len;

        let (r, g, b) = token.token_type.rgb();
        let syntax_color = Color32::from_rgb(r, g, b);

        // Determine overlap with selection
        let sel_s = sel_start_byte.unwrap_or(usize::MAX);
        let sel_e = sel_end_byte.unwrap_or(0);

        if sel_s >= sel_e || token_end <= sel_s || byte_pos >= sel_e {
            // No selection overlap — emit whole token with syntax colour
            job.append(
                &token.text,
                0.0,
                TextFormat {
                    font_id: monospace(),
                    color: syntax_color,
                    background: Color32::TRANSPARENT,
                    ..Default::default()
                },
            );
        } else {
            // Partial or full overlap — split into up to 3 parts
            let parts = [
                (byte_pos, sel_s.min(token_end), false),
                (sel_s.max(byte_pos), sel_e.min(token_end), true),
                (sel_e.max(byte_pos), token_end, false),
            ];
            for (start, end, selected) in parts {
                if start >= end {
                    continue;
                }
                let text = &line[start..end];
                if text.is_empty() {
                    continue;
                }
                job.append(
                    text,
                    0.0,
                    TextFormat {
                        font_id: monospace(),
                        color: if selected { sel_fg } else { syntax_color },
                        background: if selected {
                            sel_bg
                        } else {
                            Color32::TRANSPARENT
                        },
                        ..Default::default()
                    },
                );
            }
        }

        byte_pos = token_end;
    }

    // If there's remaining text after all tokens (shouldn't happen but be safe)
    if byte_pos < line.len() {
        job.append(
            &line[byte_pos..],
            0.0,
            TextFormat {
                font_id: monospace(),
                color: Color32::WHITE,
                background: Color32::TRANSPARENT,
                ..Default::default()
            },
        );
    }

    // Always add at least an empty section so galley has correct height
    if job.sections.is_empty() {
        job.append(
            "",
            0.0,
            TextFormat {
                font_id: monospace(),
                color: Color32::WHITE,
                ..Default::default()
            },
        );
    }

    job
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

        // Capture the full editor rect BEFORE allocating it.
        let editor_rect = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(editor_rect, Sense::click_and_drag());

        if was_search_active && !self.state.search_active {
            ui.ctx()
                .memory_mut(|m| m.surrender_focus(egui::Id::new("search_bar_input")));
        }

        if self.accepts_input {
            self.handle_input(ui, &response, editor_rect);
        }

        self.render_content(ui, editor_rect);
        response
    }

    // Search bar
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

            if !response.has_focus() && self.state.search_results.is_empty() {
                response.request_focus();
            }
            if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                do_search = true;
            }
            if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                close_search = true;
            }
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

        if close_search {
            self.state.clear_search();
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

    // Input handling
    fn handle_input(&mut self, ui: &mut Ui, response: &Response, editor_rect: Rect) {
        // Mouse scroll (wheel)
        let mut scroll_lines: f32 = 0.0;
        ui.input(|i| {
            for event in &i.events {
                if let egui::Event::MouseWheel { delta, .. } = event {
                    scroll_lines += delta.y;
                }
            }
        });
        if scroll_lines != 0.0 {
            let visible_rows = (editor_rect.height() / ROW_HEIGHT) as usize;
            let total_lines = self.state.current_buffer().lines.len();
            let max_scroll = total_lines.saturating_sub(visible_rows);
            // positive delta.y = content moves down = we scroll UP (show earlier lines)
            if scroll_lines > 0.0 {
                let steps = (scroll_lines / 10.0).ceil().max(1.0) as usize * 3;
                self.state.scroll_offset.0 = self.state.scroll_offset.0.saturating_sub(steps);
            } else {
                let steps = (scroll_lines.abs() / 10.0).ceil().max(1.0) as usize * 3;
                self.state.scroll_offset.0 = (self.state.scroll_offset.0 + steps).min(max_scroll);
            }
            ui.ctx().request_repaint();
        }

        // Clipboard
        let mut should_copy = false;
        let mut should_cut = false;
        let mut paste_text: Option<String> = None;

        ui.input(|i| {
            for event in &i.events {
                match event {
                    egui::Event::Paste(t) => paste_text = Some(t.clone()),
                    egui::Event::Copy => should_copy = true,
                    egui::Event::Cut => should_cut = true,
                    _ => {}
                }
            }
        });

        if should_copy {
            self.state.copy_selection();
            if let Some(text) = self.state.get_clipboard_text() {
                ui.ctx().copy_text(text.to_string());
            }
        }
        if should_cut {
            self.state.cut_selection();
            if let Some(text) = self.state.get_clipboard_text() {
                ui.ctx().copy_text(text.to_string());
            }
        }
        if let Some(text) = paste_text {
            // Compute approximate max columns based on editor_rect width so we emulate TUI wrapping.
            let text_area_px = (editor_rect.width() - MARGIN_WIDTH).max(0.0);
            let cell_px = 8.4_f32; // same heuristic used elsewhere
            let max_cols = (text_area_px / cell_px) as usize;

            if let Some(sel) = self.state.selection.take() {
                if sel.is_active() {
                    // Delete the active selection before inserting pasted text so we don't duplicate content.
                    self.delete_selection_inline(sel);
                }
            }

            let normalized = text.replace("\r\n", "\n").replace('\r', "\n");

            if max_cols > 0 {
                let wrapped = wrap_text_to_width(&normalized, max_cols);
                let joined = wrapped.join("\n");
                self.state.insert_text(&joined);
            } else {
                self.state.insert_text(&normalized);
            }
        }

        // Text input
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

        // Arrow keys
        if ui.input(|i| i.key_pressed(egui::Key::ArrowLeft)) && !has_ctrl {
            if has_shift {
                self.move_cursor_with_selection(-1, 0);
            } else {
                self.state.selection = None;
                self.state.move_cursor(-1, 0);
            }
        }
        if ui.input(|i| i.key_pressed(egui::Key::ArrowRight)) && !has_ctrl {
            if has_shift {
                self.move_cursor_with_selection(1, 0);
            } else {
                self.state.selection = None;
                self.state.move_cursor(1, 0);
            }
        }
        if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) && !has_ctrl {
            if has_shift {
                self.move_cursor_with_selection(0, -1);
            } else {
                self.state.selection = None;
                self.state.move_cursor(0, -1);
            }
        }
        if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) && !has_ctrl {
            if has_shift {
                self.move_cursor_with_selection(0, 1);
            } else {
                self.state.selection = None;
                self.state.move_cursor(0, 1);
            }
        }

        // Home / End
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
                .map(|l| l.chars().count())
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

        // Page Up / Down
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

        // Mouse click
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let text_pos = self.screen_to_text(editor_rect, pos);
                self.state.cursor_pos = text_pos;
                // Clear selection on simple click
                self.state.selection = None;
                ui.ctx().request_repaint();
            }
        }

        // Mouse drag
        if response.drag_started() {
            if let Some(pos) = response.interact_pointer_pos() {
                let text_pos = self.screen_to_text(editor_rect, pos);
                self.state.cursor_pos = text_pos;
                self.state.selection = Some(Selection {
                    anchor: text_pos,
                    cursor: text_pos,
                });
                self.state.is_dragging = true;
            }
        }

        if response.dragged() && self.state.is_dragging {
            if let Some(pos) = response.interact_pointer_pos() {
                // Auto-scroll when dragging near top/bottom edge
                let local_y = pos.y - editor_rect.top();
                let visible_rows = (editor_rect.height() / ROW_HEIGHT) as usize;
                let max_scroll = self
                    .state
                    .current_buffer()
                    .lines
                    .len()
                    .saturating_sub(visible_rows);
                let edge = ROW_HEIGHT;

                if local_y < edge && self.state.scroll_offset.0 > 0 {
                    self.state.scroll_offset.0 = self.state.scroll_offset.0.saturating_sub(1);
                    ui.ctx().request_repaint();
                } else if local_y > editor_rect.height() - edge
                    && self.state.scroll_offset.0 < max_scroll
                {
                    self.state.scroll_offset.0 = (self.state.scroll_offset.0 + 1).min(max_scroll);
                    ui.ctx().request_repaint();
                }

                let text_pos = self.screen_to_text(editor_rect, pos);
                self.state.cursor_pos = text_pos;
                if let Some(ref mut sel) = self.state.selection {
                    sel.cursor = text_pos;
                }
            }
        }

        if response.drag_stopped() {
            self.state.is_dragging = false;
            // Clear selection if it covers zero range (pure click with no movement)
            if let Some(ref sel) = self.state.selection {
                if !sel.is_active() {
                    self.state.selection = None;
                }
            }
        }
    }

    // Screen → text position mapping
    // Uses editor_rect (captured before allocation) for correct coordinate mapping
    fn screen_to_text(&self, editor_rect: Rect, pos: Pos2) -> TextPosition {
        let row_height = ROW_HEIGHT;
        let margin_width = MARGIN_WIDTH;

        let raw_line = ((pos.y - editor_rect.top()) / row_height) as isize
            + self.state.scroll_offset.0 as isize;
        let line = if raw_line < 0 {
            0usize
        } else {
            (raw_line as usize).min(self.state.current_buffer().lines.len().saturating_sub(1))
        };

        let column = if let Some(text_line) = self.state.current_buffer().lines.get(line) {
            let rel_x = (pos.x - editor_rect.left() - margin_width).max(0.0);
            // Walk characters, accumulating width, and find which one the click lands on.
            // We don't know exact font metrics here, so we use a fixed cell width per
            // Unicode column (same as used in render).
            // A monospace 14px font has ~8.4px per cell — but we query the actual galley
            // width from egui for accuracy. Without that, we use the character-count
            // heuristic which is good enough for monospace ASCII and close for unicode.
            x_to_grapheme_col(rel_x, text_line)
        } else {
            0
        };

        TextPosition { line, column }
    }

    // Cursor helpers
    fn move_cursor_with_selection(&mut self, dx: isize, dy: isize) {
        self.start_selection_if_needed();
        self.move_cursor_raw(dx, dy);
        self.update_selection();
    }

    fn move_cursor_raw(&mut self, dx: isize, dy: isize) {
        if dx < 0 && self.state.cursor_pos.column > 0 {
            self.state.cursor_pos.column -= 1;
        } else if dx > 0 {
            let line_len = self
                .state
                .current_buffer()
                .lines
                .get(self.state.cursor_pos.line)
                .map(|l| l.chars().count())
                .unwrap_or(0);
            if self.state.cursor_pos.column < line_len {
                self.state.cursor_pos.column += 1;
            }
        }
        if dy < 0 && self.state.cursor_pos.line > 0 {
            self.state.cursor_pos.line -= 1;
            self.clamp_column();
        } else if dy > 0
            && self.state.cursor_pos.line
                < self.state.current_buffer().lines.len().saturating_sub(1)
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

    fn delete_selection_inline(&mut self, selection: Selection) {
        let (start, end) = selection.get_range();
        let buffer = self.state.current_buffer_mut();

        if start.line == end.line {
            // Single line deletion
            if let Some(line) = buffer.lines.get_mut(start.line) {
                let line_chars = line.chars().count();
                let start_col = start.column.min(line_chars);
                let end_col = end.column.min(line_chars);

                let byte_start = char_to_byte_idx(line, start_col);
                let byte_end = char_to_byte_idx(line, end_col);
                line.drain(byte_start..byte_end);
            }
        } else {
            // Multi-line deletion: merge before/after parts
            let before_text = if let Some(line) = buffer.lines.get(start.line) {
                char_slice(line, 0, start.column.min(line.chars().count()))
            } else {
                String::new()
            };

            let after_text = if let Some(line) = buffer.lines.get(end.line) {
                let gcount = line.chars().count();
                char_slice(line, end.column.min(gcount), gcount)
            } else {
                String::new()
            };

            // Remove all lines in range
            for _ in start.line..=end.line {
                if start.line < buffer.lines.len() {
                    buffer.lines.remove(start.line);
                }
            }

            // Insert merged line
            buffer
                .lines
                .insert(start.line, format!("{}{}", before_text, after_text));
        }

        // Move cursor to start of selection
        self.state.cursor_pos = start;
        self.state.mark_dirty();
    }

    fn clamp_column(&mut self) {
        let line_len = self
            .state
            .current_buffer()
            .lines
            .get(self.state.cursor_pos.line)
            .map(|l| l.chars().count())
            .unwrap_or(0);
        self.state.cursor_pos.column = self.state.cursor_pos.column.min(line_len);
    }

    // Rendering
    fn render_content(&mut self, ui: &mut Ui, rect: Rect) {
        let painter = ui.painter();

        // Margin background
        painter.rect_filled(
            Rect::from_min_size(rect.min, egui::Vec2::new(MARGIN_WIDTH, rect.height())),
            0.0,
            Color32::from_rgb(38, 33, 28),
        );

        let scroll_line = self.state.scroll_offset.0;
        let visible_rows = (rect.height() / ROW_HEIGHT) as usize + 1;
        let end_line = (scroll_line + visible_rows).min(self.state.current_buffer().lines.len());

        let selection_range = self
            .state
            .selection
            .as_ref()
            .filter(|s| s.is_active())
            .map(|s| s.get_range());

        let filetype = self.state.tab_manager.current_tab().filetype.clone();
        let highlighter = SyntaxHighlighter::new(filetype);

        let buffer_lines: Vec<String> = self.state.current_buffer().lines.clone();

        for (visual_idx, line_idx) in (scroll_line..end_line).enumerate() {
            let y_pos = rect.top() + visual_idx as f32 * ROW_HEIGHT;

            // Line number
            painter.text(
                Pos2::new(rect.left() + 5.0, y_pos),
                egui::Align2::LEFT_TOP,
                format!("{:>3}", line_idx + 1),
                FontId::monospace(12.0),
                Color32::from_rgb(200, 160, 100),
            );

            if let Some(line) = buffer_lines.get(line_idx) {
                let tokens = highlighter.highlight_line(line);

                // Determine selection within this line (in character indices)
                let (sel_start, sel_end) = if let Some((start, end)) = selection_range {
                    if line_idx >= start.line && line_idx <= end.line {
                        let s = if line_idx == start.line {
                            Some(start.column)
                        } else {
                            Some(0)
                        };
                        let e = if line_idx == end.line {
                            Some(end.column)
                        } else {
                            Some(line.chars().count())
                        };
                        (s, e)
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                };

                // Build galley via LayoutJob — egui handles all Unicode correctly
                let job = build_line_galley(line, &tokens, sel_start, sel_end);
                let galley = painter.layout_job(job);
                let text_pos = Pos2::new(rect.left() + MARGIN_WIDTH, y_pos);
                painter.galley(text_pos, galley.clone(), Color32::WHITE);

                // Cursor
                if self.state.cursor_pos.line == line_idx {
                    // Use galley cursor position for correct pixel offset
                    let col = self.state.cursor_pos.column;
                    let cx = rect.left() + MARGIN_WIDTH + grapheme_col_to_px(&galley, col);
                    painter.line_segment(
                        [Pos2::new(cx, y_pos), Pos2::new(cx, y_pos + ROW_HEIGHT)],
                        Stroke::new(2.0, Color32::YELLOW),
                    );
                }
            }
        }
    }
}

// Helper: get byte index for a given character index (0-based). Falls back to end-of-string.
fn char_to_byte_idx(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

// Helper: slice string by character indices [start, end)
fn char_slice(s: &str, start: usize, end: usize) -> String {
    s.chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
}

/// Convert a pixel X offset to a grapheme (character) column index within a line.
/// Uses a fixed monospace cell width per Unicode column.
fn x_to_grapheme_col(rel_x: f32, line: &str) -> usize {
    // For monospace font at 14px, each cell is approximately 8.4px.
    // This won't be pixel-perfect for wide chars but is correct for ASCII
    // and reasonable for CJK. We walk characters and pick the closest boundary.
    let cell_px = 8.4_f32;
    let mut acc = 0.0f32;
    for (i, ch) in line.chars().enumerate() {
        let w = ch.width().unwrap_or(1) as f32 * cell_px;
        if rel_x < acc + w / 2.0 {
            return i;
        }
        acc += w;
    }
    line.chars().count()
}

/// Convert a character column index to a pixel X offset using the actual rendered galley.
/// galley.rows is Vec<PlacedRow>; each PlacedRow has .pos (offset) and .row (Arc<Row>).
/// Row.glyphs is Vec<Glyph>; each Glyph has .pos (relative to row) and .advance_width.
fn grapheme_col_to_px(galley: &egui::text::Galley, col: usize) -> f32 {
    if let Some(placed_row) = galley.rows.first() {
        let row = &placed_row.row;
        let row_x = placed_row.pos.x; // offset of this row within the galley
        if col == 0 {
            return row_x;
        }
        let glyph_count = row.glyphs.len();
        if col >= glyph_count {
            // Past the last glyph — place cursor after it
            if let Some(last) = row.glyphs.last() {
                return row_x + last.pos.x + last.advance_width;
            }
            return row_x;
        }
        return row_x + row.glyphs[col].pos.x;
    }
    // Fallback: estimated cell width
    col as f32 * 8.4
}
