// terminal module responsible for terminal manipulation and information
use crate::tui::{
    view::View,
    caret::{ Position, Caret },
};
use crossterm::{
    event::{
        EnableMouseCapture, DisableMouseCapture, KeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
    },
    cursor::{ DisableBlinking, EnableBlinking, Hide, Show },
    queue,
    terminal::{ 
        Clear, ClearType, DisableLineWrap, disable_raw_mode, 
        enable_raw_mode, size, EnterAlternateScreen, LeaveAlternateScreen,
        supports_keyboard_enhancement,
    }
};
use std::io::{ stdout, Error, Write };
use std::sync::atomic::{ AtomicBool, Ordering };

// true when the terminal reports keys like Ctrl+1 (kitty keyboard protocol)
static KEYBOARD_ENHANCED: AtomicBool = AtomicBool::new(false);

#[derive(Copy, Clone)]
pub struct Size {
    pub height: u16,
    pub width: u16,
}

pub struct Terminal;

impl Terminal {
    
    pub fn initialize(view: &mut View, caret: &mut Caret) -> Result<(), Error> {
        enable_raw_mode()?;
        queue!(stdout(), EnterAlternateScreen, DisableLineWrap, Hide, EnableMouseCapture )?;

        // Most terminals send nothing usable for Ctrl+number (Ctrl+1 is just "1"),
        // ask terminals that support it to report those keys properly.
        if supports_keyboard_enhancement().unwrap_or(false) {
            queue!(stdout(), PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES))?;
            KEYBOARD_ENHANCED.store(true, Ordering::Relaxed);
        }
        Self::clear_screen()?;
        
        queue!(stdout(), Caret::CARET_SETTINGS.style)?;
        Caret::set_caret_color(Caret::CARET_SETTINGS.color)?;
    
        view.render(caret)?;
        queue!(stdout(), Show, EnableBlinking)?;
        caret.move_to(Position { x: Position::MARGIN, y: Position::HEADER })?;
        
        Self::execute()?;
        Ok(())
    }

    pub fn terminate() -> Result<(), Error> {
        Caret::reset_caret_color()?;
        if KEYBOARD_ENHANCED.load(Ordering::Relaxed) {
            queue!(stdout(), PopKeyboardEnhancementFlags)?;
        }
        queue!(stdout(), DisableBlinking, Show, LeaveAlternateScreen, DisableMouseCapture)?;
        disable_raw_mode()?;
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

    pub fn execute() -> Result<(), Error> {
        stdout().flush()?;
        Ok(())
    }
    
    pub fn get_size() -> Result<Size, Error> {
        let (width, height) = size()?;
        Ok(Size { width, height })
    }
}