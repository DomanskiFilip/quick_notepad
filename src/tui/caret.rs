use crate::tui::terminal::Terminal;
use crate::tui::view::View;
use crate::tui::terminal::Position;
use crossterm::cursor::{ position, SetCursorStyle };
use std::io::{ stdout, Error, Write };

pub struct Caret {
    pub color: &'static str,
    pub style: SetCursorStyle,
}

impl Caret {
    pub const CARET_SETTINGS: Caret = Caret { 
        color: "yellow", 
        style: SetCursorStyle::BlinkingBar
    };
    
    
    pub fn next_line() -> Result<(), Error> {
        let (_, y) = position()?;
        let size = View::get_size()?; 
        Terminal::move_cursor_to(Position { x: 4, y: y + 1 })?;
        if y + 1 == size.height - 1 {
            Terminal::move_cursor_to(Position { x: 4, y: y })?;
        }
        stdout().flush()?;
        Ok(())
    }

    pub fn move_left() -> Result<(), Error> {
        let (x, y) = position()?;
        let size = View::get_size()?;
    
        if x > 4 {
            Terminal::move_cursor_to(Position { x: x - 1, y: y })?;
        } else if y > 0 {
            Terminal::move_cursor_to(Position { x: size.width - 1, y: y - 1 })?;
        }
        Ok(())
    }
    
    pub fn move_right() -> Result<(), Error> {
        let (x, y) = position()?;
        let size = View::get_size()?; 
    
        if x < size.width - 1 {
            Terminal::move_cursor_to(Position { x: x + 1, y: y })?;
        } else if y < size.height - 2 {
            Terminal::move_cursor_to(Position { x: 4, y: y + 1 })?;
        }
        Ok(())
    }

    pub fn move_up() -> Result<(), Error> {
        let (x, y) = position()?;
        if y > 0 {
            Terminal::move_cursor_to(Position { x, y: y - 1 })?;
        }
        Ok(())
    }

    pub fn move_down() -> Result<(), Error> {
        let (x, y) = position()?;
        let size = View::get_size()?; 
        if y < size.height - 2 { 
            Terminal::move_cursor_to(Position { x, y: y + 1 })?;
        }
        Ok(())
    }

    pub fn move_top() -> Result<(), Error> {
        let (x, _) = position()?;
        Terminal::move_cursor_to(Position { x, y: 0 })?;
        Ok(())
    }

    pub fn move_bottom() -> Result<(), Error> {
        let (x, _) = position()?;
        let size = View::get_size()?;
        Terminal::move_cursor_to(Position { x, y: size.height - 2 })?;
        Ok(())
    }

    pub fn move_max_left() -> Result<(), Error> {
        let (_, y) = position()?;
        Terminal::move_cursor_to(Position { x: 4, y })?; 
        Ok(())
    }

    pub fn move_max_right() -> Result<(), Error> {
        let (_, y) = position()?;
        let size = View::get_size()?;
        Terminal::move_cursor_to(Position { x: size.width - 1, y })?;
        Ok(())
    }
}