// view module responsible for render and buffer
use crate::tui::{
    caret::{ Position, Caret },
    terminal::Terminal,
};
use std::io::{stdout, Error };
use crossterm::{
    style::{Print, Color, SetForegroundColor, ResetColor},
    queue,
    cursor::MoveTo,
};

pub struct View {
    buffer: Buffer,
    scroll_offset: usize,
}

pub struct Buffer {
    pub lines: Vec<String> 
}

impl View {
    
    pub fn render(&self) -> Result<(), Error> {
        let size = Terminal::get_size()?;
        let visible_rows = (size.height - 1) as usize;
        
        Terminal::clear_screen()?;
        
        for row in 0..visible_rows {
            let buffer_line_idx = row + self.scroll_offset;
            queue!(stdout(), MoveTo(0, row as u16))?;
            self.draw_margin_line(row as u16, buffer_line_idx)?;
            
            if let Some(line) = self.buffer.lines.get(buffer_line_idx) {
                let max_width = (size.width.saturating_sub(4)) as usize;
                let truncated_line = if line.len() > max_width {
                    &line[..max_width]
                } else {
                    line
                };
                Self::print(truncated_line)?;
            }
        }
        
        self.draw_footer()?;
        Ok(())
    }

    fn draw_margin_line(&self, row: u16, buffer_line_idx: usize) -> Result<(), Error> {
        queue!(
            stdout(),
            MoveTo(0, row),
            SetForegroundColor(Color::DarkGrey),
            Print(format!("{:>3} ", buffer_line_idx + 1)),
            ResetColor
        )?;
        Ok(())
    }

    pub fn draw_footer(&self) -> Result<(), Error> {
        let size = Terminal::get_size()?;
        let footer_row = size.height - 1;
        
        queue!(
            stdout(),
            MoveTo(0, footer_row), 
            SetForegroundColor(Color::DarkGrey),
            Print("ctrl + q = quit |"),
            MoveTo(size.width / 2, footer_row),
            Print("© Filip Domanski"),
            ResetColor,
        )?;
        Ok(())
    }

    pub fn print(text: &str) -> Result<(), Error> {
        queue!(stdout(), Print(text))?;
        Ok(())
    }
    
    pub fn type_character(&mut self, character: char, caret: &mut Caret) -> Result<(), Error> {
        let size = Terminal::get_size()?;
        let pos = caret.get_position();
        
        if pos.y >= size.height - 1 {
            return Ok(());
        }
        
        let buffer_line_idx = pos.y as usize + self.scroll_offset;

        while self.buffer.lines.len() <= buffer_line_idx {
            self.buffer.lines.push(String::new());
        }

        let char_pos = (pos.x as usize).saturating_sub(4);
        
        if pos.x >= size.width - 1 {
            self.insert_newline(caret)?;
            return self.type_character(character, caret);
        }

        let line = &mut self.buffer.lines[buffer_line_idx];
        if char_pos <= line.len() {
            line.insert(char_pos, character);
        } else {
            line.push(character);
        }

        self.render()?;
        
        let new_x = pos.x + 1;
        if new_x < size.width - 1 {
            caret.move_to(Position { x: new_x, y: pos.y })?;
        } else if pos.y < size.height - 2 {
            caret.move_to(Position { x: 4, y: pos.y + 1 })?;
        }
        
        Ok(())
    }
    
    pub fn insert_newline(&mut self, caret: &mut Caret) -> Result<(), Error> {
        let size = Terminal::get_size()?;
        let pos = caret.get_position();
        
        if pos.y >= size.height - 1 {
            return Ok(());
        }
        
        let buffer_line_idx = pos.y as usize + self.scroll_offset;
        let char_pos = (pos.x as usize).saturating_sub(4);

        while self.buffer.lines.len() <= buffer_line_idx {
            self.buffer.lines.push(String::new());
        }
        
        let current_line = &mut self.buffer.lines[buffer_line_idx];
        let new_line_content = if char_pos < current_line.len() {
            current_line.split_off(char_pos)
        } else {
            String::new()
        };
        
        self.buffer.lines.insert(buffer_line_idx + 1, new_line_content);

        if pos.y >= size.height - 2 {
            self.scroll_offset += 1;
            self.render()?;
            caret.next_line()?;
        } else {
            self.render()?;
            caret.next_line()?;
        }
        
        Ok(())
    }
    
    pub fn move_up(&mut self, caret: &mut Caret) -> Result<(), Error> {
        let (new_offset, needs_render) = caret.move_up(self.scroll_offset)?;
        self.scroll_offset = new_offset;
        if needs_render {
            self.render()?;
        }
        self.clamp_cursor_to_line(caret)?;
        Ok(())
    }
    
    pub fn move_down(&mut self, caret: &mut Caret) -> Result<(), Error> {
        let (new_offset, needs_render) = caret.move_down(self.scroll_offset, self.buffer.lines.len())?;
        self.scroll_offset = new_offset;
        if needs_render {
            self.render()?;
        }
        self.clamp_cursor_to_line(caret)?;
        Ok(())
    }
    
    fn clamp_cursor_to_line(&self, caret: &mut Caret) -> Result<(), Error> {
        let pos = caret.get_position();
        let buffer_line_idx = pos.y as usize + self.scroll_offset;
        
        if let Some(line) = self.buffer.lines.get(buffer_line_idx) {
            let line_end = 4 + line.len() as u16;
            let size = Terminal::get_size()?;
            let max_x = line_end.min(size.width - 1);
            
            if pos.x > max_x {
                caret.move_to(Position { x: max_x, y: pos.y })?;
            }
        }
        
        Ok(())
    }
    
    pub fn move_left(&mut self, caret: &mut Caret) -> Result<(), Error> {
        let pos = caret.get_position();
        let buffer_line_idx = pos.y as usize + self.scroll_offset;
        
        let (new_offset, needs_render) = caret.move_left(self.scroll_offset)?;
        self.scroll_offset = new_offset;
        if needs_render {
            self.render()?;
        }
        self.clamp_cursor_to_line(caret)?;
        Ok(())
    }
    
    pub fn move_right(&mut self, caret: &mut Caret) -> Result<(), Error> {
        let pos = caret.get_position();
        let buffer_line_idx = pos.y as usize + self.scroll_offset;
        
        // Check if we can move right on current line
        if let Some(line) = self.buffer.lines.get(buffer_line_idx) {
            let char_pos = (pos.x as usize).saturating_sub(4);
            let size = Terminal::get_size()?;
            let line_end = 4 + line.len() as u16;
            
            // If at end of line, move to next line
            if pos.x >= line_end && pos.x >= size.width - 1 {
                let (new_offset, needs_render) = caret.move_right(self.scroll_offset, self.buffer.lines.len())?;
                self.scroll_offset = new_offset;
                if needs_render {
                    self.render()?;
                }
                self.clamp_cursor_to_line(caret)?;
                return Ok(());
            }
            
            // If at end of line content but not screen edge
            if char_pos >= line.len() {
                // Try to move to next line
                if pos.y < size.height - 2 || self.scroll_offset + pos.y as usize + 1 < self.buffer.lines.len() {
                    let (new_offset, needs_render) = caret.move_right(self.scroll_offset, self.buffer.lines.len())?;
                    self.scroll_offset = new_offset;
                    if needs_render {
                        self.render()?;
                    }
                    self.clamp_cursor_to_line(caret)?;
                    return Ok(());
                }
                return Ok(()); // Can't move further
            }
        }
        
        let (new_offset, needs_render) = caret.move_right(self.scroll_offset, self.buffer.lines.len())?;
        self.scroll_offset = new_offset;
        if needs_render {
            self.render()?;
        }
        self.clamp_cursor_to_line(caret)?;
        Ok(())
    }
    
    pub fn move_top(&mut self, caret: &mut Caret) -> Result<(), Error> {
        let (new_offset, needs_render) = caret.move_top()?;
        self.scroll_offset = new_offset;
        if needs_render {
            self.render()?;
        }
        self.clamp_cursor_to_line(caret)?;
        Ok(())
    }
    
    pub fn move_bottom(&mut self, caret: &mut Caret) -> Result<(), Error> {
        caret.move_bottom()?;
        self.clamp_cursor_to_line(caret)?;
        Ok(())
    }
    
    pub fn move_max_left(&mut self, caret: &mut Caret) -> Result<(), Error> {
        caret.move_max_left()?;
        self.clamp_cursor_to_line(caret)?;
        Ok(())
    }
    
    pub fn move_max_right(&mut self, caret: &mut Caret) -> Result<(), Error> {
        caret.move_max_right()?;
        self.clamp_cursor_to_line(caret)?;
        Ok(())
    }
    
    pub fn handle_resize(&mut self, caret: &mut Caret) -> Result<(), Error> {
        caret.clamp_to_bounds()?;
        self.render()?;
        caret.move_to(caret.get_position())?;
        Ok(())
    }
}

impl Default for View {
    fn default() -> Self {
        Self {
            buffer: Buffer::default(),
            scroll_offset: 0,
        }
    }
}

impl Default for Buffer {
     fn default() -> Self{
        let mut lines = Vec::new();
        for _ in 0..500 {
            lines.push(String::new());
        }
        Self { lines }
    }
}