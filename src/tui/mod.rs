// TUI module - handles terminal user interface
pub mod view;
mod main_error_wrapper;
mod terminal;
mod caret;

use crate::core::{
    actions::Action,
    shortcuts::Shortcuts,
};
use crossterm::event::{Event, KeyCode, KeyEventKind, read};
use main_error_wrapper::MainErrorWrapper;
use view::{View, Buffer};
use terminal::Terminal;
use caret::Caret;

pub struct TerminalEditor {
    view: View,
    caret: Caret,
    shortcuts: Shortcuts,
    quit_program: bool,
}

impl TerminalEditor {
    pub fn new(buffer: Buffer) -> Self {
        Self {
            view: View::new(buffer),
            caret: Caret::new(),
            shortcuts: Shortcuts::new(),
            quit_program: false,
        }
    }
    
    pub fn set_filename(&mut self, filename: String) {
        self.view.set_filename(filename);
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
                        if let Some(action) = self.shortcuts.resolve(&event) {
                            match action {
                                // Clipboard operations
                                Action::Copy => {
                                    self.view.copy_selection()?;
                                },
                                Action::Paste => {
                                    self.view.paste_from_clipboard(&mut self.caret)?;
                                },
                                Action::Cut => {
                                    self.view.cut_selection(&mut self.caret)?;
                                },
                                
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
                                Action::SelectLeft => self.view.move_with_selection("left", &mut self.caret)?,
                                Action::SelectRight => self.view.move_with_selection("right", &mut self.caret)?,
                                Action::SelectUp => self.view.move_with_selection("up", &mut self.caret)?,
                                Action::SelectDown => self.view.move_with_selection("down", &mut self.caret)?,
                                Action::SelectTop => self.view.move_with_selection("top", &mut self.caret)?,
                                Action::SelectBottom => self.view.move_with_selection("bottom", &mut self.caret)?,
                                Action::SelectMaxLeft => self.view.move_with_selection("max_left", &mut self.caret)?,
                                Action::SelectMaxRight => self.view.move_with_selection("max_right", &mut self.caret)?,
                                
                                Action::SelectAll => self.view.select_all(&mut self.caret)?,
                                Action::NextLine => self.view.insert_newline(&mut self.caret)?,
                                Action::Backspace => self.view.backspace(&mut self.caret)?,
                                Action::Delete => self.view.delete_char(&mut self.caret)?,
                                Action::ToggleCtrlShortcuts => {
                                    self.view.toggle_ctrl_shortcuts();
                                    self.view.render(&mut self.caret)?;
                                },
                                Action::Save => {
                                    // TODO: Implement save
                                },
                                Action::New => {
                                    // TODO: Implement new file
                                },
                                Action::Quit => self.quit_program = true,
                                Action::Print => {
                                    if let KeyCode::Char(character) = event.code {
                                        self.view.type_character(character, &mut self.caret)?;
                                    }
                                }
                                _ => {}
                            }
                            Terminal::execute()?;
                        }
                    }
                },
                Event::Mouse(mouse_event) => {
                    if let Some(action) = self.shortcuts.resolve_mouse(&mouse_event) {
                        match action {
                            Action::MouseDown(x, y) => {
                                self.view.handle_mouse_down(x, y, &mut self.caret)?;
                            },
                            Action::MouseDrag(x, y) => {
                                self.view.handle_mouse_drag(x, y, &mut self.caret)?;
                            },
                            Action::MouseUp(x, y) => {
                                self.view.handle_mouse_up(x, y, &mut self.caret)?;
                            },
                            Action::MouseDoubleClick(x, y) => {
                                self.view.handle_double_click(x, y, &mut self.caret)?;
                            },
                            Action::MouseTripleClick(x, y) => {
                                self.view.handle_triple_click(x, y, &mut self.caret)?;
                            },
                            _ => {}
                        }
                        Terminal::execute()?;
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