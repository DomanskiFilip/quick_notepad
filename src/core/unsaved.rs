// unsaved module - the "save before closing?" questions, shared by tui and gui
// used when quitting and when a new tab drops the last tab at the tab limit
use crate::core::tabs::TabManager;

#[derive(Clone, Copy, PartialEq)]
pub enum CloseReason {
    Quit,
    DropTab,
}

#[derive(Clone, Copy, PartialEq)]
pub enum UnsavedStep {
    AskSave,        // save / don't save / cancel
    ConfirmDiscard, // really close without saving? yes / go back
}

pub enum UnsavedChoice {
    Save,
    DontSave,
    ConfirmDiscard,
    GoBack,
    Cancel,
}

pub enum UnsavedOutcome {
    Ask,     // keep asking, step has changed
    Save,    // save the tabs, then close
    Discard, // close without saving
    Cancel,
}

pub struct UnsavedPrompt {
    pub reason: CloseReason,
    pub tabs: Vec<usize>,
    pub step: UnsavedStep,
    names: Vec<String>,
}

impl UnsavedPrompt {
    // None when no tab has unsaved changes - quit right away
    pub fn for_quit(tab_manager: &TabManager) -> Option<Self> {
        Self::new(CloseReason::Quit, tab_manager.dirty_tabs(), tab_manager)
    }

    // None when the new tab doesn't drop a tab with unsaved changes
    pub fn for_new_tab(tab_manager: &TabManager) -> Option<Self> {
        let dropped = tab_manager.tab_dropped_by_new_tab()?;
        let tabs = if tab_manager.tabs[dropped].has_unsaved_changes {
            vec![dropped]
        } else {
            Vec::new()
        };
        Self::new(CloseReason::DropTab, tabs, tab_manager)
    }

    fn new(reason: CloseReason, tabs: Vec<usize>, tab_manager: &TabManager) -> Option<Self> {
        if tabs.is_empty() {
            return None;
        }
        let names = tabs
            .iter()
            .map(|&i| tab_manager.tabs[i].display_name())
            .collect();
        Some(Self {
            reason,
            tabs,
            step: UnsavedStep::AskSave,
            names,
        })
    }

    pub fn names(&self) -> &[String] {
        &self.names
    }

    pub fn title(&self) -> &'static str {
        match (self.step, self.reason) {
            (UnsavedStep::AskSave, CloseReason::Quit) => "Unsaved changes",
            (UnsavedStep::AskSave, CloseReason::DropTab) => "Tab limit reached",
            (UnsavedStep::ConfirmDiscard, CloseReason::Quit) => "Quit without saving?",
            (UnsavedStep::ConfirmDiscard, CloseReason::DropTab) => "Close tab without saving?",
        }
    }

    pub fn message(&self) -> String {
        let what = if self.names.len() == 1 {
            self.names[0].clone()
        } else {
            format!("{} tabs", self.names.len())
        };
        match (self.step, self.reason) {
            (UnsavedStep::AskSave, CloseReason::Quit) => {
                format!("Unsaved changes in {}. Save before quitting?", what)
            }
            (UnsavedStep::AskSave, CloseReason::DropTab) => {
                format!("Tab limit reached, {} will be closed. Save it first?", what)
            }
            (UnsavedStep::ConfirmDiscard, _) => {
                format!("Changes to {} will be lost.", what)
            }
        }
    }

    // Labels for the gui buttons
    pub fn save_label(&self) -> &'static str {
        match self.reason {
            CloseReason::Quit => "Save & Quit",
            CloseReason::DropTab => "Save & Close Tab",
        }
    }

    // Label for the button that discards changes
    pub fn discard_label(&self) -> &'static str {
        match self.reason {
            CloseReason::Quit => "Quit Without Saving",
            CloseReason::DropTab => "Close Without Saving",
        }
    }

    pub fn choose(&mut self, choice: UnsavedChoice) -> UnsavedOutcome {
        match (self.step, choice) {
            (_, UnsavedChoice::Cancel) => UnsavedOutcome::Cancel,
            (UnsavedStep::AskSave, UnsavedChoice::Save) => UnsavedOutcome::Save,
            (UnsavedStep::AskSave, UnsavedChoice::DontSave) => {
                // ask a second time before anything is thrown away
                self.step = UnsavedStep::ConfirmDiscard;
                UnsavedOutcome::Ask
            }
            (UnsavedStep::ConfirmDiscard, UnsavedChoice::ConfirmDiscard) => UnsavedOutcome::Discard,
            (UnsavedStep::ConfirmDiscard, UnsavedChoice::GoBack) => {
                self.step = UnsavedStep::AskSave;
                UnsavedOutcome::Ask
            }
            _ => UnsavedOutcome::Ask,
        }
    }
}
