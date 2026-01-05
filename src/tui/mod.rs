// TUI module with undo/redo support
pub mod caret;
mod terminal;
pub mod view;

use crate::core::{actions::Action, edit_history::EditHistory, shortcuts::Shortcuts};
use caret::Caret;
use crossterm::event::{Event, KeyCode, KeyEventKind, read};
use terminal::Terminal;
use view::{Buffer, View};

pub struct TerminalEditor {
    view: View,
    caret: Caret,
    shortcuts: Shortcuts,
    edit_history: EditHistory,
    quit_program: bool,
    has_unsaved_changes: bool,
}

impl TerminalEditor {
    pub fn new(buffer: Buffer) -> Self {
        Self {
            view: View::new(buffer),
            caret: Caret::new(),
            shortcuts: Shortcuts::new(),
            edit_history: EditHistory::new(500), // Keep last 500 operations
            quit_program: false,
            has_unsaved_changes: false,
        }
    }

    pub fn set_filename(&mut self, filename: String) {
        self.view.set_filename(filename);
    }

    pub fn run(&mut self) {
        if let Err(error) = Terminal::initialize(&mut self.view, &mut self.caret) {
            eprintln!("Terminal Initialisation Failed: {:?}", error);
        }

        // Run main loop and display any runtime errors in the footer prompt.
        match self.main_loop() {
            Ok(_) => {}
            Err(e) => {
                // Show error in footer prompt
                self.view.show_prompt(
                    crate::tui::view::PromptKind::Error,
                    format!("Error encountered: {}", e),
                );

                // Try to render the prompt to the user. Ignore render/execute errors
                let _ = self.view.render_if_needed(
                    &self.caret,
                    self.tab_manager.current_tab.has_unsaved_changes,
                );
                let _ = Terminal::execute();
            }
        }

        if let Err(error) = Terminal::terminate() {
            eprintln!("Terminal Termination Failed: {:?}", error);
        }
    }

    fn main_loop(&mut self) -> Result<(), std::io::Error> {
        loop {
            // Auto-clear footer prompts after 2 seconds
            if let Some(since) = self.view.prompt_since {
                if since.elapsed() >= std::time::Duration::from_secs(2) {
                    self.view.clear_prompt();
                    // Try to render the cleared footer; ignore errors to avoid recursive failures.
                    let _ = self.view.render_if_needed(
                        &self.caret,
                        self.tab_manager.current_tab.has_unsaved_changes,
                    );
                    let _ = Terminal::execute();
                }
            }

            match read()? {
                Event::Key(event) => {
                    if event.kind == KeyEventKind::Press {
                        // Handle search navigation when search is active
                        if self.view.is_search_active() {
                            match event.code {
                                KeyCode::Down => {
                                    self.view.next_search_match(&mut self.caret)?;
                                    Terminal::execute()?;
                                    continue;
                                }
                                KeyCode::Up => {
                                    self.view.prev_search_match(&mut self.caret)?;
                                    Terminal::execute()?;
                                    continue;
                                }
                                KeyCode::Esc => {
                                    self.view.clear_search();
                                    self.view.render(&mut self.caret)?;
                                    Terminal::execute()?;
                                    continue;
                                }
                                _ => {
                                    // Any other key exits search mode
                                    self.view.clear_search();
                                }
                            }
                        }
                        
                        if let Some(action) = self.shortcuts.resolve(&event) {
                            match action {
                                // Undo/Redo
                                Action::Undo => {
                                    self.perform_undo()?;
                                }
                                Action::Redo => {
                                    self.perform_redo()?;
                                }
    
                                // Save
                                Action::Save => {
                                    self.save_file()?;
                                }
    
                                // Search in text
                                Action::Search => {
                                    self.view.search(&mut self.caret)?;
                                }
    
                                // Clipboard operations
                                Action::Copy => {
                                    self.view.copy_selection()?;
                                }
                                Action::Paste => {
                                    if let Some(op) =
                                        self.view.paste_from_clipboard(&mut self.caret)?
                                    {
                                        self.edit_history.push(op);
                                        self.has_unsaved_changes = true;
                                    }
                                }
                                Action::Cut => {
                                    if let Some(op) = self.view.cut_selection(&mut self.caret)? {
                                        self.edit_history.push(op);
                                        self.has_unsaved_changes = true;
                                    }
                                }
    
                                // Regular movement (clears selection)
                                Action::Left => self.view.move_left(&mut self.caret)?,
                                Action::Right => self.view.move_right(&mut self.caret)?,
                                Action::Up => self.view.move_up(&mut self.caret)?,
                                Action::Down => self.view.move_down(&mut self.caret)?,
                                Action::Top => self.view.move_top(&mut self.caret)?,
                                Action::Bottom => self.view.move_bottom(&mut self.caret)?,
                                Action::MaxLeft => self.view.move_max_left(&mut self.caret)?,
                                Action::MaxRight => self.view.move_max_right(&mut self.caret)?,
    
                                // Movement with selection (Shift+arrows)
                                Action::SelectLeft => {
                                    self.view.move_with_selection("left", &mut self.caret)?
                                }
                                Action::SelectRight => {
                                    self.view.move_with_selection("right", &mut self.caret)?
                                }
                                Action::SelectUp => {
                                    self.view.move_with_selection("up", &mut self.caret)?
                                }
                                Action::SelectDown => {
                                    self.view.move_with_selection("down", &mut self.caret)?
                                }
                                Action::SelectTop => {
                                    self.view.move_with_selection("top", &mut self.caret)?
                                }
                                Action::SelectBottom => {
                                    self.view.move_with_selection("bottom", &mut self.caret)?
                                }
                                Action::SelectMaxLeft => {
                                    self.view.move_with_selection("max_left", &mut self.caret)?
                                }
                                Action::SelectMaxRight => self
                                    .view
                                    .move_with_selection("max_right", &mut self.caret)?,
                                Action::SelectAll => self.view.select_all(&mut self.caret)?,
    
                                // Editing operations that generate EditOperations
                                Action::NextLine => {
                                    if let Some(op) = self.view.insert_newline(&mut self.caret)? {
                                        self.edit_history.push(op);
                                        self.has_unsaved_changes = true;
                                    }
                                }
                                Action::Backspace => {
                                    if let Some(op) = self.view.backspace(&mut self.caret)? {
                                        self.edit_history.push(op);
                                        self.has_unsaved_changes = true;
                                    }
                                }
                                Action::Delete => {
                                    if let Some(op) = self.view.delete_char(&mut self.caret)? {
                                        self.edit_history.push(op);
                                        self.has_unsaved_changes = true;
                                    }
                                }
    
                                Action::ToggleCtrlShortcuts => {
                                    self.view.toggle_ctrl_shortcuts();
                                    self.view.render(&mut self.caret)?;
                                }
                                Action::New => {
                                    // TODO: Implement new file
                                }
                                Action::Quit => {
                                    // If current buffer has unsaved changes, prompt the user for confirmation.
                                    if self.has_unsaved_changes {
                                        self.view.show_prompt(
                                            crate::tui::view::PromptKind::Error,
                                            "Unsaved changes. Quit without saving? (y/n)"
                                                .to_string(),
                                        );
                                        self.view.needs_redraw = true;
                                        self.view.render_if_needed(
                                            &self.caret,
                                            self.has_unsaved_changes,
                                        )?;
                                        Terminal::execute()?;

                                        // Read simple confirmation (y/n) from user
                                        loop {
                                            match read()? {
                                                Event::Key(event)
                                                    if event.kind == KeyEventKind::Press =>
                                                {
                                                    match event.code {
                                                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                                                            // User confirmed quit
                                                            self.quit_program = true;
                                                            break;
                                                        }
                                                        KeyCode::Char('n')
                                                        | KeyCode::Char('N')
                                                        | KeyCode::Esc => {
                                                            // Cancel quit; clear prompt and continue main loop
                                                            self.view.clear_prompt();
                                                            self.view.render_if_needed(
                                                                &self.caret,
                                                                self.has_unsaved_changes,
                                                            )?;
                                                            Terminal::execute()?;
                                                            break;
                                                        }
                                                        _ => {
                                                            // ignore other keys
                                                        }
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                    } else {
                                        // No unsaved changes -> quit immediately
                                        self.quit_program = true;
                                    }
                                }
                                Action::Print => {
                                    if let KeyCode::Char(character) = event.code {
                                        if let Some(op) =
                                            self.view.type_character(character, &mut self.caret)?
                                        {
                                            self.edit_history.push(op);
                                            self.has_unsaved_changes = true;
                                        }
                                    }
                                }
                                _ => {}
                            }
                            self.view
                                .render_if_needed(&self.caret, self.has_unsaved_changes)?;
                            Terminal::execute()?;
                        }
                    }
                }
                Event::Mouse(mouse_event) => {
                    if let Some(action) = self.shortcuts.resolve_mouse(&mouse_event) {
                        match action {
                            Action::MouseDown(x, y) => {
                                self.view.handle_mouse_down(x, y, &mut self.caret)?;
                            }
                            Action::MouseDrag(x, y) => {
                                self.view.handle_mouse_drag(x, y, &mut self.caret)?;
                            }
                            Action::MouseUp(x, y) => {
                                self.view.handle_mouse_up(x, y, &mut self.caret)?;
                            }
                            Action::MouseDoubleClick(x, y) => {
                                self.view.handle_double_click(x, y, &mut self.caret)?;
                            }
                            Action::MouseTripleClick(x, y) => {
                                self.view.handle_triple_click(x, y, &mut self.caret)?;
                            }
                            _ => {}
                        }
                        Terminal::execute()?;
                    }
                }
                Event::Resize(_, _) => self
                    .view
                    .handle_resize(&mut self.caret, self.has_unsaved_changes)?,
                _ => {}
            }
            if self.quit_program {
                break;
            }
        }
        Ok(())
    }

    fn perform_undo(&mut self) -> Result<(), std::io::Error> {
        if let Some(operation) = self.edit_history.undo() {
            // Reverse the edit
            operation.edit.reverse(&mut self.view.buffer.lines);

            // Restore cursor and scroll position
            self.view.scroll_offset = operation.scroll_before;
            self.view.needs_redraw = true;
            self.view
                .render_if_needed(&self.caret, self.has_unsaved_changes)?;
            self.caret.move_to(operation.cursor_before)?;

            self.has_unsaved_changes = true;
        }
        Ok(())
    }

    fn perform_redo(&mut self) -> Result<(), std::io::Error> {
        if let Some(operation) = self.edit_history.redo() {
            // Apply the edit
            operation.edit.apply(&mut self.view.buffer.lines);

            // Restore cursor and scroll position
            self.view.scroll_offset = operation.scroll_after;
            self.view.needs_redraw = true;
            self.view
                .render_if_needed(&self.caret, self.has_unsaved_changes)?;
            self.caret.move_to(operation.cursor_after)?;

            self.has_unsaved_changes = true;
        }
        Ok(())
    }

    fn save_file(&mut self) -> Result<(), std::io::Error> {
        use std::fs;
    
        if let Some(ref filename) = self.view.filename {
            // Find last non-empty line to avoid saving empty buffer space
            let last_line = self
                .view
                .buffer
                .lines
                .iter()
                .rposition(|line| !line.is_empty())
                .unwrap_or(0);
    
            // Get only the actual content lines
            let content_lines: Vec<String> = self
                .view
                .buffer
                .lines
                .iter()
                .take(last_line + 1)
                .cloned()
                .collect();
    
            // Join with newlines
            let content = content_lines.join("\n");
    
            // Write to file
            match fs::write(filename, content) {
                Ok(_) => {
                    self.has_unsaved_changes = false;
    
                    // Mark for redraw to update footer status
                    self.view.needs_redraw = true;
                    self.view
                        .render_if_needed(&self.caret, self.has_unsaved_changes)?;
                    Terminal::execute()?;
                }
                Err(e) => {
                    // TODO: Show error in footer
                    self.view.needs_redraw = true;
                    Terminal::execute()?;
                    return Err(e);
                }
            }
        } else {
            // SaveAs prompt
            self.view.show_prompt(
                crate::tui::view::PromptKind::SaveAs,
                "Save as: ".to_string(),
            );
            self.view.needs_redraw = true;
            self.view
                .render_if_needed(&self.caret, self.has_unsaved_changes)?;
            Terminal::execute()?;
    
            // Event loop to capture prompt input (keeps editor in raw mode)
            loop {
                match read()? {
                    Event::Key(event) if event.kind == KeyEventKind::Press => {
                        match event.code {
                            KeyCode::Char(c) => {
                                // append typed character to prompt input
                                self.view.append_prompt_char(c);
                            }
                            KeyCode::Backspace => {
                                self.view.backspace_prompt();
                            }
                            KeyCode::Enter => {
                                // On Enter, take current prompt input and attempt save
                                if let Some((_kind, _msg, input)) = self.view.get_prompt() {
                                    let filename = input.to_string();
                                    // Clear prompt immediately (UI will update)
                                    self.view.clear_prompt();
    
                                    if filename.is_empty() {
                                        // nothing entered -> cancel
                                        break;
                                    }
    
                                    // Set filename and write file
                                    self.view.set_filename(filename.clone());
    
                                    // Find last non-empty line to avoid saving empty buffer space
                                    let last_line = self
                                        .view
                                        .buffer
                                        .lines
                                        .iter()
                                        .rposition(|line| !line.is_empty())
                                        .unwrap_or(0);
    
                                    // Get only the actual content lines
                                    let content_lines: Vec<String> = self
                                        .view
                                        .buffer
                                        .lines
                                        .iter()
                                        .take(last_line + 1)
                                        .cloned()
                                        .collect();
    
                                    // Join with newlines
                                    let content = content_lines.join("\n");
    
                                    // Write to file
                                    match fs::write(&filename, content) {
                                        Ok(_) => {
                                            self.has_unsaved_changes = false;
                                            self.view.needs_redraw = true;
                                            self.view.render_if_needed(
                                                &self.caret,
                                                self.has_unsaved_changes,
                                            )?;
                                            Terminal::execute()?;
                                        }
                                        Err(e) => {
                                            // Show error in prompt footer so it's visible to user
                                            self.view.show_prompt(
                                                crate::tui::view::PromptKind::Error,
                                                format!("Failed to save: {}", e),
                                            );
                                            self.view.needs_redraw = true;
                                            self.view.render_if_needed(
                                                &self.caret,
                                                self.has_unsaved_changes,
                                            )?;
                                            Terminal::execute()?;
                                            return Err(e);
                                        }
                                    }
                                }
                                break;
                            }
                            KeyCode::Esc => {
                                // Cancel prompt
                                self.view.clear_prompt();
                                self.view.needs_redraw = true;
                                self.view
                                    .render_if_needed(&self.caret, self.has_unsaved_changes)?;
                                Terminal::execute()?;
                                break;
                            }
                            _ => {}
                        }
    
                        // Re-render footer after each key handled
                        self.view
                            .render_if_needed(&self.caret, self.has_unsaved_changes)?;
                        Terminal::execute()?;
                    }
                    _ => {}
                }
            }
        }
    
        Ok(())
    }
}