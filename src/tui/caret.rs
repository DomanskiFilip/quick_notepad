// caret module responsible for caret manipulation and settings
use crate::tui::terminal::Terminal;
use crossterm::{
    cursor::{ SetCursorStyle, MoveTo },
    style::Print,
    queue,
};
use std::io::{ stdout, Error, Write };

#[derive(Copy, Clone, Debug)]
pub struct Position {
    pub x: u16,
    pub y: u16,
}

impl Position {
    pub const MARGIN: u16 = 4; // Width of the margin
    pub const HEADER: u16 = 1; // Height of the header
}

impl Default for Position {
    fn default() -> Self {
        Self { 
            x: Self::MARGIN, 
            y: Self::HEADER 
        } 
    }
}

pub struct Caret {
    pub color: &'static str,
    pub style: SetCursorStyle,
    position: Position,
}

impl Caret {
    pub const CARET_SETTINGS: Caret = Caret { 
        color: "yellow", 
        style: SetCursorStyle::BlinkingBar,
        position: Position { x: 4, y: 0 },
    };
    
    pub fn new() -> Self {
        Self {
            color: "yellow",
            style: SetCursorStyle::BlinkingBar,
            position: Position::default(),
        }
    }
    
    pub fn set_caret_color(color: &str) -> Result<(), Error> {
        queue!(stdout(), Print(format!("\x1b]12;{}\x07", color)))?;
        Ok(())
    }

    pub fn reset_caret_color() -> Result<(), Error> {
        queue!(stdout(), Print("\x1b]112\x07"))?;
        Ok(())
    }

    pub fn move_to(&mut self, pos: Position) -> Result<(), Error> {
        self.position = pos;
        queue!(stdout(), MoveTo(pos.x, pos.y))?;
        stdout().flush()?;
        Ok(())
    }
    
    pub fn get_position(&self) -> Position {
        self.position
    }
    
    pub fn clamp_to_bounds(&mut self) -> Result<(), Error> {
        let size = Terminal::get_size()?;
        self.position.x = self.position.x.min(size.width - 1).max(4);
        self.position.y = self.position.y.min(size.height - 2);
        self.move_to(self.position)?;
        Ok(())
    }
}