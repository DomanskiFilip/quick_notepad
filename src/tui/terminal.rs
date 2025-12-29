// terminal module responsible for terminal manipulation and information
use crate::tui::{
    view::View,
    caret::{ Position, Caret },
};
use crossterm::{
    cursor::{ DisableBlinking, EnableBlinking, Hide, Show },
    queue,
    terminal::{ Clear, ClearType, DisableLineWrap, disable_raw_mode, enable_raw_mode, size }
};
use std::io::{ stdout, Error, Write };

#[derive(Copy, Clone)]
pub struct Size {
    pub height: u16,
    pub width: u16,
}

pub struct Terminal;

impl Terminal {
    
    // initialize tui
    pub fn initialize(view: &View, caret: &mut Caret) -> Result<(), Error> {
        enable_raw_mode()?;
        queue!(stdout(), DisableLineWrap, Hide)?;
        Self::clear_screen()?;
        
        // Setup Caret
        queue!(stdout(), Caret::CARET_SETTINGS.style)?;
        Caret::set_caret_color(Caret::CARET_SETTINGS.color)?;
    
        // Render the view (buffer + margins + footer)
        view.render()?;
    
        queue!(stdout(), Show, EnableBlinking)?;
        // Start the cursor at the "text area" (after the margin)
        caret.move_to(Position { x: 4, y: 0 })?;
        
        Self::execute()?;
        Ok(())
    }

    // terminate tui
    pub fn terminate() -> Result<(), Error> {
        // show cursor
        Caret::reset_caret_color()?;
        queue!(stdout(), DisableBlinking, Show)?;
        Self::execute()?;
        // draw Godbye msg
        disable_raw_mode()?;
        Self::clear_screen()?;
        queue!(stdout(), crossterm::cursor::MoveTo(0, 0))?;
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

    // The "Flush" method - sends all queued commands to the terminal
    pub fn execute() -> Result<(), Error> {
        stdout().flush()?;
        Ok(())
    }
    
    pub fn get_size() -> Result<Size, Error> {
        let (width, height) = size()?;
        Ok(Size { width, height })
    }
}