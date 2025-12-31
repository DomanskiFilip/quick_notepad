// terminal module responsible for terminal manipulation and information
use crate::tui::{
    view::View,
    caret::{ Position, Caret },
};
use crossterm::{
    event::{EnableMouseCapture, DisableMouseCapture},
    cursor::{ DisableBlinking, EnableBlinking, Hide, Show },
    queue,
    terminal::{ 
        Clear, ClearType, DisableLineWrap, disable_raw_mode, 
        enable_raw_mode, size, EnterAlternateScreen, LeaveAlternateScreen 
    }
};
use std::io::{ stdout, Error, Write };

#[derive(Copy, Clone)]
pub struct Size {
    pub height: u16,
    pub width: u16,
}

pub struct Terminal;

impl Terminal {
    
    pub fn initialize(view: &View, caret: &mut Caret) -> Result<(), Error> {
        enable_raw_mode()?;
        queue!(stdout(), EnterAlternateScreen, DisableLineWrap, Hide, EnableMouseCapture )?;
        Self::clear_screen()?;
        
        queue!(stdout(), Caret::CARET_SETTINGS.style)?;
        Caret::set_caret_color(Caret::CARET_SETTINGS.color)?;
    
        view.render(caret)?;
        queue!(stdout(), Show, EnableBlinking)?;
        caret.move_to(Position { x: Position::MARGIN, y: Position::HEADER })?;
        
        Self::execute()?;
        Ok(())
    }

    pub fn terminate() -> Result<(), Error> {
        Caret::reset_caret_color()?;
        queue!(stdout(), DisableBlinking, Show, LeaveAlternateScreen, DisableMouseCapture)?;
        disable_raw_mode()?;
        Self::execute()?;
        println!("Goodbye.");
        Ok(())
    }

    pub fn clear_screen() -> Result<(), Error> {
        queue!(stdout(), Clear(ClearType::All))?;
        Ok(())
    }
    
    pub fn clear_rest_of_line() -> Result<(), Error> {
        queue!(stdout(), Clear(ClearType::UntilNewLine))?;
        Ok(())
    }

    pub fn execute() -> Result<(), Error> {
        stdout().flush()?;
        Ok(())
    }
    
    pub fn get_size() -> Result<Size, Error> {
        let (width, height) = size()?;
        Ok(Size { width, height })
    }
}