// editor module handles the editor's state and logic
mod drawing;
mod main_error_wrapper;
mod terminal;

use crate::core::actions::Action;
use crate::core::shortcuts::Shortcuts;
use crossterm::event::{ Event::Key, KeyCode, KeyEventKind, read };
use drawing::Draw;
use main_error_wrapper::MainErrorWrapper;
use terminal::Terminal;

pub struct TerminalEditor {
    quit_program: bool,
}

impl TerminalEditor {
    pub fn default() -> Self {
        Self {
            quit_program: false,
        }
    }

    pub fn run(&mut self) {
        Terminal::initialize();
        // runs main program loop with error wrapper
        MainErrorWrapper::handle_error(self.main_loop());
        Terminal::terminate();
    }

    // handle action logic
    fn main_loop(&mut self) -> Result<(), std::io::Error> {
        // main program loop
        loop {
            if let Key(event) = read()? {
                if event.kind == KeyEventKind::Press {
                    // Shortcuts resolves key events into actions
                    if let Some(action) = Shortcuts::resolve(&event) {
                        // logic to handle actions
                        match action {
                            Action::NextLine => {
                                let _ = Terminal::next_line();
                            }
                            Action::Quit => {
                                self.quit_program = true;
                            }
                            Action::Print => {
                                if let KeyCode::Char(character) = event.code {
                                    Draw::print_character(&character.to_string())
                                }
                            }
                        }
                    }
                }
            }

            // end the program
            if self.quit_program {
                break;
            }
        }

        Ok(())
    }
}
