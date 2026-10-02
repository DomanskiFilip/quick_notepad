// src/gui/app.rs
use super::{editor::EditorPanel, state::EditorState, themes};
use crate::core::actions::Action;
use crate::core::unsaved::{
    CloseReason, UnsavedChoice, UnsavedOutcome, UnsavedPrompt, UnsavedStep,
};
use crate::core::updater::UpdateInfo;
use egui::{Color32, Context, ViewportCommand};
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub struct QuickNotepadApp {
    state: EditorState,
    show_shortcuts: bool,
    show_save_dialog: bool,
    save_filename: String,
    save_error: Option<String>,
    focus_save_field: bool,
    show_update_dialog: bool,
    update_info: Option<UpdateInfo>,
    update_check: Option<mpsc::Receiver<Result<UpdateInfo, String>>>,
    // "save before closing?" dialog, used for quitting and for the tab dropped at the tab limit
    unsaved_prompt: Option<UnsavedPrompt>,
    // Save was chosen but a tab still needs a name, continues after the save dialog
    saving_before_close: Option<UnsavedPrompt>,
    allow_close: bool,
    status_message: Option<(String, bool, Instant)>, // (text, is_error, shown at)
}

impl QuickNotepadApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, file_path: Option<String>) -> Self {
        Self {
            state: EditorState::new(file_path),
            show_shortcuts: false,
            show_save_dialog: false,
            save_filename: String::new(),
            save_error: None,
            focus_save_field: false,
            show_update_dialog: false,
            update_info: None,
            update_check: None,
            unsaved_prompt: None,
            saving_before_close: None,
            allow_close: false,
            status_message: None,
        }
    }

    // While a dialog is open the editor and shortcuts ignore input
    fn dialog_open(&self) -> bool {
        self.show_save_dialog || self.show_update_dialog || self.unsaved_prompt.is_some()
    }

    fn set_status(&mut self, text: String, is_error: bool) {
        self.status_message = Some((text, is_error, Instant::now()));
    }

    fn menu_bar(&mut self, ctx: &Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("📄 New (Ctrl+N)").clicked() {
                        self.handle_action(ctx, Action::New);
                        ui.close();
                    }

                    if ui.button("💾 Save (Ctrl+S / Ctrl+O)").clicked() {
                        self.handle_action(ctx, Action::Save);
                        ui.close();
                    }

                    if ui.button("💾 Save As...").clicked() {
                        self.open_save_dialog();
                        ui.close();
                    }

                    ui.separator();

                    if ui.button("❌ Quit (Ctrl+Q)").clicked() {
                        self.handle_action(ctx, Action::Quit);
                        ui.close();
                    }
                });

                ui.menu_button("Edit", |ui| {
                    let can_undo = self.state.can_undo();
                    let can_redo = self.state.can_redo();
                    let has_selection = self.state.has_selection();

                    if ui
                        .add_enabled(can_undo, egui::Button::new("↶ Undo (Ctrl+Z)"))
                        .clicked()
                    {
                        self.handle_action(ctx, Action::Undo);
                        ui.close();
                    }

                    if ui
                        .add_enabled(can_redo, egui::Button::new("↷ Redo (Ctrl+Y)"))
                        .clicked()
                    {
                        self.handle_action(ctx, Action::Redo);
                        ui.close();
                    }

                    ui.separator();

                    if ui
                        .add_enabled(has_selection, egui::Button::new("📋 Copy (Ctrl+C)"))
                        .clicked()
                    {
                        self.handle_action(ctx, Action::Copy);
                        ui.close();
                    }

                    if ui
                        .add_enabled(has_selection, egui::Button::new("✂ Cut (Ctrl+X)"))
                        .clicked()
                    {
                        self.handle_action(ctx, Action::Cut);
                        ui.close();
                    }

                    if ui.button("📄 Paste (Ctrl+V)").clicked() {
                        self.handle_action(ctx, Action::Paste);
                        ui.close();
                    }

                    ui.separator();

                    if ui.button("🔍 Find (Ctrl+F)").clicked() {
                        self.handle_action(ctx, Action::Search);
                        ui.close();
                    }

                    if ui.button("🔤 Select All (Ctrl+A)").clicked() {
                        self.handle_action(ctx, Action::SelectAll);
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
                        self.handle_action(ctx, Action::CheckUpdate);
                        ui.close();
                    }
                });

                ui.menu_button("Tabs", |ui| {
                    let active = self.state.tab_manager.active_tab_index;
                    for i in 0..self.state.tab_manager.tabs.len() {
                        let tab = &self.state.tab_manager.tabs[i];
                        let dirty = if tab.has_unsaved_changes { "*" } else { "" };
                        let tab_text = format!("{}{} (Ctrl+{})", tab.display_name(), dirty, i + 1);
                        if ui.selectable_label(i == active, tab_text).clicked() {
                            self.handle_action(ctx, Action::SwitchTab(i + 1));
                            ui.close();
                        }
                    }
                });
            });
        });
    }

    fn tab_bar(&mut self, ctx: &Context) {
        egui::TopBottomPanel::top("tab_bar").show(ctx, |ui| {
            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    let active = self.state.tab_manager.active_tab_index;
                    let mut clicked_tab = None;

                    for i in 0..self.state.tab_manager.tabs.len() {
                        let tab = &self.state.tab_manager.tabs[i];
                        let dirty = if tab.has_unsaved_changes { "*" } else { "" };
                        let path = tab.filepath.as_deref().unwrap_or("Not saved yet");
                        if ui
                            .selectable_label(i == active, format!("{}{}", tab.display_name(), dirty))
                            .on_hover_text(path)
                            .clicked()
                        {
                            clicked_tab = Some(i + 1);
                        }
                    }

                    if ui.button("+").on_hover_text("New tab (Ctrl+N)").clicked() {
                        self.handle_action(ctx, Action::New);
                    }

                    if let Some(tab_number) = clicked_tab {
                        self.handle_action(ctx, Action::SwitchTab(tab_number));
                    }
                });
            });
        });
    }

    fn status_bar(&mut self, ctx: &Context) {
        // Status messages disappear after a few seconds
        if let Some((_, _, shown_at)) = &self.status_message {
            let timeout = Duration::from_secs(4);
            if shown_at.elapsed() >= timeout {
                self.status_message = None;
            } else {
                ctx.request_repaint_after(timeout - shown_at.elapsed());
            }
        }

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

                ui.separator();
                let version = env!("CARGO_PKG_VERSION");
                ui.label(format!("v{}", version));

                if self.update_check.is_some() {
                    ui.separator();
                    ui.spinner();
                    ui.label("Checking for updates...");
                } else if let Some((text, is_error, _)) = &self.status_message {
                    ui.separator();
                    let color = if *is_error {
                        Color32::from_rgb(230, 110, 90)
                    } else {
                        Color32::from_rgb(140, 200, 120)
                    };
                    ui.colored_label(color, text);
                }

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
        if self.dialog_open() {
            return;
        }

        let mut triggered = Vec::new();
        ctx.input_mut(|i| {
            // Map egui shortcuts to our Action enum
            let actions = vec![
                (
                    egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::S),
                    Action::Save,
                ),
                (
                    egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::O),
                    Action::Save,
                ),
                (
                    egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::N),
                    Action::New,
                ),
                (
                    egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Q),
                    Action::Quit,
                ),
                (
                    egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Z),
                    Action::Undo,
                ),
                (
                    egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Y),
                    Action::Redo,
                ),
                (
                    egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::C),
                    Action::Copy,
                ),
                (
                    egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::X),
                    Action::Cut,
                ),
                (
                    egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::V),
                    Action::Paste,
                ),
                (
                    egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::F),
                    Action::Search,
                ),
                (
                    egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::A),
                    Action::SelectAll,
                ),
                (
                    egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::U),
                    Action::CheckUpdate,
                ),
            ];

            for (shortcut, action) in actions {
                if i.consume_shortcut(&shortcut) {
                    triggered.push(action);
                }
            }

            // Tab switching - Ctrl+1-9
            for num in 1..=9 {
                let key = match num {
                    1 => egui::Key::Num1,
                    2 => egui::Key::Num2,
                    3 => egui::Key::Num3,
                    4 => egui::Key::Num4,
                    5 => egui::Key::Num5,
                    6 => egui::Key::Num6,
                    7 => egui::Key::Num7,
                    8 => egui::Key::Num8,
                    9 => egui::Key::Num9,
                    _ => egui::Key::Num0,
                };
                if i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::CTRL, key)) {
                    triggered.push(Action::SwitchTab(num));
                }
            }
        });

        for action in triggered {
            self.handle_action(ctx, action);
        }
    }

    // Centralized action handler - uses the Action enum from core
    fn handle_action(&mut self, ctx: &Context, action: Action) {
        match action {
            Action::Save => match self.state.save() {
                Ok(true) => {
                    let name = self.state.current_filename().unwrap_or_default();
                    self.set_status(format!("Saved {}", name), false);
                }
                Ok(false) => self.open_save_dialog(),
                Err(e) => self.set_status(format!("Failed to save: {}", e), true),
            },
            Action::New => {
                // At the tab limit the last tab is dropped, ask first if it has unsaved changes
                match UnsavedPrompt::for_new_tab(&self.state.tab_manager) {
                    Some(prompt) => self.unsaved_prompt = Some(prompt),
                    None => self.state.new_tab(),
                }
            }
            Action::Quit => match UnsavedPrompt::for_quit(&self.state.tab_manager) {
                Some(prompt) => self.unsaved_prompt = Some(prompt),
                None => self.finish_close(ctx, CloseReason::Quit),
            },
            Action::Undo => {
                self.state.undo();
            }
            Action::Redo => {
                self.state.redo();
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
                // Start with the selected text if it's on one line
                if let Some(text) = self.state.selected_text().filter(|t| !t.contains('\n')) {
                    self.state.search_query = text;
                }
                self.state.search_active = true;
                self.state.search_focus_requested = true;
                self.state.perform_search();
            }
            Action::SelectAll => {
                self.state.select_all();
            }
            Action::SwitchTab(num) => {
                self.state.switch_to_tab(num);
            }
            Action::CheckUpdate => {
                self.check_for_updates_gui(ctx);
            }
            _ => {}
        }
    }

    fn open_save_dialog(&mut self) {
        self.show_save_dialog = true;
        self.focus_save_field = true;
        self.save_error = None;
        self.save_filename = self
            .state
            .tab_manager
            .current_tab()
            .filepath
            .clone()
            .unwrap_or_default();
    }

    fn show_save_dialog(&mut self, ctx: &Context) {
        let mut close_dialog = false;
        let mut save = false;

        let modal = egui::Modal::new(egui::Id::new("save_dialog")).show(ctx, |ui| {
            ui.set_width(400.0);
            ui.heading("Save As");

            ui.horizontal(|ui| {
                ui.label("Filename:");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.save_filename)
                        .desired_width(f32::INFINITY),
                );

                if self.focus_save_field {
                    response.request_focus();
                    self.focus_save_field = false;
                }

                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    save = true;
                }
            });

            if let Some(error) = &self.save_error {
                ui.colored_label(Color32::from_rgb(230, 110, 90), error);
            }

            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    save = true;
                }

                if ui.button("Cancel").clicked() {
                    close_dialog = true;
                }
            });
        });

        // Escape or clicking outside the dialog
        if modal.should_close() {
            close_dialog = true;
        }

        if save && !close_dialog {
            match self.state.save_as(&self.save_filename) {
                Ok(_) => {
                    self.show_save_dialog = false;
                    let name = self.state.current_filename().unwrap_or_default();
                    self.set_status(format!("Saved {}", name), false);
                    // Continue quitting / closing the tab if that's why we were saving
                    if let Some(prompt) = self.saving_before_close.take() {
                        self.save_before_close(ctx, prompt);
                    }
                }
                Err(e) => {
                    self.save_error = Some(format!("Could not save: {}", e));
                    self.focus_save_field = true;
                }
            }
        }

        if close_dialog {
            self.show_save_dialog = false;
            self.saving_before_close = None;
        }
    }

    fn show_unsaved_dialog(&mut self, ctx: &Context) {
        let Some(prompt) = &mut self.unsaved_prompt else {
            return;
        };
        let mut choice = None;

        let modal = egui::Modal::new(egui::Id::new("unsaved_dialog")).show(ctx, |ui| {
            ui.set_width(380.0);
            ui.heading(prompt.title());
            ui.label(prompt.message());
            if prompt.names().len() > 1 {
                for name in prompt.names() {
                    ui.label(format!("  • {}", name));
                }
            }

            ui.horizontal(|ui| match prompt.step {
                UnsavedStep::AskSave => {
                    if ui.button(format!("💾 {}", prompt.save_label())).clicked() {
                        choice = Some(UnsavedChoice::Save);
                    }
                    if ui.button("Don't Save").clicked() {
                        choice = Some(UnsavedChoice::DontSave);
                    }
                    if ui.button("Cancel").clicked() {
                        choice = Some(UnsavedChoice::Cancel);
                    }
                }
                UnsavedStep::ConfirmDiscard => {
                    let discard = egui::Button::new(
                        egui::RichText::new(prompt.discard_label()).color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(160, 50, 40));
                    if ui.add(discard).clicked() {
                        choice = Some(UnsavedChoice::ConfirmDiscard);
                    }
                    if ui.button("Go Back").clicked() {
                        choice = Some(UnsavedChoice::GoBack);
                    }
                }
            });
        });

        // Escape or clicking outside the dialog
        if modal.should_close() {
            choice = Some(UnsavedChoice::Cancel);
        }

        let Some(choice) = choice else {
            return;
        };
        match prompt.choose(choice) {
            UnsavedOutcome::Ask => {}
            UnsavedOutcome::Save => {
                if let Some(prompt) = self.unsaved_prompt.take() {
                    self.save_before_close(ctx, prompt);
                }
            }
            UnsavedOutcome::Discard => {
                if let Some(prompt) = self.unsaved_prompt.take() {
                    self.finish_close(ctx, prompt.reason);
                }
            }
            UnsavedOutcome::Cancel => self.unsaved_prompt = None,
        }
    }

    // Save the prompt's tabs, a tab without a file opens the save dialog first
    fn save_before_close(&mut self, ctx: &Context, prompt: UnsavedPrompt) {
        match self.state.tab_manager.save_tabs(&prompt.tabs) {
            Ok(None) => self.finish_close(ctx, prompt.reason),
            Ok(Some(index)) => {
                self.state.switch_to_tab(index + 1);
                self.open_save_dialog();
                self.saving_before_close = Some(prompt);
            }
            Err(e) => self.set_status(format!("Failed to save {}", e), true),
        }
    }

    fn finish_close(&mut self, ctx: &Context, reason: CloseReason) {
        match reason {
            CloseReason::Quit => {
                self.allow_close = true;
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
            CloseReason::DropTab => self.state.new_tab(),
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
            let modal = egui::Modal::new(egui::Id::new("update_dialog")).show(ctx, |ui| {
                ui.set_width(400.0);
                ui.heading(format!(
                    "Version {} → {}",
                    info.current_version, info.latest_version
                ));
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

            if modal.should_close() {
                close_dialog = true;
            }
        }

        if perform_update {
            use crate::core::updater::Updater;
            let updater = Updater::new();

            match updater.perform_update() {
                Ok(_) => {
                    self.set_status(
                        "Update successful! Restart to use the new version.".to_string(),
                        false,
                    );
                    self.handle_action(ctx, Action::Quit);
                }
                Err(e) => {
                    self.set_status(format!("Update failed: {}", e), true);
                }
            }
        }

        if close_dialog {
            self.show_update_dialog = false;
            self.update_info = None;
        }
    }

    // Check in the background so the window doesn't freeze
    fn check_for_updates_gui(&mut self, ctx: &Context) {
        use crate::core::updater::Updater;

        if self.update_check.is_some() {
            return;
        }

        let (sender, receiver) = mpsc::channel();
        self.update_check = Some(receiver);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = Updater::new()
                .check_for_updates()
                .map_err(|e| e.to_string());
            let _ = sender.send(result);
            ctx.request_repaint();
        });
    }

    fn poll_update_check(&mut self) {
        let Some(receiver) = &self.update_check else {
            return;
        };
        let Ok(result) = receiver.try_recv() else {
            return;
        };
        self.update_check = None;

        match result {
            Ok(info) => {
                if info.update_available {
                    self.show_update_dialog = true;
                    self.update_info = Some(info);
                } else {
                    self.set_status(
                        format!(
                            "No updates available. Running version {}",
                            info.current_version
                        ),
                        false,
                    );
                }
            }
            Err(e) => {
                self.set_status(format!("Failed to check for updates: {}", e), true);
            }
        }
    }
}

impl eframe::App for QuickNotepadApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        themes::apply_theme(ctx);
        self.poll_update_check();

        // Window close button goes through the same unsaved changes questions as Ctrl+Q
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close {
            if let Some(prompt) = UnsavedPrompt::for_quit(&self.state.tab_manager) {
                ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                if self.unsaved_prompt.is_none() && self.saving_before_close.is_none() {
                    self.unsaved_prompt = Some(prompt);
                }
            }
        }

        self.handle_shortcuts(ctx);
        self.menu_bar(ctx);
        self.tab_bar(ctx);
        self.status_bar(ctx);

        let accepts_input = !self.dialog_open();
        egui::CentralPanel::default().show(ctx, |ui| {
            EditorPanel::new(&mut self.state, accepts_input).show(ui);
        });

        if self.show_save_dialog {
            self.show_save_dialog(ctx);
        }

        if self.unsaved_prompt.is_some() {
            self.show_unsaved_dialog(ctx);
        }

        if self.show_shortcuts {
            self.show_shortcuts_window(ctx);
        }

        if self.show_update_dialog {
            self.show_update_dialog(ctx);
        }
    }
}
