// src/gui/editor.rs - Editor with syntax highlighting
use super::state::EditorState;
use crate::core::selection::{Selection, TextPosition};
use crate::core::syntax::SyntaxHighlighter;
use egui::{Color32, FontId, Pos2, Rect, Response, Sense, Stroke, Ui};

pub struct EditorPanel<'a> {
    state: &'a mut EditorState,
    accepts_input: bool,
}

impl<'a> EditorPanel<'a> {
    pub fn new(state: &'a mut EditorState, accepts_input: bool) -> Self {
        Self { state, accepts_input }
    }

    pub fn show(&mut self, ui: &mut Ui) -> Response {
        if self.state.search_active {
            self.show_search_bar(ui);
        }

        let available_rect = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(available_rect, Sense::click_and_drag());

        if self.accepts_input {
            self.handle_input(ui, &response);
        }

        self.render_content(ui, &response, available_rect);

        response
    }

    fn show_search_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("🔍");
            let response = ui.text_edit_singleline(&mut self.state.search_query);

            response.request_focus();

            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.state.search_active = false;
                self.state.search_query.clear();
            }

            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.state.perform_search();
            }

            if ui.button("Next").clicked() {
                self.state.next_search_match();
            }

            if ui.button("Prev").clicked() {
                self.state.prev_search_match();
            }

            if ui.button("✕").clicked() {
                self.state.search_active = false;
                self.state.search_query.clear();
            }
        });
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
            self.state.insert_text(&text);
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
                .map(|l| l.len())
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
                .map(|l| l.len())
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
            .map(|l| l.len())
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
        let char_width = 8.4;
        let margin_width = 40.0;

        let rect = ui.available_rect_before_wrap();

        let line = ((pos.y - rect.top()) / row_height) as usize + self.state.scroll_offset.0;
        let column = ((pos.x - rect.left() - margin_width) / char_width).max(0.0) as usize;

        let max_line = self.state.current_buffer().lines.len().saturating_sub(1);
        let line = line.min(max_line);

        let line_len = self
            .state
            .current_buffer()
            .lines
            .get(line)
            .map(|l| l.len())
            .unwrap_or(0);
        let column = column.min(line_len);

        TextPosition { line, column }
    }

    fn render_content(&mut self, ui: &mut Ui, response: &Response, rect: Rect) {
        let painter = ui.painter();
        let font_id = FontId::monospace(14.0);
        let row_height = 20.0;
        let char_width = 8.4;
        let margin_width = 40.0;
        let scroll_line = self.state.scroll_offset.0;
    
        // Handle Mouse Interaction
        if response.clicked() || response.dragged() {
            if let Some(mouse_pos) = response.interact_pointer_pos() {
                let local_pos = mouse_pos - rect.min;
                
                let line_count = self.state.current_buffer().lines.len();
                let clicked_line = ((local_pos.y / row_height) as usize + scroll_line)
                    .min(line_count.saturating_sub(1));
                
                let line_len = self.state.current_buffer().lines[clicked_line].len();
                let clicked_col = ((local_pos.x - margin_width) / char_width).round().max(0.0) as usize;
                let clicked_col = clicked_col.min(line_len);
    
                let new_pos = TextPosition { line: clicked_line, column: clicked_col };
    
                if response.clicked() {
                    self.state.cursor_pos = new_pos;
                    self.state.selection = Some(Selection { anchor: new_pos, cursor: new_pos });
                } else if response.dragged() {
                    self.state.cursor_pos = new_pos;
                    if let Some(sel) = &mut self.state.selection {
                        sel.cursor = new_pos;
                    }
                }
            }
        }
    
        // Draw content
        let visible_rows = (rect.height() / row_height) as usize + 1;
        let end_line = (scroll_line + visible_rows).min(self.state.current_buffer().lines.len());
    
        // Draw margin background
        let margin_rect = Rect::from_min_size(rect.min, egui::Vec2::new(margin_width, rect.height()));
        painter.rect_filled(margin_rect, 0.0, Color32::from_rgb(38, 33, 28));
    
        let selection_range = self.state.selection.as_ref()
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
                        let chars: Vec<char> = line.chars().collect();
                        let sel_start = if line_idx == start.line { start.column } else { 0 };
                        let sel_end = if line_idx == end.line { end.column } else { chars.len() };
                        
                        render_line_with_syntax_and_selection(
                            &painter,
                            &tokens,
                            text_pos,
                            char_width,
                            row_height,
                            sel_start,
                            sel_end,
                            &font_id,
                        );
                    } else {
                        // No selection on this line - just render with syntax highlighting
                        render_line_with_syntax(&painter, &tokens, text_pos, &font_id);
                    }
                } else {
                    // No selection at all - just render with syntax highlighting
                    render_line_with_syntax(&painter, &tokens, text_pos, &font_id);
                }
            }
    
            // Cursor
            if self.state.cursor_pos.line == line_idx {
                let cx = rect.left() + margin_width + (self.state.cursor_pos.column as f32 * char_width);
                painter.line_segment(
                    [Pos2::new(cx, y_pos), Pos2::new(cx, y_pos + row_height)],
                    Stroke::new(2.0, Color32::YELLOW)
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
) {
    for token in tokens {
        let (r, g, b) = token.token_type.rgb();
        let color = Color32::from_rgb(r, g, b);
        painter.text(pos, egui::Align2::LEFT_TOP, &token.text, font_id.clone(), color);
        pos.x += token.text.len() as f32 * 8.4; // char_width
    }
}

// Helper function to render a line with both syntax highlighting and selection
fn render_line_with_syntax_and_selection(
    painter: &egui::Painter,
    tokens: &[crate::core::syntax::Token],
    base_pos: Pos2,
    char_width: f32,
    row_height: f32,
    sel_start: usize,
    sel_end: usize,
    font_id: &FontId,
) {
    let mut char_pos = 0;
    let mut x_offset = 0.0;
    
    for token in tokens {
        let token_len = token.text.chars().count();
        let token_end = char_pos + token_len;
        let (r, g, b) = token.token_type.rgb();
        let syntax_color = Color32::from_rgb(r, g, b);
        
        if token_end <= sel_start || char_pos >= sel_end {
            // Token is completely outside selection
            let pos = Pos2::new(base_pos.x + x_offset, base_pos.y);
            painter.text(pos, egui::Align2::LEFT_TOP, &token.text, font_id.clone(), syntax_color);
            x_offset += token.text.len() as f32 * char_width;
        } else if char_pos >= sel_start && token_end <= sel_end {
            // Token is completely inside selection
            let pos = Pos2::new(base_pos.x + x_offset, base_pos.y);
            let sel_rect = Rect::from_min_size(
                pos,
                egui::Vec2::new(token.text.len() as f32 * char_width, row_height)
            );
            painter.rect_filled(sel_rect, 0.0, Color32::from_rgb(50, 100, 200));
            painter.text(pos, egui::Align2::LEFT_TOP, &token.text, font_id.clone(), Color32::WHITE);
            x_offset += token.text.len() as f32 * char_width;
        } else {
            // Token is partially selected - render character by character
            let token_chars: Vec<char> = token.text.chars().collect();
            for (i, ch) in token_chars.iter().enumerate() {
                let abs_pos = char_pos + i;
                let pos = Pos2::new(base_pos.x + x_offset, base_pos.y);
                
                if abs_pos >= sel_start && abs_pos < sel_end {
                    // Character is selected
                    let sel_rect = Rect::from_min_size(pos, egui::Vec2::new(char_width, row_height));
                    painter.rect_filled(sel_rect, 0.0, Color32::from_rgb(50, 100, 200));
                    painter.text(pos, egui::Align2::LEFT_TOP, ch.to_string(), font_id.clone(), Color32::WHITE);
                } else {
                    // Character is not selected - use syntax color
                    painter.text(pos, egui::Align2::LEFT_TOP, ch.to_string(), font_id.clone(), syntax_color);
                }
                
                x_offset += char_width;
            }
        }
        
        char_pos = token_end;
    }
}