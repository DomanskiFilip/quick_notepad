// GUI-specific syntax highlighting wrapper
// Converts core TokenType RGB values into egui Color32 for rendering in the GUI.

use crate::core::syntax::{SyntaxHighlighter as CoreSyntaxHighlighter, TokenType};
use egui::Color32;

pub trait TokenTypeExt {
    fn color(&self) -> Color32;
}

impl TokenTypeExt for TokenType {
    fn color(&self) -> Color32 {
        let (r, g, b) = self.rgb();
        Color32::from_rgb(r, g, b)
    }
}

// Re-export core SyntaxHighlighter for convenience in GUI code
pub type SyntaxHighlighter = CoreSyntaxHighlighter;
