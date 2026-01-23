// module binding tui logic, consumeing shortcuts and save logic
pub mod caret;
mod terminal;
pub mod syntax;
pub mod view;

use crate::core::{
    actions::Action, 
    shortcuts::Shortcuts, 
    tabs::{TabManager, get_friendly_filetype},
    updater::Updater,
};
use caret::Caret;
use crossterm::event::{read, Event, KeyCode, KeyEventKind};
use terminal::Terminal;
use view::{Buffer, View};

pub struct TerminalEditor {
    tab_manager: TabManager,
    view: View,
    caret: Caret,
    shortcuts: Shortcuts,
    quit_program: bool,
    pub start_on_tab_zero: bool,
}

impl TerminalEditor {
    pub fn new(buffer: Buffer) -> Self {
        Self {
            tab_manager: TabManager::new(buffer.clone(), None, None),
            view: View::new(buffer),
            caret: Caret::new(),
            shortcuts: Shortcuts::new(),
            quit_program: false,
            start_on_tab_zero: true,
        }
    }

    pub fn new_with_file(path: &str) -> Result<Self, std::io::Error> {
        let mut tab_manager = TabManager::new(Buffer::default(), None, None);
        tab_manager.open_file_in_new_tab(path)?;

        let tab = tab_manager.current_tab();
        let view = View::new(tab.buffer.clone());

        Ok(Self {
            tab_manager,
            view,
            caret: Caret::new(),
            shortcuts: Shortcuts::new(),
            quit_program: false,
            start_on_tab_zero: false,
        })
    }
    
    pub fn open_file_in_new_tab(&mut self) -> Result<(), std::io::Error> {
        self.sync_tab_to_view();
        self.tab_manager.new_tab();
        self.sync_view_to_tab();
        self.caret.move_to(caret::Position::default())?;
        self.view.render(&mut self.caret)?;
        Ok(())
    }
    
    pub fn set_filename_and_filetype(&mut self, filename: Option<String>, filetype: Option<String>) {
        self.tab_manager.current_tab_mut().filename = filename.clone();
        self.view.set_filename_and_filetype(filename, filetype);
    }
    
    fn check_and_install_update(&mut self) -> Result<(), std::io::Error> {
        // Show checking message
        self.view.show_prompt(
            crate::tui::view::PromptKind::SearchInfo,
            "Checking for updates...".to_string(),
        );
        self.view.needs_redraw = true;
        self.view.render_if_needed(&self.caret, false)?;
        Terminal::execute()?;
        
        // Check for updates in a separate thread to avoid blocking
        let updater = Updater::new();
        
        let update_info = match updater.check_for_updates() {
            Ok(info) => info,
            Err(e) => {
                self.view.show_prompt(
                    crate::tui::view::PromptKind::Error,
                    format!("Failed to check for updates: {}", e),
                );
                self.view.render_if_needed(&self.caret, false)?;
                Terminal::execute()?;
                std::thread::sleep(std::time::Duration::from_secs(3));
                self.view.clear_prompt();
                self.view.render_if_needed(&self.caret, false)?;
                Terminal::execute()?;
                return Ok(());
            }
        };
        
        if !update_info.update_available {
            self.view.show_prompt(
                crate::tui::view::PromptKind::SearchInfo,
                format!("You're running the latest version ({})", update_info.current_version),
            );
            self.view.render_if_needed(&self.caret, false)?;
            Terminal::execute()?;
            std::thread::sleep(std::time::Duration::from_secs(2));
            self.view.clear_prompt();
            self.view.render_if_needed(&self.caret, false)?;
            Terminal::execute()?;
            return Ok(());
        }
        
        // Update available - show release notes and prompt
        let message = format!(
            "Update available: v{} → v{} | Press Y to install, N to cancel",
            update_info.current_version,
            update_info.latest_version
        );
        
        self.view.show_prompt(
            crate::tui::view::PromptKind::SearchInfo,
            message,
        );
        self.view.needs_redraw = true;
        self.view.render_if_needed(&self.caret, false)?;
        Terminal::execute()?;
        
        // Wait for user confirmation
        loop {
            match read()? {
                Event::Key(event) if event.kind == KeyEventKind::Press => {
                    match event.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            // User confirmed - perform update
                            self.view.show_prompt(
                                crate::tui::view::PromptKind::SearchInfo,
                                "Downloading update...".to_string(),
                            );
                            self.view.render_if_needed(&self.caret, false)?;
                            Terminal::execute()?;
                            
                            match updater.perform_update() {
                                Ok(_) => {
                                    self.view.show_prompt(
                                        crate::tui::view::PromptKind::SearchInfo,
                                        "Update successful! Restart the application to use the new version.".to_string(),
                                    );
                                    self.view.render_if_needed(&self.caret, false)?;
                                    Terminal::execute()?;
                                    std::thread::sleep(std::time::Duration::from_secs(3));
                                    
                                    // Optionally quit after update
                                    self.quit_program = true;
                                }
                                Err(e) => {
                                    self.view.show_prompt(
                                        crate::tui::view::PromptKind::Error,
                                        format!("Update failed: {}", e),
                                    );
                                    self.view.render_if_needed(&self.caret, false)?;
                                    Terminal::execute()?;
                                    std::thread::sleep(std::time::Duration::from_secs(3));
                                }
                            }
                            break;
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            // User cancelled
                            self.view.clear_prompt();
                            self.view.render_if_needed(&self.caret, false)?;
                            Terminal::execute()?;
                            break;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        
        self.view.clear_prompt();
        self.view.render_if_needed(&self.caret, false)?;
        Terminal::execute()?;
        
        Ok(())
    }


    pub fn run(&mut self) {
        if let Err(error) = Terminal::initialize(&mut self.view, &mut self.caret) {
            eprintln!("Terminal Initialisation Failed: {:?}", error);
        }

        // Create a new tab at start if quick started without a path
        if self.start_on_tab_zero {
            let _ = match self.open_file_in_new_tab() {
                Ok(_) => Ok(()),
                Err(e) => Err(e),
            };
        }
        
        self.sync_view_to_tab();
        self.caret
            .move_to(self.tab_manager.current_tab().cursor_pos)
            .ok();

        match self.main_loop() {
            Ok(_) => {}
            Err(e) => {
                self.view
                    .show_prompt(crate::tui::view::PromptKind::Error, format!("Error: {}", e));
                let _ = self.view.render_if_needed(
                    &self.caret,
                    self.tab_manager.current_tab().has_unsaved_changes,
                );
                let _ = Terminal::execute();
            }
        }

        if let Err(error) = Terminal::terminate() {
            eprintln!("Terminal Termination Failed: {:?}", error);
        }
    }

    fn sync_view_to_tab(&mut self) {
        let tab = self.tab_manager.current_tab();
        self.view.buffer = tab.buffer.clone();
        self.view.scroll_offset = tab.scroll_offset;
        self.view.filename = tab.filename.clone();
        self.view.filetype = tab.filetype.clone();
        self.view.needs_redraw = true;
    }

    fn sync_tab_to_view(&mut self) {
        let tab = self.tab_manager.current_tab_mut();
        tab.buffer = self.view.buffer.clone();
        tab.scroll_offset = self.view.scroll_offset;
        tab.cursor_pos = self.caret.get_position();
    }

    fn main_loop(&mut self) -> Result<(), std::io::Error> {
        loop {
            if let Some(since) = self.view.prompt_since {
                if since.elapsed() >= std::time::Duration::from_secs(2) {
                    self.view.clear_prompt();
                    let _ = self.view.render_if_needed(
                        &self.caret,
                        self.tab_manager.current_tab().has_unsaved_changes,
                    );
                    let _ = Terminal::execute();
                }
            }

            match read()? {
                Event::Key(event) => {
                    if event.kind == KeyEventKind::Press {
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
                                    self.view.clear_search();
                                }
                            }
                        }

                        if let Some(action) = self.shortcuts.resolve(&event) {
                            match action {
                                Action::SwitchTab(tab_num) => {
                                    self.sync_tab_to_view();
                                    self.tab_manager.switch_to_tab(tab_num)?;
                                    self.sync_view_to_tab();
                                    self.caret
                                        .move_to(self.tab_manager.current_tab().cursor_pos)?;
                                    self.view.render(&mut self.caret)?;
                                }

                                Action::Undo => {
                                    if let Some(operation) =
                                        self.tab_manager.current_tab_mut().edit_history.undo()
                                    {
                                        operation.edit.reverse(&mut self.view.buffer.lines);
                                        self.view.scroll_offset = operation.scroll_before;
                                        self.view.needs_redraw = true;
                                        self.view.render_if_needed(
                                            &self.caret,
                                            self.tab_manager.current_tab().has_unsaved_changes,
                                        )?;
                                        self.caret.move_to(operation.cursor_before)?;
                                        self.tab_manager.current_tab_mut().has_unsaved_changes =
                                            true;
                                    }
                                }

                                Action::Redo => {
                                    if let Some(operation) =
                                        self.tab_manager.current_tab_mut().edit_history.redo()
                                    {
                                        operation.edit.apply(&mut self.view.buffer.lines);
                                        self.view.scroll_offset = operation.scroll_after;
                                        self.view.needs_redraw = true;
                                        self.view.render_if_needed(
                                            &self.caret,
                                            self.tab_manager.current_tab().has_unsaved_changes,
                                        )?;
                                        self.caret.move_to(operation.cursor_after)?;
                                        self.tab_manager.current_tab_mut().has_unsaved_changes =
                                            true;
                                    }
                                }

                                Action::Save => self.save_file()?,
                                
                                Action::CheckUpdate => {
                                    self.check_and_install_update()?;
                                }

                                Action::New => {
                                    self.sync_tab_to_view();
                                    self.tab_manager.new_tab();
                                    self.sync_view_to_tab();
                                    self.caret.move_to(caret::Position::default())?;
                                    self.view.render(&mut self.caret)?;
                                }

                                Action::Search => self.view.search(&mut self.caret)?,
                                Action::Copy => {
                                     if let Err(e) = self.view.copy_selection() {
                                         self.view.show_prompt(crate::tui::view::PromptKind::Error, e.to_string());
                                     } else {
                                         self.view.show_prompt(crate::tui::view::PromptKind::Error, "Copied!".into());
                                     }
                                 }
                                 Action::Cut => {
                                     match self.view.cut_selection(&mut self.caret) {
                                         Ok(Some(op)) => {
                                
                                             let tab = self.tab_manager.current_tab_mut();
                                
                                             tab.edit_history.push(op);
                                
                                             tab.has_unsaved_changes = true;
                                         }
                                
                                         Err(e) => self.view.show_prompt(crate::tui::view::PromptKind::Error, e.to_string()),
                                         _ => {}
                                     }
                                 }
                                 Action::Paste => {
                                     match self.view.paste_from_clipboard(&mut self.caret) {
                                
                                         Ok(ops) => {
                                
                                             let tab = self.tab_manager.current_tab_mut();
                                
                                             if let Some(op) = ops {
                                                 tab.edit_history.push(op);
                                                 tab.has_unsaved_changes = true;
                                             }
                                         }
                                         Err(e) => self.view.show_prompt(crate::tui::view::PromptKind::Error, e.to_string()),
                                     }
                                 }
                                        

                                Action::Left => self.view.move_left(&mut self.caret)?,
                                Action::Right => self.view.move_right(&mut self.caret)?,
                                Action::Up => self.view.move_up(&mut self.caret)?,
                                Action::Down => self.view.move_down(&mut self.caret)?,
                                Action::Top => self.view.move_top(&mut self.caret)?,
                                Action::Bottom => self.view.move_bottom(&mut self.caret)?,
                                Action::MaxLeft => self.view.move_max_left(&mut self.caret)?,
                                Action::MaxRight => self.view.move_max_right(&mut self.caret)?,

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

                                Action::NextLine => {
                                    if let Some(op) = self.view.insert_newline(&mut self.caret)? {
                                        self.tab_manager.current_tab_mut().edit_history.push(op);
                                        self.tab_manager.current_tab_mut().has_unsaved_changes =
                                            true;
                                    }
                                }

                                Action::Backspace => {
                                    if let Some(op) = self.view.backspace(&mut self.caret)? {
                                        self.tab_manager.current_tab_mut().edit_history.push(op);
                                        self.tab_manager.current_tab_mut().has_unsaved_changes =
                                            true;
                                    }
                                }

                                Action::Delete => {
                                    if let Some(op) = self.view.delete_char(&mut self.caret)? {
                                        self.tab_manager.current_tab_mut().edit_history.push(op);
                                        self.tab_manager.current_tab_mut().has_unsaved_changes =
                                            true;
                                    }
                                }

                                Action::ToggleCtrlShortcuts => {
                                    self.view.toggle_ctrl_shortcuts();
                                    self.view.render(&mut self.caret)?;
                                }

                                Action::Quit => {
                                    if self.tab_manager.current_tab().has_unsaved_changes {
                                        self.view.show_prompt(
                                            crate::tui::view::PromptKind::Error,
                                            "Unsaved changes. Quit? (y/n)".to_string(),
                                        );
                                        self.view.needs_redraw = true;
                                        self.view.render_if_needed(&self.caret, true)?;
                                        Terminal::execute()?;

                                        loop {
                                            match read()? {
                                                Event::Key(ev)
                                                    if ev.kind == KeyEventKind::Press =>
                                                {
                                                    match ev.code {
                                                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                                                            self.quit_program = true;
                                                            break;
                                                        }
                                                        KeyCode::Char('n')
                                                        | KeyCode::Char('N')
                                                        | KeyCode::Esc => {
                                                            self.view.clear_prompt();
                                                            self.view.render_if_needed(
                                                                &self.caret,
                                                                true,
                                                            )?;
                                                            Terminal::execute()?;
                                                            break;
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                    } else {
                                        self.quit_program = true;
                                    }
                                }

                                Action::Print => {
                                    // Handle both Tab and regular characters
                                    match event.code {
                                        KeyCode::Tab => {
                                            // Insert 4 spaces for tab
                                            if let Some(op) =
                                                self.view.type_character(' ', &mut self.caret)?
                                            {
                                                self.tab_manager
                                                    .current_tab_mut()
                                                    .edit_history
                                                    .push(op);
                                            }
                                            if let Some(op) =
                                                self.view.type_character(' ', &mut self.caret)?
                                            {
                                                self.tab_manager
                                                    .current_tab_mut()
                                                    .edit_history
                                                    .push(op);
                                            }
                                            if let Some(op) =
                                                self.view.type_character(' ', &mut self.caret)?
                                            {
                                                self.tab_manager
                                                    .current_tab_mut()
                                                    .edit_history
                                                    .push(op);
                                            }
                                            if let Some(op) =
                                                self.view.type_character(' ', &mut self.caret)?
                                            {
                                                self.tab_manager
                                                    .current_tab_mut()
                                                    .edit_history
                                                    .push(op);
                                            }
                                            self.tab_manager
                                                .current_tab_mut()
                                                .has_unsaved_changes = true;
                                        }
                                        KeyCode::Char(character) => {
                                            if let Some(op) = self
                                                .view
                                                .type_character(character, &mut self.caret)?
                                            {
                                                self.tab_manager
                                                    .current_tab_mut()
                                                    .edit_history
                                                    .push(op);
                                                self.tab_manager
                                                    .current_tab_mut()
                                                    .has_unsaved_changes = true;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                _ => {}
                            }

                            self.view.render_if_needed(
                                &self.caret,
                                self.tab_manager.current_tab().has_unsaved_changes,
                            )?;
                            Terminal::execute()?;
                        }
                    }
                }
                Event::Mouse(mouse_event) => {
                    if let Some(action) = self.shortcuts.resolve_mouse(&mouse_event) {
                        match action {
                            Action::MouseDown(x, y) => {
                                self.view.handle_mouse_down(x, y, &mut self.caret)?
                            }
                            Action::MouseDrag(x, y) => {
                                self.view.handle_mouse_drag(x, y, &mut self.caret)?
                            }
                            Action::MouseUp(x, y) => {
                                self.view.handle_mouse_up(x, y, &mut self.caret)?
                            }
                            Action::MouseDoubleClick(x, y) => {
                                self.view.handle_double_click(x, y, &mut self.caret)?
                            }
                            Action::MouseTripleClick(x, y) => {
                                self.view.handle_triple_click(x, y, &mut self.caret)?
                            }
                            _ => {}
                        }
                        Terminal::execute()?;
                    }
                }
                Event::Resize(_, _) => self.view.handle_resize(
                    &mut self.caret,
                    self.tab_manager.current_tab().has_unsaved_changes,
                )?,
                _ => {}
            }

            if self.quit_program {
                break;
            }
        }
        Ok(())
    }

    fn save_file(&mut self) -> Result<(), std::io::Error> {
        use std::fs;

        let filename_opt = self.tab_manager.current_tab().filename.clone();

        if let Some(filename) = filename_opt {
            let last_line = self
                .view
                .buffer
                .lines
                .iter()
                .rposition(|line| !line.is_empty())
                .unwrap_or(0);
            let content_lines: Vec<String> = self
                .view
                .buffer
                .lines
                .iter()
                .take(last_line + 1)
                .cloned()
                .collect();
            let content = content_lines.join("\n");

            match fs::write(&filename, content) {
                Ok(_) => {
                    self.tab_manager.current_tab_mut().has_unsaved_changes = false;
                    let _ = self.tab_manager.save_session();
                    self.view.needs_redraw = true;
                    self.view.render_if_needed(&self.caret, false)?;
                    Terminal::execute()?;
                }
                Err(e) => return Err(e),
            }
        } else {
            self.view.show_prompt(
                crate::tui::view::PromptKind::SaveAs,
                "Save as: ".to_string(),
            );
            self.view.needs_redraw = true;
            self.view.render_if_needed(
                &self.caret,
                self.tab_manager.current_tab().has_unsaved_changes,
            )?;
            Terminal::execute()?;

            loop {
                match read()? {
                    Event::Key(event) if event.kind == KeyEventKind::Press => {
                        match event.code {
                            KeyCode::Char(c) => self.view.append_prompt_char(c),
                            KeyCode::Backspace => self.view.backspace_prompt(),
                            KeyCode::Enter => {
                                if let Some((_, _, input)) = self.view.get_prompt() {
                                    let filename = input.to_string();
                                    self.view.clear_prompt();
                                    if filename.is_empty() {
                                        break;
                                    }
                                    
                                    let raw_ext = std::path::Path::new(&filename)
                                                .extension()
                                                .map(|ext| ext.to_string_lossy().into_owned());
                                            
                                    let friendly_filetype = get_friendly_filetype(raw_ext);

                                    self.tab_manager.current_tab_mut().filename = Some(filename.clone());
                                    self.tab_manager.current_tab_mut().filetype = friendly_filetype.clone();
                                    self.view.set_filename_and_filetype(Some(filename.clone()), friendly_filetype.clone());

                                    let last_line = self
                                        .view
                                        .buffer
                                        .lines
                                        .iter()
                                        .rposition(|line| !line.is_empty())
                                        .unwrap_or(0);
                                    let content_lines: Vec<String> = self
                                        .view
                                        .buffer
                                        .lines
                                        .iter()
                                        .take(last_line + 1)
                                        .cloned()
                                        .collect();
                                    let content = content_lines.join("\n");

                                    match fs::write(&filename, content) {
                                        Ok(_) => {
                                            self.tab_manager
                                                .current_tab_mut()
                                                .has_unsaved_changes = false;
                                            let _ = self.tab_manager.save_session();
                                            self.view.needs_redraw = true;
                                            self.view.render_if_needed(&self.caret, false)?;
                                            Terminal::execute()?;
                                        }
                                        Err(e) => {
                                            self.view.show_prompt(
                                                crate::tui::view::PromptKind::Error,
                                                format!("Failed to save: {}", e),
                                            );
                                            self.view.render_if_needed(&self.caret, true)?;
                                            Terminal::execute()?;
                                            return Err(e);
                                        }
                                    }
                                }
                                break;
                            }
                            KeyCode::Esc => {
                                self.view.clear_prompt();
                                self.view.render_if_needed(
                                    &self.caret,
                                    self.tab_manager.current_tab().has_unsaved_changes,
                                )?;
                                Terminal::execute()?;
                                break;
                            }
                            _ => {}
                        }
                        self.view.render_if_needed(
                            &self.caret,
                            self.tab_manager.current_tab().has_unsaved_changes,
                        )?;
                        Terminal::execute()?;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}
