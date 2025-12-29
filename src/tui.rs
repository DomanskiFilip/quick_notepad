// editor module handles the editor's state and logic
mod view;
mod main_error_wrapper;
mod terminal;
mod caret;

use crate::core::actions::Action;
use crate::core::shortcuts::Shortcuts;
use crossterm::event::{ Event, KeyCode, KeyEventKind, read };
use main_error_wrapper::MainErrorWrapper;
use view::View;
use terminal::Terminal;
use caret::Caret;

pub struct TerminalEditor {
    view: View,
    caret: Caret,
    quit_program: bool,
}

impl TerminalEditor {
    pub fn default() -> Self {
        Self {
            view: View::default(),
            caret: Caret::new(),
            quit_program: false,
        }
    }
    
    pub fn run(&mut self) {
        if let Err(error) = Terminal::initialize(&self.view, &mut self.caret) {
            eprintln!("Terminal Initialisation Failed: {:?}", error); 
        }
        MainErrorWrapper::handle_error(self.main_loop());
        if let Err(error) = Terminal::terminate() {
            eprintln!("Terminal Termination Failed: {:?}", error); 
        }
    }

    fn main_loop(&mut self) -> Result<(), std::io::Error> {
        loop {
            match read()? {
                Event::Key(event) => {
                    if event.kind == KeyEventKind::Press {
                        if let Some(action) = Shortcuts::resolve(&event) {
                            match action {
                                Action::Left => self.view.move_left(&mut self.caret)?,
                                Action::Right => self.view.move_right(&mut self.caret)?,
                                Action::Up => self.view.move_up(&mut self.caret)?,
                                Action::Down => self.view.move_down(&mut self.caret)?,
                                Action::Top => self.view.move_top(&mut self.caret)?,
                                Action::Bottom => self.view.move_bottom(&mut self.caret)?,
                                Action::MaxLeft => self.view.move_max_left(&mut self.caret)?,
                                Action::MaxRight => self.view.move_max_right(&mut self.caret)?,
                                Action::NextLine => self.view.insert_newline(&mut self.caret)?,
                                Action::Quit => self.quit_program = true,
                                Action::Print => {
                                    if let KeyCode::Char(character) = event.code {
                                        self.view.type_character(character, &mut self.caret)?;
                                    }
                                }
                            }
                            Terminal::execute()?;
                        }
                    }
                },
                Event::Resize(_, _) => self.view.handle_resize(&mut self.caret)?,
                _ => {}
            }

            if self.quit_program { break; }
        }
        Ok(())
    }
}