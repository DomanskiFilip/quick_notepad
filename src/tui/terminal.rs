use crate::tui::drawing::Draw;
use crossterm::{
    cursor::{MoveTo, position, Hide, Show, EnableBlinking, DisableBlinking},
    execute,
    terminal::{Clear, ClearType, disable_raw_mode, enable_raw_mode, DisableLineWrap},
};
use std::io::stdout;

pub struct Terminal;

impl Terminal {
    pub fn initialize() -> Result<(), std::io::Error> {
        enable_raw_mode()?;
        execute!(stdout(), DisableLineWrap, Hide)?;
        // hide cursor
        let _ = execute!(stdout(), Hide);
        Self::clear_screen()?;
        Draw::draw_margin()?;
        Draw::draw_footer()?;
        // show cursor
        let _ = execute!(stdout(), Show);
        let _ = execute!(stdout(), EnableBlinking);
        Self::move_cursor_to(4, 0)?;
        Ok(())
    }

    pub fn terminate() -> Result<(), std::io::Error> {
        // hide cursor
        let _ = execute!(stdout(), DisableBlinking);
        execute!(stdout(), Hide)?;
        disable_raw_mode()?;
        Self::clear_screen()?;
        // print Goodbye msg
        Self::move_cursor_to(0, 0)?;
        println!("Goodbye.");
        Ok(())
    }

    pub fn clear_screen() -> Result<(), std::io::Error> {
        execute!(stdout(), Clear(ClearType::All))?;
        Ok(())
    }

    fn move_cursor_to(x: u16, y: u16) -> Result<(), std::io::Error> {
        execute!(stdout(), MoveTo(x, y))?;
        Ok(())
    }

    pub fn next_line() -> Result<(), std::io::Error> {
        let (_, y) = position()?;
        Self::move_cursor_to(4, y + 1)?;
        Ok(())
    }
}