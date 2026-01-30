// render module responsible for all the render logic
use super::View;
use super::graphemes::*;
use unicode_segmentation::UnicodeSegmentation; 
use crate::core::selection::TextPosition;
use crate::tui::{
    caret::{Caret, Position},
    terminal::Terminal,
    syntax::{SyntaxHighlighter, TokenTypeExt},
};
use crossterm::{
    cursor::MoveTo,
    queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor},
};
use std::io::{Error, stdout};

pub fn render_view(view: &View, caret: &Caret, is_dirty: bool) -> Result<(), Error> {
    let current_pos = caret.get_position();
    let size = Terminal::get_size()?;

    draw_header()?;

    let visible_rows = (size.height.saturating_sub(Position::HEADER + 1)) as usize;

    let last_non_empty_line = view
        .buffer
        .lines
        .iter()
        .rposition(|line| !line.is_empty())
        .unwrap_or(0);

    let selection_range = view
        .selection
        .as_ref()
        .filter(|s| s.is_active())
        .map(|s| s.get_range());

    // Create syntax highlighter based on current file type
    let highlighter = SyntaxHighlighter::new(view.filetype.clone());

    for row in 0..visible_rows {
        let buffer_line_idx = row + view.scroll_offset;
        let terminal_row = row as u16 + Position::HEADER;

        queue!(stdout(), MoveTo(0, terminal_row))?;
        Terminal::clear_rest_of_line()?;

        if buffer_line_idx <= last_non_empty_line {
            draw_margin_line(terminal_row, buffer_line_idx)?;
        }

        if let Some(line) = view.buffer.lines.get(buffer_line_idx) {
            let max_width = (size.width.saturating_sub(Position::MARGIN)) as usize;
            
            // Truncate by visual width, not grapheme count
            let mut truncated = String::new();
            let mut current_width = 0;
            
            for grapheme in line.graphemes(true) {
                let g_width = visual_width(grapheme);
                if current_width + g_width > max_width {
                    break;
                }
                truncated.push_str(grapheme);
                current_width += g_width;
            }

            render_line_with_selection_and_syntax(
                &truncated, 
                buffer_line_idx, 
                selection_range, 
                &highlighter
            )?;
        }
    }

    draw_footer(view, caret, is_dirty)?;

    queue!(stdout(), MoveTo(current_pos.x, current_pos.y))?;
    Ok(())
}

fn draw_header() -> Result<(), Error> {
    let size = Terminal::get_size()?;
    queue!(
        stdout(),
        MoveTo(0, 0),
        SetForegroundColor(Color::Yellow),
        MoveTo(size.width / 2, 0),
        Print(" Quick Notepad ".to_string()),
        ResetColor
    )?;
    Terminal::clear_rest_of_line()?;
    Ok(())
}

fn draw_margin_line(row: u16, buffer_line_idx: usize) -> Result<(), Error> {
    queue!(
        stdout(),
        MoveTo(0, row),
        SetForegroundColor(Color::Yellow),
        Print(format!("{:>3} ", buffer_line_idx + 1)),
        ResetColor
    )?;
    Ok(())
}

pub fn draw_footer(view: &View, caret: &Caret, is_dirty: bool) -> Result<(), Error> {
    let size = Terminal::get_size()?;
    let footer_row = size.height - 1;

    // Clear the footer line and set background
    queue!(
        stdout(),
        MoveTo(0, footer_row),
        SetBackgroundColor(Color::Black),
    )?;
    Terminal::clear_rest_of_line()?;

    // Move back to start of footer
    queue!(stdout(), MoveTo(0, footer_row))?;

    // If shortcuts are toggled, show them. If a prompt is active, render the prompt/footer.
    if view.show_shortcuts {
        draw_shortcuts_footer()?;
    } else if view.prompt.is_some() {
        draw_prompt_footer(view, caret)?;
    } else {
        draw_info_footer(view, caret, is_dirty)?;
    }

    queue!(stdout(), ResetColor)?;
    Ok(())
}

fn draw_info_footer(view: &View, caret: &Caret, is_dirty: bool) -> Result<(), Error> {
    let current_pos = caret.get_position();

    let size = Terminal::get_size()?;
    let footer_row = size.height - 1;

    // Left side: Filename or [No Name] and modified tag
    queue!(stdout(), MoveTo(1, footer_row))?;
    let filename_display = view.filename.as_deref().unwrap_or("[No Name]");
    let modified_tag = if is_dirty { "*" } else { "" };
    queue!(
        stdout(),
        SetBackgroundColor(Color::Black),
        SetForegroundColor(if is_dirty { Color::Red } else { Color::Yellow }),
        SetAttribute(Attribute::Bold),
        Print(format!(" {}{} ", filename_display, modified_tag)),
        SetAttribute(Attribute::Reset),
    )?;
    
    // filetype or [unknown file type]
    let filetype_display = view.filetype.as_deref().unwrap_or("[unknown file type]");
    queue!(
        stdout(),
        SetBackgroundColor(Color::Black),
        SetForegroundColor(Color::Yellow),
        SetAttribute(Attribute::Bold),
        Print(format!(" {} ", filetype_display)),
        SetAttribute(Attribute::Reset),
    )?;

    // Calculate stats - find last non-empty line for accurate count
    let total_lines = view
        .buffer
        .lines
        .iter()
        .rposition(|line| !line.is_empty())
        .map(|idx| idx + 1)
        .unwrap_or(1);
    let total_chars: usize = view
        .buffer
        .lines
        .iter()
        .take(total_lines)
        .map(|line| line.len())
        .sum();

    // Current position (adjust for margin)
    let line_num = current_pos.x.saturating_sub(Position::HEADER) + 1 + view.scroll_offset as u16;
    let col_num = current_pos.y.saturating_sub(Position::MARGIN - 1);

    // Middle-left: Stats
    let stats = format!(" Ln {}, Col {} ", line_num, col_num);
    queue!(stdout(), SetForegroundColor(Color::White), Print(stats),)?;

    // Middle: Lines and Characters count
    let counts = format!("Lines: {} | Chars: {} ", total_lines, total_chars);
    let counts_width = counts.len() as u16;
    let middle_pos = (size.width / 2).saturating_sub(counts_width / 2);
    queue!(
        stdout(),
        MoveTo(middle_pos, footer_row),
        SetBackgroundColor(Color::Black),
        SetForegroundColor(Color::White),
        Print(counts),
    )?;
    
    let credits = "© Filip Domanski";
    let credits_width = credits.len() as u16;
    let credits_pos = middle_pos + credits_width;
    queue!(
        stdout(),
        MoveTo(credits_pos, footer_row),
        SetBackgroundColor(Color::Black),
        SetForegroundColor(Color::Yellow),
        SetAttribute(Attribute::Bold),
        Print(format!(" {} ", credits)),
        SetAttribute(Attribute::Reset),
    )?;

    // Right side: Tab hint - show "Ctrl+1-9 for tabs" or actual tab info if we have tab_manager
    let hint = " Ctrl+1-9 for tabs | Ctrl+g for shortcuts ";
    let hint_width = hint.len() as u16;
    let hint_pos = size.width.saturating_sub(hint_width + 1);
    queue!(
        stdout(),
        MoveTo(hint_pos, footer_row),
        SetForegroundColor(Color::DarkYellow),
        SetAttribute(Attribute::Italic),
        Print(hint),
        SetAttribute(Attribute::Reset),
    )?;

    Ok(())
}

fn draw_shortcuts_footer() -> Result<(), Error> {
    use crate::core::shortcuts::Shortcuts;

    let size = Terminal::get_size()?;
    let footer_row = size.height - 1;

    queue!(
        stdout(),
        MoveTo(1, footer_row),
        SetBackgroundColor(Color::Black),
    )?;

    // Get shortcuts from Shortcuts module
    let shortcuts = Shortcuts::get_ctrl_shortcuts();

    let mut current_x = 1;
    for (i, (key, desc)) in shortcuts.iter().enumerate() {
        // Check if we have space
        let entry_width = key.len() + desc.len() + 4;
        if current_x + entry_width as u16 > size.width - 2 {
            break;
        }

        // Draw key in bold yellow
        queue!(
            stdout(),
            MoveTo(current_x, footer_row),
            SetForegroundColor(Color::DarkYellow),
            SetAttribute(Attribute::Bold),
            Print(format!("{}", key)),
            SetAttribute(Attribute::Reset),
        )?;
        current_x += key.len() as u16;

        // Draw description
        queue!(
            stdout(),
            MoveTo(current_x, footer_row),
            SetForegroundColor(Color::White),
            Print(format!(" {} ", desc)),
        )?;
        current_x += desc.len() as u16 + 1;

        // Add separator except for last item
        if i < shortcuts.len() - 1 {
            queue!(stdout(), SetForegroundColor(Color::DarkGrey), Print("│ "),)?;
            current_x += 2;
        }
    }

    Ok(())
}

// Render a prompt-style footer
fn draw_prompt_footer(view: &View, _caret: &Caret) -> Result<(), Error> {
    let size = Terminal::get_size()?;
    let footer_row = size.height - 1;

    // Clear and set background for the prompt area
    queue!(
        stdout(),
        MoveTo(0, footer_row),
        SetBackgroundColor(Color::Black),
    )?;
    Terminal::clear_rest_of_line()?;

    // Move to content start
    queue!(stdout(), MoveTo(1, footer_row))?;

    if let Some(prompt) = &view.prompt {
        // Use local paths for enum to avoid needing extra imports
        match &prompt.kind {
            super::PromptKind::SaveAs => {
                queue!(
                    stdout(),
                    SetForegroundColor(Color::DarkYellow),
                    SetAttribute(Attribute::Bold),
                    Print(format!("{}{}", prompt.message, prompt.input)),
                )?;
                draw_esc_hint(size.width, footer_row)?;
            }
            super::PromptKind::Search => {
                queue!(
                    stdout(),
                    SetForegroundColor(Color::DarkYellow),
                    SetAttribute(Attribute::Bold),
                    Print(format!(" {} ", prompt.message)),
                    SetAttribute(Attribute::Reset),
                    SetForegroundColor(Color::White),
                    Print(&prompt.input),
                )?;
                draw_esc_hint(size.width, footer_row)?;
            }
            super::PromptKind::SearchInfo => {
                queue!(
                    stdout(),
                    SetForegroundColor(Color::Green),
                    SetAttribute(Attribute::Bold),
                    Print(" 🔍 "),
                    SetAttribute(Attribute::Reset),
                    SetForegroundColor(Color::White),
                    Print(&prompt.message),
                )?;
                draw_esc_hint(size.width, footer_row)?;
            }
            super::PromptKind::Error => {
                queue!(
                    stdout(),
                    SetForegroundColor(Color::Red),
                    SetAttribute(Attribute::Bold),
                    Print(format!(" {} ", prompt.message)),
                    SetAttribute(Attribute::Reset),
                )?;
            }
        }
    }

    Ok(())
}

fn render_line_with_selection_and_syntax(
    line: &str, 
    line_idx: usize, 
    selection_range: Option<(TextPosition, TextPosition)>,
    highlighter: &SyntaxHighlighter,
) -> Result<(), Error> {
    // Get syntax tokens for this line
    let tokens = highlighter.highlight_line(line);
    
    if let Some((start, end)) = selection_range {
        let in_selection = line_idx >= start.line && line_idx <= end.line;

        if !in_selection {
            // No selection on this line - just render with syntax highlighting
            render_tokens(&tokens)?;
            return Ok(());
        }

        // Line has selection - need to handle both selection highlighting and syntax
        let chars: Vec<char> = line.chars().collect();
        let sel_start = if line_idx == start.line {
            start.column
        } else {
            0
        };
        let sel_end = if line_idx == end.line {
            end.column
        } else {
            chars.len()
        };

        render_tokens_with_selection(&tokens, sel_start, sel_end)?;
    } else {
        // No selection at all - just render with syntax highlighting
        render_tokens(&tokens)?;
    }

    Ok(())
}

// Helper: Render tokens with syntax highlighting (no selection)
fn render_tokens(tokens: &[crate::core::syntax::Token]) -> Result<(), Error> {
    for token in tokens {
        print_text_colored(&token.text, token.token_type.color())?;
    }
    Ok(())
}

// Helper: Render tokens with both syntax highlighting and selection
fn render_tokens_with_selection(
    tokens: &[crate::core::syntax::Token],
    sel_start: usize,
    sel_end: usize,
) -> Result<(), Error> {
    let mut char_pos = 0;
    
    for token in tokens {
        let token_len = token.text.chars().count();
        let token_end = char_pos + token_len;
        
        // Check if this token overlaps with selection
        if token_end <= sel_start || char_pos >= sel_end {
            // Token is completely outside selection
            print_text_colored(&token.text, token.token_type.color())?;
        } else if char_pos >= sel_start && token_end <= sel_end {
            // Token is completely inside selection
            print_text_selected(&token.text)?;
        } else {
            // Token is partially selected - need to split it
            render_token_partial_selection(token, char_pos, sel_start, sel_end)?;
        }
        
        char_pos = token_end;
    }
    
    Ok(())
}

// Helper: Render a token that's partially selected
fn render_token_partial_selection(
    token: &crate::core::syntax::Token,
    char_pos: usize,
    sel_start: usize,
    sel_end: usize,
) -> Result<(), Error> {
    let token_chars: Vec<char> = token.text.chars().collect();
    
    for (i, ch) in token_chars.iter().enumerate() {
        let abs_pos = char_pos + i;
        if abs_pos >= sel_start && abs_pos < sel_end {
            // Character is selected
            print_char_selected(*ch)?;
        } else {
            // Character is not selected - use syntax color
            print_char_colored(*ch, token.token_type.color())?;
        }
    }
    
    Ok(())
}

// Helper: Print text with a specific color
fn print_text_colored(text: &str, color: Color) -> Result<(), Error> {
    queue!(
        stdout(),
        SetForegroundColor(color),
        Print(text),
        ResetColor
    )?;
    Ok(())
}

// Helper: Print text with selection highlighting
fn print_text_selected(text: &str) -> Result<(), Error> {
    queue!(
        stdout(),
        SetBackgroundColor(Color::DarkBlue),
        SetForegroundColor(Color::White),
        Print(text),
        ResetColor
    )?;
    Ok(())
}

// Helper: Print a single character with a specific color
fn print_char_colored(ch: char, color: Color) -> Result<(), Error> {
    queue!(
        stdout(),
        SetForegroundColor(color),
        Print(ch),
        ResetColor
    )?;
    Ok(())
}

// Helper: Print a single character with selection highlighting
fn print_char_selected(ch: char) -> Result<(), Error> {
    queue!(
        stdout(),
        SetBackgroundColor(Color::DarkBlue),
        SetForegroundColor(Color::White),
        Print(ch),
        ResetColor
    )?;
    Ok(())
}

// Helper function to draw the Esc hint on the right side of the footer
fn draw_esc_hint(screen_width: u16, footer_row: u16) -> Result<(), Error> {
    let hint = " Press Esc to cancel ";
    let hint_width = hint.len() as u16;
    let hint_pos = screen_width.saturating_sub(hint_width + 1);
    queue!(
        stdout(),
        MoveTo(hint_pos, footer_row),
        SetForegroundColor(Color::DarkYellow),
        SetAttribute(Attribute::Italic),
        Print(hint),
        SetAttribute(Attribute::Reset),
    )?;
    Ok(())
}