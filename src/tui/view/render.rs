// render module responsible for all the render logic
use super::View;
use unicode_segmentation::UnicodeSegmentation;
use crate::core::graphemes::visual_width;
use crate::core::selection::TextPosition;
use crate::core::wrap::{self, RowPos};
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
    let max_width = size.width.saturating_sub(Position::MARGIN) as usize;

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

    // Walk visual rows from the viewport top; long lines take several rows
    let mut top = wrap::clamp_row(&view.buffer.lines, view.wrap_cols, view.top());
    let mut screen_row = 0;
    while screen_row < visible_rows {
        let line_idx = top.line;
        let Some(line) = view.buffer.lines.get(line_idx) else {
            clear_row(screen_row)?;
            screen_row += 1;
            continue;
        };

        let styled = style_line(line, line_idx, selection_range, &highlighter);
        let starts = wrap::row_starts(line, view.wrap_cols);

        for row in top.row..starts.len() {
            if screen_row >= visible_rows {
                break;
            }
            let terminal_row = clear_row(screen_row)?;
            if row == 0 && line_idx <= last_non_empty_line {
                draw_margin_line(terminal_row, line_idx)?;
            }
            queue!(stdout(), MoveTo(Position::MARGIN, terminal_row))?;
            let (start, end) = wrap::row_range(&starts, row, styled.len());
            print_styled(&styled[start..end], max_width)?;
            screen_row += 1;
        }

        top = RowPos {
            line: line_idx + 1,
            row: 0,
        };
    }

    draw_footer(view, caret, is_dirty)?;

    queue!(stdout(), MoveTo(current_pos.x, current_pos.y))?;
    Ok(())
}

fn clear_row(screen_row: usize) -> Result<u16, Error> {
    let terminal_row = screen_row as u16 + Position::HEADER;
    queue!(stdout(), MoveTo(0, terminal_row))?;
    Terminal::clear_rest_of_line()?;
    Ok(terminal_row)
}

// One grapheme of a line with the style it is drawn in
struct StyledGrapheme<'a> {
    text: &'a str,
    color: Color,
    selected: bool,
}

// Split a line into graphemes carrying their syntax colour and selection state
fn style_line<'a>(
    line: &'a str,
    line_idx: usize,
    selection_range: Option<(TextPosition, TextPosition)>,
    highlighter: &SyntaxHighlighter,
) -> Vec<StyledGrapheme<'a>> {
    // Byte range of each syntax token
    let mut token_ends: Vec<(usize, Color)> = Vec::new();
    let mut byte_end = 0;
    for token in highlighter.highlight_line(line) {
        byte_end += token.text.len();
        token_ends.push((byte_end, token.token_type.color()));
    }

    let (sel_start, sel_end) = match selection_range {
        Some((start, end)) if line_idx >= start.line && line_idx <= end.line => {
            let s = if line_idx == start.line { start.column } else { 0 };
            let e = if line_idx == end.line { end.column } else { usize::MAX };
            (s, e)
        }
        _ => (0, 0),
    };

    let mut token = 0;
    line.grapheme_indices(true)
        .enumerate()
        .map(|(i, (byte_idx, text))| {
            while token < token_ends.len() && token_ends[token].0 <= byte_idx {
                token += 1;
            }
            StyledGrapheme {
                text,
                color: token_ends.get(token).map_or(Color::Reset, |t| t.1),
                selected: i >= sel_start && i < sel_end,
            }
        })
        .collect()
}

// Print graphemes, batching runs of the same style, without going past max_width columns
fn print_styled(graphemes: &[StyledGrapheme], max_width: usize) -> Result<(), Error> {
    let mut width = 0;
    let mut run = String::new();
    let mut run_style: Option<(Color, bool)> = None;

    let flush = |run: &mut String, style: Option<(Color, bool)>| -> Result<(), Error> {
        match style {
            Some((_, true)) if !run.is_empty() => print_text_selected(run)?,
            Some((color, false)) if !run.is_empty() => print_text_colored(run, color)?,
            _ => {}
        }
        run.clear();
        Ok(())
    };

    for g in graphemes {
        let w = visual_width(g.text);
        if width + w > max_width {
            break;
        }
        width += w;
        let style = (g.color, g.selected);
        if run_style != Some(style) {
            flush(&mut run, run_style)?;
            run_style = Some(style);
        }
        run.push_str(g.text);
    }
    flush(&mut run, run_style)
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

    // Current position in the buffer (wrapped rows still belong to one line)
    let text_pos = super::helpers::get_current_text_pos(view, caret);
    let (line_num, col_num) = (text_pos.line + 1, text_pos.column + 1);

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
    // Alt+number works in every terminal, Ctrl+number only where the terminal supports it
    let hint = " Alt+1-9 for tabs | Ctrl+g for shortcuts ";
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
            Print(key),
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