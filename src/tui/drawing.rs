// drawing module handles printing and other drawing operations
use std::io::stdout;
use crossterm::{
    style::{ Print, Color, SetForegroundColor, ResetColor },
    terminal::size,
    execute,
    cursor::MoveTo,
};

pub struct Draw;

impl Draw {
    // Draws margin with numbers in the first column, grayed out
    pub fn draw_margin() -> Result<(), std::io::Error> {
        match size() {
            Ok((_width, height)) => {
                // Print line numbers
                for i in 0..height - 1 {
                    match execute!(
                        stdout(),
                        MoveTo(0, i),
                        SetForegroundColor(Color::DarkGrey),
                        Print(format!("{:>3}", i + 1)),
                        ResetColor
                    ) {
                        Ok(_) => {},
                        Err(error) => eprintln!("Failed to print line {}: {error:#?}", i + 1),
                    }
                }    
                Ok(())
            }
            Err(error) => Err(error),
        }
    }
    
    pub fn draw_footer() -> Result<(), std::io::Error> {
        match size() {
            Ok((width, height)) => {
                // print legend at the bottom left of the screen
                match execute!(
                    stdout(),
                    MoveTo(0, height), 
                    SetForegroundColor(Color::DarkGrey),
                    Print("ctrl + q = quit|"),
                    ResetColor,
                ) {
                    Ok(_) => {},
                    Err(error) => eprintln!("Failed to print copyright line: {error:#?}"),
                }
                
                // print copyright message at the bottom center of the screen
                match execute!(
                    stdout(),
                    MoveTo(width / 2, height), 
                    SetForegroundColor(Color::DarkGrey),
                    Print("© Filip Domanski"),
                    ResetColor,
                ) {
                    Ok(_) => {},
                    Err(error) => eprintln!("Failed to print copyright line: {error:#?}"),
                }
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    // Prints a string character at the current cursor position
    pub fn print_character(character: &str) {
        match execute!(stdout(), Print(character)) {
            Ok(_) => {},
            Err(e) => eprintln!("Failed to print character: {e:#?}"),
        }
    }
}
