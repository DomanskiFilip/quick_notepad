use std::io::{stdout, Error };
use crossterm::{
    style::{Print, Color, SetForegroundColor, ResetColor},
    terminal::size,
    queue,
    cursor::MoveTo,
};

#[derive(Copy, Clone)]
pub struct Size {
    pub height: u16,
    pub width: u16,
}

pub struct View;

impl View {
    pub fn get_size() -> Result<Size, Error> {
        let (width, height) = size()?;
        Ok(Size { width, height })
    }
    
    pub fn draw_margin() -> Result<(), Error> {
        let size = Self::get_size()?;
        
        // Loop from top to bottom (minus the footer line)
        for i in 0..size.height - 1 {
            queue!(
                stdout(),
                MoveTo(0, i),
                SetForegroundColor(Color::DarkGrey),
                Print(format!("{:>3} ", i + 1)),
                ResetColor
            )?;
        }    
        Ok(())
    }
    
    pub fn draw_footer() -> Result<(), Error> {
        let size = Self::get_size()?;
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

    pub fn print_character(character: &str) -> Result<(), Error> {
        queue!(stdout(), Print(character))?;
        Ok(())
    }
}