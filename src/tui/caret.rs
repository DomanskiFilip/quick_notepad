// caret module responsible for caret manipulation and settings
use crate::tui::terminal::Terminal;
use crossterm::{
    cursor::{ SetCursorStyle, MoveTo },
    style::Print,
    queue,
};
use std::io::{ stdout, Error, Write };

#[derive(Copy, Clone)]
pub struct Position {
    pub x: u16,
    pub y: u16,
}

impl Default for Position {
    fn default() -> Self {
        Self { x: 4, y: 0 }
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
    
    pub fn next_line(&mut self) -> Result<(), Error> {
        let size = Terminal::get_size()?;
        
        if self.position.y < size.height - 2 {
            self.position.x = 4;
            self.position.y += 1;
            self.move_to(self.position)?;
        }
        Ok(())
    }

    pub fn move_left(&mut self, scroll_offset: usize) -> Result<(usize, bool), Error> {
        let size = Terminal::get_size()?;
        let mut new_offset = scroll_offset;
        let mut needs_render = false;
        
        if self.position.x > 4 {
            self.position.x -= 1;
            self.move_to(self.position)?;
        } else if self.position.y > 0 {
            self.position.x = size.width - 1;
            self.position.y -= 1;
            self.move_to(self.position)?;
        } else if scroll_offset > 0 {
            self.position.x = size.width - 1;
            new_offset -= 1;
            needs_render = true;
        }
        
        Ok((new_offset, needs_render))
    }
    
    pub fn move_right(&mut self, scroll_offset: usize, max_lines: usize) -> Result<(usize, bool), Error> {
        let size = Terminal::get_size()?;
        let mut new_offset = scroll_offset;
        let mut needs_render = false;
        
        if self.position.x < size.width - 1 {
            self.position.x += 1;
            self.move_to(self.position)?;
        } else if self.position.y < size.height - 2 {
            self.position.x = 4;
            self.position.y += 1;
            self.move_to(self.position)?;
        } else {
            let max_scroll = max_lines.saturating_sub((size.height - 1) as usize);
            if scroll_offset < max_scroll {
                self.position.x = 4;
                new_offset += 1;
                needs_render = true;
            }
        }
        
        Ok((new_offset, needs_render))
    }

    pub fn move_up(&mut self, scroll_offset: usize) -> Result<(usize, bool), Error> {
        let mut new_offset = scroll_offset;
        let mut needs_render = false;
        
        if self.position.y == 0 && scroll_offset > 0 {
            new_offset -= 1;
            needs_render = true;
        } else if self.position.y > 0 {
            self.position.y -= 1;
            self.move_to(self.position)?;
        }
        
        Ok((new_offset, needs_render))
    }

    pub fn move_down(&mut self, scroll_offset: usize, max_lines: usize) -> Result<(usize, bool), Error> {
        let size = Terminal::get_size()?;
        let mut new_offset = scroll_offset;
        let mut needs_render = false;
        
        if self.position.y >= size.height - 2 {
            let max_scroll = max_lines.saturating_sub((size.height - 1) as usize);
            if scroll_offset < max_scroll {
                new_offset += 1;
                needs_render = true;
            }
        } else if self.position.y < size.height - 2 {
            self.position.y += 1;
            self.move_to(self.position)?;
        }
        
        Ok((new_offset, needs_render))
    }

    pub fn move_top(&mut self) -> Result<(usize, bool), Error> {
        self.position.y = 0;
        self.move_to(self.position)?;
        Ok((0, true))
    }

    pub fn move_bottom(&mut self) -> Result<(), Error> {
        let size = Terminal::get_size()?;
        self.position.y = size.height - 2;
        self.move_to(self.position)?;
        Ok(())
    }

    pub fn move_max_left(&mut self) -> Result<(), Error> {
        self.position.x = 4;
        self.move_to(self.position)?;
        Ok(())
    }

    pub fn move_max_right(&mut self) -> Result<(), Error> {
        let size = Terminal::get_size()?;
        self.position.x = size.width - 1;
        self.move_to(self.position)?;
        Ok(())
    }
    
    pub fn clamp_to_bounds(&mut self) -> Result<(), Error> {
        let size = Terminal::get_size()?;
        self.position.x = self.position.x.min(size.width - 1).max(4);
        self.position.y = self.position.y.min(size.height - 2);
        self.move_to(self.position)?;
        Ok(())
    }
}