// TUI-specific syntax highlighting wrapper
// This wraps the core syntax module and provides crossterm Color conversion

use crate::core::syntax::{SyntaxHighlighter as CoreHighlighter, Token as CoreToken, TokenType};
use crossterm::style::Color;

// Re-export the core types
pub use crate::core::syntax::{Token, TokenType as TokenTypeCore};

// Extension trait to add color method for TUI
pub trait TokenTypeExt {
    fn color(&self) -> Color;
}

impl TokenTypeExt for TokenType {
    fn color(&self) -> Color {
        let (r, g, b) = self.rgb();
        Color::Rgb { r, g, b }
    }
}

// Re-export SyntaxHighlighter from core
pub type SyntaxHighlighter = CoreHighlighter;