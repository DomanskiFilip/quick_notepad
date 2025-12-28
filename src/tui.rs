// editor module handles the editor's state and logic
mod drawing;
mod main_error_wrapper;
mod terminal;
mod caret;

use crate::core::actions::Action;
use crate::core::shortcuts::Shortcuts;
use crossterm::{
    event::{ Event, KeyCode, KeyEventKind, read },
    cursor::position,
};
use main_error_wrapper::MainErrorWrapper;
use drawing::Draw;
use terminal::Terminal;
use caret::Caret;


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
        // initialise tui
        if let Err(error) = Terminal::initialize() {
            eprintln!("Terminal Initialisation Failed: {:?}", error); 
        }
        // runs main program loop with error wrapper
        MainErrorWrapper::handle_error(self.main_loop());
        // terminate tui
        if let Err(error) = Terminal::terminate() {
            eprintln!("Terminal Termination Failed: {:?}", error); 
        }
    }

    // main program loop
    fn main_loop(&mut self) -> Result<(), std::io::Error> {
        loop {
            if let Event::Key(event) = read()? {
                if event.kind == KeyEventKind::Press {
                    // Shortcuts resolves key events into actions
                    if let Some(action) = Shortcuts::resolve(&event) {
                        // logic to handle actions
                        match action {
                            Action::Left => Caret::move_left()?,
                            Action::Right => Caret::move_right()?,
                            Action::Up => Caret::move_up()?,
                            Action::Down => Caret::move_down()?,
                            Action::Top => Caret::move_top()?,
                            Action::Bottom => Caret::move_bottom()?,
                            Action::MaxLeft => Caret::move_max_left()?,
                            Action::MaxRight => Caret::move_max_right()?,
                            Action::NextLine => Terminal::next_line()?,
                            Action::Quit => self.quit_program = true,
                            Action::Print => {
                                if let KeyCode::Char(character) = event.code {
                                    let (x, _) = position()?;
                                    let size = Draw::get_size()?;
                            
                                    // Check if we are at the very last column
                                    if x >= size.width - 1 {
                                        // Move to the next line
                                        Terminal::next_line()?
                                    }
                                    
                                    Draw::print_character(&character.to_string())?;
                                }
                            }
                        }
                        Terminal::execute()?;
                    }
                }
            }

            if self.quit_program { break; }
        }
        Ok(())
    }
}