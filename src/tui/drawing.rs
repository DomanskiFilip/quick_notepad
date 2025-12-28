use std::io::stdout;
use crossterm::{
    style::{Print, Color, SetForegroundColor, ResetColor},
    terminal::size,
    execute,
    cursor::MoveTo,
};

pub struct Draw;

impl Draw {
    pub fn draw_margin() -> Result<(), std::io::Error> {
        let (_width, height) = size()?;
        
        for i in 0..height - 1 {
            execute!(
                stdout(),
                MoveTo(0, i),
                SetForegroundColor(Color::DarkGrey),
                Print(format!("{:>3} ", i + 1)),
                ResetColor
            )?;
        }    
        Ok(())
    }
    
    pub fn draw_footer() -> Result<(), std::io::Error> {
        let (width, height) = size()?;
        
        execute!(
            stdout(),
            MoveTo(0, height - 1), 
            SetForegroundColor(Color::DarkGrey),
            Print("ctrl + q = quit |"),
            MoveTo(width / 2, height - 1),
            Print("© Filip Domanski"),
            ResetColor,
        )?;
        Ok(())
    }

    pub fn print_character(character: &str) -> Result<(), std::io::Error> {
        execute!(stdout(), Print(character))?;
        Ok(())
    }
}