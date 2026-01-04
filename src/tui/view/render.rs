// render module responsible for all the render logic
use super::View;
use crate::core::selection::TextPosition;
use crate::tui::{
    caret::{Caret, Position},
    terminal::Terminal,
};
use crossterm::{
    cursor::MoveTo,
    queue,
    style::{
        Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
    },
};
use std::io::{Error, stdout};

pub fn render_view(view: &View, caret: &Caret, is_dirty: bool) -> Result<(), Error> {
    let current_pos = caret.get_position();
    let size = Terminal::get_size()?;

    draw_header()?;

    // Visible rows = Total height - Header - Footer
    let visible_rows = (size.height.saturating_sub(Position::HEADER + 1)) as usize;

    // Find last line with text for dynamic margin
    let last_non_empty_line = view
        .buffer
        .lines
        .iter()
        .rposition(|line| !line.is_empty())
        .unwrap_or(0);

    // Get selection range if active
    let selection_range = view
        .selection
        .as_ref()
        .filter(|s| s.is_active())
        .map(|s| s.get_range());

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
            let truncated_line = if line.len() > max_width {
                &line[..max_width]
            } else {
                line
            };

            render_line_with_selection(truncated_line, buffer_line_idx, selection_range)?;
        }
    }

    draw_footer(view, caret, is_dirty)?;

    // Restore cursor position
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

    let credits = "© Filip Domanski";
    queue!(
        stdout(),
        SetBackgroundColor(Color::Black),
        SetForegroundColor(Color::Yellow),
        SetAttribute(Attribute::Bold),
        Print(format!(" {} ", credits)),
        SetAttribute(Attribute::Reset),
    )?;

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

    // Right side: Help hint
    let hint = " Press Ctrl + g for shortcuts ";
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

    if let Some(p) = &view.prompt {
        // Use local paths for enum to avoid needing extra imports
        match &p.kind {
            super::PromptKind::SaveAs => {
                // Show label and current input (input may be empty)
                queue!(
                    stdout(),
                    SetForegroundColor(Color::White),
                    Print(format!(" {} {}", p.message, p.input)),
                )?;
            }
            super::PromptKind::Error => {
                // Show error message in red bold
                queue!(
                    stdout(),
                    SetForegroundColor(Color::Red),
                    SetAttribute(Attribute::Bold),
                    Print(format!(" {} ", p.message)),
                    SetAttribute(Attribute::Reset),
                )?;
            }
            _ => {
                // Generic message
                queue!(
                    stdout(),
                    SetForegroundColor(Color::White),
                    Print(format!(" {} ", p.message)),
                )?;
            }
        }
    }

    Ok(())
}

fn render_line_with_selection(
    line: &str,
    line_idx: usize,
    selection_range: Option<(TextPosition, TextPosition)>,
) -> Result<(), Error> {
    if let Some((start, end)) = selection_range {
        let in_selection = line_idx >= start.line && line_idx <= end.line;

        if !in_selection {
            print_text(line)?;
            return Ok(());
        }

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

        // Render before selection
        if sel_start > 0 {
            let before: String = chars[..sel_start].iter().collect();
            print_text(&before)?;
        }

        // Render selection with highlight
        if sel_start < chars.len() && sel_end > sel_start {
            let selected: String = chars[sel_start..sel_end.min(chars.len())].iter().collect();
            queue!(
                stdout(),
                SetBackgroundColor(Color::DarkBlue),
                SetForegroundColor(Color::White),
                Print(selected),
                ResetColor
            )?;
        }

        // Render after selection
        if sel_end < chars.len() {
            let after: String = chars[sel_end..].iter().collect();
            print_text(&after)?;
        }
    } else {
        print_text(line)?;
    }

    Ok(())
}

pub fn print_text(text: &str) -> Result<(), Error> {
    queue!(stdout(), Print(text))?;
    Ok(())
}
