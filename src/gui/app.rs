// src/gui/app.rs
use super::{editor::EditorPanel, state::EditorState, themes};
use crate::core::actions::Action;
use crate::core::updater::UpdateInfo;
use egui::{Context, ViewportCommand};

pub struct QuickNotepadApp {
    state: EditorState,
    show_shortcuts: bool,
    show_save_dialog: bool,
    save_filename: String,
    dialog_has_focus: bool,
    show_update_dialog: bool,
    update_info: Option<UpdateInfo>,
}

impl QuickNotepadApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, file_path: Option<String>) -> Self {
        Self {
            state: EditorState::new(file_path),
            show_shortcuts: false,
            show_save_dialog: false,
            save_filename: String::new(),
            dialog_has_focus: false,
            show_update_dialog: false,
            update_info: None,
        }
    }

    fn menu_bar(&mut self, ctx: &Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("📄 New (Ctrl+N)").clicked() {
                        self.handle_action(Action::New);
                        ui.close();
                    }

                    if ui.button("💾 Save (Ctrl+S)").clicked() {
                        self.handle_action(Action::Save);
                        ui.close();
                    }

                    if ui.button("💾 Save As...").clicked() {
                        self.show_save_dialog = true;
                        ui.close();
                    }

                    ui.separator();

                    if ui.button("❌ Quit (Ctrl+Q)").clicked() {
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                        ui.close();
                    }
                });

                ui.menu_button("Edit", |ui| {
                    if ui.button("↶ Undo (Ctrl+Z)").clicked() {
                        self.handle_action(Action::Undo);
                        ui.close();
                    }

                    if ui.button("↷ Redo (Ctrl+Y)").clicked() {
                        self.handle_action(Action::Redo);
                        ui.close();
                    }

                    ui.separator();

                    if ui.button("📋 Copy (Ctrl+C)").clicked() {
                        self.handle_action(Action::Copy);
                        ui.close();
                    }

                    if ui.button("✂ Cut (Ctrl+X)").clicked() {
                        self.handle_action(Action::Cut);
                        ui.close();
                    }

                    if ui.button("📄 Paste (Ctrl+V)").clicked() {
                        self.handle_action(Action::Paste);
                        ui.close();
                    }

                    ui.separator();

                    if ui.button("🔍 Find (Ctrl+F)").clicked() {
                        self.handle_action(Action::Search);
                        ui.close();
                    }

                    if ui.button("🔤 Select All (Ctrl+A)").clicked() {
                        self.handle_action(Action::SelectAll);
                        ui.close();
                    }
                });

                ui.menu_button("View", |ui| {
                    if ui.button("⌨ Shortcuts").clicked() {
                        self.show_shortcuts = !self.show_shortcuts;
                        ui.close();
                    }
                });
                
                ui.menu_button("Help", |ui| {
                    if ui.button("🔄 Check for Updates (Ctrl+U)").clicked() {
                        self.handle_action(Action::CheckUpdate);
                        ui.close();
                    }
                });

                ui.menu_button("Tabs", |ui| {
                    for i in 1..=9 {
                        let tab_text = format!("Tab {} (Ctrl+{})", i, i);
                        if ui.button(tab_text).clicked() {
                            self.handle_action(Action::SwitchTab(i));
                            ui.close();
                        }
                    }
                });
            });
        });
    }

    fn status_bar(&mut self, ctx: &Context) {
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let filename = self.state.current_filename().unwrap_or("[No Name]");
                let dirty = if self.state.has_unsaved_changes() {
                    "*"
                } else {
                    ""
                };
                ui.label(format!("{}{}", filename, dirty));

                ui.separator();

                ui.label(format!(
                    "Ln {}, Col {}",
                    self.state.cursor_pos.line + 1,
                    self.state.cursor_pos.column + 1
                ));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("© Filip Domanski");
                    ui.separator();

                    let line_count = self
                        .state
                        .current_buffer()
                        .lines
                        .iter()
                        .rposition(|l| !l.is_empty())
                        .map(|i| i + 1)
                        .unwrap_or(1);
                    ui.label(format!("Lines: {}", line_count));
                });
            });
        });
    }

    fn handle_shortcuts(&mut self, ctx: &Context) {
        if self.dialog_has_focus {
            return;
        }

        ctx.input_mut(|i| {
            // Map egui shortcuts to our Action enum
            let actions = vec![
                (egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::S), Action::Save),
                (egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::N), Action::New),
                (egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Q), Action::Quit),
                (egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Z), Action::Undo),
                (egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Y), Action::Redo),
                (egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::C), Action::Copy),
                (egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::X), Action::Cut),
                (egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::V), Action::Paste),
                (egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::F), Action::Search),
                (egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::A), Action::SelectAll),
                (egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::U), Action::CheckUpdate),
            ];

            for (shortcut, action) in actions {
                if i.consume_shortcut(&shortcut) {
                    self.handle_action(action);
                }
            }

            // Tab switching - Ctrl+1-9
            for num in 1..=9 {
                let key = match num {
                    1 => egui::Key::Num1, 2 => egui::Key::Num2, 3 => egui::Key::Num3,
                    4 => egui::Key::Num4, 5 => egui::Key::Num5, 6 => egui::Key::Num6,
                    7 => egui::Key::Num7, 8 => egui::Key::Num8, 9 => egui::Key::Num9,
                    _ => egui::Key::Num0,
                };
                if i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::CTRL, key)) {
                    self.handle_action(Action::SwitchTab(num));
                }
            }
        });
    }

    // Centralized action handler - uses the Action enum from core
    fn handle_action(&mut self, action: Action) {
        match action {
            Action::Save => {
                if self.state.current_filename().is_some() {
                    let _ = self.state.save();
                } else {
                    self.show_save_dialog = true;
                    self.dialog_has_focus = true;
                }
            }
            Action::New => {
                self.state.tab_manager.new_tab();
            }
            Action::Quit => {
                // Handle in update loop
            }
            Action::Undo => {
                if let Some(op) = self.state.current_edit_history().undo() {
                    op.edit.reverse(&mut self.state.current_buffer_mut().lines);
                }
            }
            Action::Redo => {
                if let Some(op) = self.state.current_edit_history().redo() {
                    op.edit.apply(&mut self.state.current_buffer_mut().lines);
                }
            }
            Action::Copy => {
                self.state.copy_selection();
            }
            Action::Cut => {
                self.state.cut_selection();
            }
            Action::Paste => {
                self.state.paste_from_clipboard();
            }
            Action::Search => {
                self.state.search_active = true;
                self.dialog_has_focus = true;
            }
            Action::SelectAll => {
                self.state.select_all();
            }
            Action::SwitchTab(num) => {
                let _ = self.state.tab_manager.switch_to_tab(num);
            }
            Action::CheckUpdate => {
                self.check_for_updates_gui();
            }
            _ => {}
        }
    }

    fn show_save_dialog(&mut self, ctx: &Context) {
        let mut close_dialog = false;

        egui::Window::new("Save As")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Filename:");
                    let response = ui.text_edit_singleline(&mut self.save_filename);

                    if self.dialog_has_focus {
                        response.request_focus();
                    }

                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        if !self.save_filename.is_empty() {
                            let _ = self.state.save_as(&self.save_filename);
                            close_dialog = true;
                        }
                    }
                });

                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        if !self.save_filename.is_empty() {
                            let _ = self.state.save_as(&self.save_filename);
                            close_dialog = true;
                        }
                    }

                    if ui.button("Cancel").clicked() {
                        close_dialog = true;
                    }
                });

                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    close_dialog = true;
                }
            });

        if close_dialog {
            self.show_save_dialog = false;
            self.save_filename.clear();
            self.dialog_has_focus = false;
        }
    }

    fn show_shortcuts_window(&mut self, ctx: &Context) {
        use crate::core::shortcuts::Shortcuts;
        
        egui::Window::new("Keyboard Shortcuts")
            .collapsible(true)
            .resizable(true)
            .show(ctx, |ui| {
                egui::Grid::new("shortcuts_grid")
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Action");
                        ui.label("Shortcut");
                        ui.end_row();

                        // Get shortcuts from core Shortcuts module
                        let shortcuts = Shortcuts::get_ctrl_shortcuts();
                        
                        for (shortcut, description) in shortcuts {
                            ui.label(description);
                            ui.label(shortcut);
                            ui.end_row();
                        }
                    });

                if ui.button("Close").clicked() {
                    self.show_shortcuts = false;
                }
            });
    }
    
    fn show_update_dialog(&mut self, ctx: &Context) {
        let mut close_dialog = false;
        let mut perform_update = false;
        
        if let Some(info) = &self.update_info {
            egui::Window::new("Update Available")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.heading(format!("Version {} → {}", info.current_version, info.latest_version));
                    ui.separator();
                    
                    ui.label("Release Notes:");
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            ui.label(&info.release_notes);
                        });
                    
                    ui.separator();
                    
                    ui.horizontal(|ui| {
                        if ui.button("Update Now").clicked() {
                            perform_update = true;
                            close_dialog = true;
                        }
                        
                        if ui.button("Later").clicked() {
                            close_dialog = true;
                        }
                    });
                });
        }
        
        if perform_update {
            // Perform update in background
            use crate::core::updater::Updater;
            let updater = Updater::new();
            
            match updater.perform_update() {
                Ok(_) => {
                    eprintln!("Update successful! Please restart the application.");
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
                Err(e) => {
                    eprintln!("Update failed: {}", e);
                }
            }
        }
        
        if close_dialog {
            self.show_update_dialog = false;
            self.update_info = None;
        }
    }

    fn check_for_updates_gui(&mut self) {
        use crate::core::updater::Updater;
        
        let updater = Updater::new();
        
        match updater.check_for_updates() {
            Ok(info) => {
                if info.update_available {
                    self.show_update_dialog = true;
                    self.update_info = Some(info);
                } else {
                    eprintln!("No updates available. Running version {}", info.current_version);
                }
            }
            Err(e) => {
                eprintln!("Failed to check for updates: {}", e);
            }
        }
    }
}

impl eframe::App for QuickNotepadApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        themes::apply_theme(ctx);
        self.handle_shortcuts(ctx);
        self.menu_bar(ctx);
        self.status_bar(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            EditorPanel::new(&mut self.state, !self.dialog_has_focus).show(ui);
        });

        if self.show_save_dialog {
            self.show_save_dialog(ctx);
        }

        if self.show_shortcuts {
            self.show_shortcuts_window(ctx);
        }
        
        if self.show_update_dialog {
            self.show_update_dialog(ctx);
        }
    }
}