// shortcuts module handles key events and resolves them into actions
use crossterm::event::{ KeyCode, KeyEvent, KeyModifiers };
use crate::core::actions::Action;

pub struct Shortcuts;

impl Shortcuts {
    pub fn resolve(event: &KeyEvent) -> Option<Action> {
        match (event.code, event.modifiers) {
            // next line
            (KeyCode::Enter, _) => Some(Action::NextLine),
            // set (shortcut) to exit the program
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                Some(Action::Quit)
            },
            // print everything else
            _ => Some(Action::Print),
        }
    }
}