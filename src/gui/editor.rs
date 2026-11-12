// editor.rs responsible for rendering the text editor panel in GUI mode, handling input and selection logic
use super::state::EditorState;
use crate::core::graphemes::grapheme_len;
use crate::core::selection::{Selection, TextPosition};
use crate::core::syntax::Token;
use crate::core::wrap::{self, RowPos};
use crate::gui::syntax::{SyntaxHighlighter, TokenTypeExt};
use egui::{
    text::{LayoutJob, TextFormat},
    Color32, FontFamily, FontId, Pos2, Rect, Response, Sense, Stroke, Ui,
};
use unicode_segmentation::UnicodeSegmentation;

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

    let sel_start_byte = sel_start.map(byte_for_char);
    let sel_end_byte = sel_end.map(byte_for_char);

    // Walk tokens, splitting each token's byte range against the selection range.
    let mut byte_pos = 0usize;
    for token in tokens {
        let token_len = token.text.len(); // byte length
        let token_end = byte_pos + token_len;

        let syntax_color = token.token_type.color();

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

pub const SEARCH_INPUT_ID: &str = "search_bar_input";

pub struct EditorPanel<'a> {
    state: &'a mut EditorState,
    accepts_input: bool,
    cell_px: f32, // width of one monospace column, measured from the font
}

impl<'a> EditorPanel<'a> {
    pub fn new(state: &'a mut EditorState, accepts_input: bool) -> Self {
        Self {
            state,
            accepts_input,
            cell_px: 8.4,
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

        // Soft wrap lines to the visible width (one column kept free for the cursor)
        self.cell_px = ui
            .fonts_mut(|f| f.glyph_width(&monospace(), 'M'))
            .max(1.0);
        let text_px = editor_rect.width() - MARGIN_WIDTH - self.cell_px;
        self.state.wrap_cols = ((text_px / self.cell_px).floor() as usize).max(1);

        if was_search_active && !self.state.search_active {
            ui.ctx()
                .memory_mut(|m| m.surrender_focus(egui::Id::new(SEARCH_INPUT_ID)));
        }

        // The editor takes keyboard focus whenever no other widget has it,
        // and keeps arrow keys / tab from moving focus to other widgets.
        if self.accepts_input && ui.memory(|m| m.focused().is_none()) {
            response.request_focus();
        }
        if response.has_focus() {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    response.id,
                    egui::EventFilter {
                        tab: true,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: false,
                    },
                )
            });
        }

        let search_has_focus = ui
            .ctx()
            .memory(|m| m.has_focus(egui::Id::new(SEARCH_INPUT_ID)));
        let cursor_before = self.state.cursor_pos;

        if self.accepts_input && !search_has_focus {
            self.handle_input(ui, &response, editor_rect);
        }

        // Keep the cursor visible after it moved (not while dragging or wheel scrolling)
        if self.state.scroll_to_cursor || self.state.cursor_pos != cursor_before {
            if !self.state.is_dragging {
                self.state.ensure_cursor_visible(visible_rows(editor_rect));
            }
            self.state.scroll_to_cursor = false;
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
                .id(egui::Id::new(SEARCH_INPUT_ID))
                .show(ui)
                .response;

            if self.state.search_focus_requested {
                response.request_focus();
                self.state.search_focus_requested = false;
            }
            // singleline TextEdit drops focus on Enter, so check lost_focus not has_focus
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                if ui.input(|i| i.modifiers.shift) {
                    do_prev = true;
                } else {
                    do_next = true;
                }
                response.request_focus();
            }
            if response.changed() {
                do_search = true;
            }
            if ui.button("Next").on_hover_text("Enter").clicked() {
                do_next = true;
            }
            if ui.button("Prev").on_hover_text("Shift+Enter").clicked() {
                do_prev = true;
            }

            match &self.state.search_state {
                Some(search_state) => {
                    ui.label(format!(
                        "{} of {}",
                        search_state.current_match_idx + 1,
                        search_state.matches.len()
                    ));
                }
                None if !self.state.search_query.is_empty() => {
                    ui.colored_label(Color32::from_rgb(230, 110, 90), "No matches");
                }
                None => {}
            }

            if ui.button("X").on_hover_text("Esc").clicked() {
                close_search = true;
            }
        });

        // egui clears focus on Escape before widgets run, so this can't check has_focus
        if self.accepts_input && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            close_search = true;
        }

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
        let hovered = ui.rect_contains_pointer(editor_rect);
        ui.input(|i| {
            for event in i.events.iter().filter(|_| hovered) {
                if let egui::Event::MouseWheel { delta, .. } = event {
                    scroll_lines += delta.y;
                }
            }
        });
        if scroll_lines != 0.0 {
            // positive delta.y = content moves down = we scroll UP (show earlier rows)
            let steps = (scroll_lines.abs() / 10.0).ceil().max(1.0) as isize * 3;
            let delta = if scroll_lines > 0.0 { -steps } else { steps };
            self.state.scroll_rows(delta, visible_rows(editor_rect));
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
            // insert_text replaces the active selection so we don't duplicate content.
            // Long lines are soft wrapped on screen, so the text goes in unchanged.
            let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
            self.state.insert_text(&normalized);
        }

        // Text input
        ui.input(|i| {
            for event in &i.events {
                if let egui::Event::Text(text) = event {
                    if !i.modifiers.ctrl
                        && !i.modifiers.alt
                        && !i.modifiers.command
                        && !text.chars().any(|c| c.is_control())
                    {
                        self.state.insert_text(text);
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
                .map(|l| grapheme_len(l))
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
                self.clamp_column();
                self.update_selection();
            } else {
                self.state.cursor_pos.line = self.state.cursor_pos.line.saturating_sub(20);
                self.clamp_column();
                self.state.selection = None;
            }
        }
        if ui.input(|i| i.key_pressed(egui::Key::PageDown)) {
            let max_line = self.state.current_buffer().lines.len().saturating_sub(1);
            if has_shift {
                self.start_selection_if_needed();
                self.state.cursor_pos.line = (self.state.cursor_pos.line + 20).min(max_line);
                self.clamp_column();
                self.update_selection();
            } else {
                self.state.cursor_pos.line = (self.state.cursor_pos.line + 20).min(max_line);
                self.clamp_column();
                self.state.selection = None;
            }
        }

        // Mouse click
        if response.clicked() {
            response.request_focus();
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
                let edge = ROW_HEIGHT;

                if local_y < edge {
                    self.state.scroll_rows(-1, visible_rows(editor_rect));
                    ui.ctx().request_repaint();
                } else if local_y > editor_rect.height() - edge {
                    self.state.scroll_rows(1, visible_rows(editor_rect));
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

    // Screen → text position mapping through the wrapped rows
    fn screen_to_text(&self, editor_rect: Rect, pos: Pos2) -> TextPosition {
        let lines = &self.state.current_buffer().lines;
        let cols = self.state.wrap_cols;

        let rows_down = ((pos.y - editor_rect.top()) / ROW_HEIGHT).max(0.0) as usize;
        let rp = wrap::step_forward(lines, cols, self.state.top(), rows_down);

        let rel_x = (pos.x - editor_rect.left() - MARGIN_WIDTH).max(0.0);
        let x = (rel_x / self.cell_px).round() as usize;
        let line = lines.get(rp.line).map(String::as_str).unwrap_or("");

        TextPosition {
            line: rp.line,
            column: wrap::col_at_x(line, cols, rp.row, x),
        }
    }

    // Cursor helpers
    fn move_cursor_with_selection(&mut self, dx: isize, dy: isize) {
        self.start_selection_if_needed();
        self.state.move_cursor(dx, dy);
        self.update_selection();
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
            .map(|l| l.chars().count())
            .unwrap_or(0);
        self.state.cursor_pos.column = self.state.cursor_pos.column.min(line_len);
    }

    // Rendering - each buffer line is drawn as one or more wrapped rows
    fn render_content(&mut self, ui: &mut Ui, rect: Rect) {
        let painter = ui.painter();

        // Margin background
        painter.rect_filled(
            Rect::from_min_size(rect.min, egui::Vec2::new(MARGIN_WIDTH, rect.height())),
            0.0,
            Color32::from_rgb(38, 33, 28),
        );

        let rows_on_screen = (rect.height() / ROW_HEIGHT) as usize + 1;
        let cols = self.state.wrap_cols;
        let cursor = self.state.cursor_pos;

        let selection_range = self
            .state
            .selection
            .as_ref()
            .filter(|s| s.is_active())
            .map(|s| s.get_range());

        let filetype = self.state.tab_manager.current_tab().filetype.clone();
        let highlighter = SyntaxHighlighter::new(filetype);

        let lines = &self.state.current_buffer().lines;
        let mut top = wrap::clamp_row(lines, cols, self.state.top());
        let mut screen_row = 0;

        while screen_row < rows_on_screen {
            let line_idx = top.line;
            let Some(line) = lines.get(line_idx) else {
                break;
            };

            let tokens = highlighter.highlight_line(line);
            // Byte offset of every grapheme, plus one past the end
            let g_bytes: Vec<usize> = line
                .grapheme_indices(true)
                .map(|(b, _)| b)
                .chain(std::iter::once(line.len()))
                .collect();
            let g_count = g_bytes.len() - 1;
            let starts = wrap::row_starts(line, cols);

            // Selection on this line, in graphemes
            let line_sel = selection_range.and_then(|(start, end)| {
                (line_idx >= start.line && line_idx <= end.line).then(|| {
                    let s = if line_idx == start.line { start.column } else { 0 };
                    let e = if line_idx == end.line { end.column } else { g_count };
                    (s.min(g_count), e.min(g_count))
                })
            });

            for row in top.row..starts.len() {
                if screen_row >= rows_on_screen {
                    break;
                }
                let y_pos = rect.top() + screen_row as f32 * ROW_HEIGHT;
                screen_row += 1;

                if row == 0 {
                    painter.text(
                        Pos2::new(rect.left() + 5.0, y_pos),
                        egui::Align2::LEFT_TOP,
                        format!("{:>3}", line_idx + 1),
                        FontId::monospace(12.0),
                        Color32::from_rgb(200, 160, 100),
                    );
                }

                let (g_start, g_end) = wrap::row_range(&starts, row, g_count);
                let (b_start, b_end) = (g_bytes[g_start], g_bytes[g_end]);
                let segment = &line[b_start..b_end];
                // grapheme index on this line -> char index inside the segment
                let char_idx =
                    |g: usize| line[b_start..g_bytes[g.clamp(g_start, g_end)]].chars().count();

                let (sel_start, sel_end) = match line_sel {
                    Some((s, e)) if s < g_end && e > g_start => {
                        (Some(char_idx(s)), Some(char_idx(e)))
                    }
                    _ => (None, None),
                };

                let row_tokens = slice_tokens(&tokens, b_start, b_end);
                let job = build_line_galley(segment, &row_tokens, sel_start, sel_end);
                let galley = painter.layout_job(job);
                let text_pos = Pos2::new(rect.left() + MARGIN_WIDTH, y_pos);
                painter.galley(text_pos, galley.clone(), Color32::WHITE);

                // Cursor
                if cursor.line == line_idx && wrap::row_of(&starts, cursor.column.min(g_count)) == row {
                    let col = char_idx(cursor.column);
                    let cx = rect.left() + MARGIN_WIDTH + grapheme_col_to_px(&galley, col);
                    painter.line_segment(
                        [Pos2::new(cx, y_pos), Pos2::new(cx, y_pos + ROW_HEIGHT)],
                        Stroke::new(2.0, Color32::YELLOW),
                    );
                }
            }

            top = RowPos {
                line: line_idx + 1,
                row: 0,
            };
        }
    }
}

fn visible_rows(editor_rect: Rect) -> usize {
    ((editor_rect.height() / ROW_HEIGHT) as usize).max(1)
}

// The part of the syntax tokens that falls inside byte range [start, end) of the line
fn slice_tokens(tokens: &[Token], start: usize, end: usize) -> Vec<Token> {
    let mut out = Vec::new();
    let mut token_start = 0;
    for token in tokens {
        let token_end = token_start + token.text.len();
        let (s, e) = (token_start.max(start), token_end.min(end));
        if s < e {
            out.push(Token {
                text: token.text[s - token_start..e - token_start].to_string(),
                token_type: token.token_type,
            });
        }
        token_start = token_end;
    }
    out
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
