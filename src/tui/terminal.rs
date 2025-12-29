use crate::tui::view::View;
use crate::tui::caret::Caret;
use crossterm::{
    cursor::{ DisableBlinking, EnableBlinking, Hide, MoveTo, Show, position },
    queue,
    style::Print,
    terminal::{ Clear, ClearType, DisableLineWrap, disable_raw_mode, enable_raw_mode }
};
use std::io::{ stdout, Error, Write };

#[derive(Copy, Clone)]
pub struct Position {
    pub x: u16,
    pub y: u16,
}

pub struct Terminal;

impl Terminal {
    
    // initialize tui
    pub fn initialize() -> Result<(), Error> {
        enable_raw_mode()?;
        // Queue all initial setup commands
        queue!(stdout(), DisableLineWrap, Hide)?;
        Self::clear_screen()?;
        // set cursor color
        queue!(stdout(), Caret::CARET_SETTINGS.style)?;
        Self::set_cursor_color(Caret::CARET_SETTINGS.color)?;
        View::draw_margin()?;
        View::draw_footer()?;
        queue!(stdout(), Show, EnableBlinking)?;
        Self::move_cursor_to(Position { x: 4, y: 0 })?;
        // Single flush to render everything at once
        Self::execute()?;
        Ok(())
    }

    // terminate tui
    pub fn terminate() -> Result<(), Error> {
        // show cursor
        Self::reset_cursor_color()?;
        queue!(stdout(), DisableBlinking, Show)?;
        Self::execute()?;
        // draw Godbye msg
        disable_raw_mode()?;
        Self::clear_screen()?;
        Self::move_cursor_to(Position { x: 0, y: 0 })?;
        Self::execute()?;
        println!("Goodbye.");
        Ok(())
    }

    pub fn clear_screen() -> Result<(), Error> {
        queue!(stdout(), Clear(ClearType::All))?;
        Ok(())
    }
    
    pub fn set_cursor_color(color: &str) -> Result<(), Error> {
        // \x1b]12; is the start of the "Change Cursor Color" sequence
        // \x07 is the string terminator (Bell character)
        queue!(stdout(), Print(format!("\x1b]12;{}\x07", color)))?;
        Ok(())
    }

    pub fn reset_cursor_color() -> Result<(), Error> {
        queue!(stdout(), Print("\x1b]112\x07"))?;
        Ok(())
    }

    pub fn move_cursor_to(pos: Position) -> Result<(), Error> {
        queue!(stdout(), MoveTo(pos.x, pos.y))?;
        Ok(())
    }

    // The "Flush" method - sends all queued commands to the terminal
    pub fn execute() -> Result<(), Error> {
        stdout().flush()?;
        Ok(())
    }
}