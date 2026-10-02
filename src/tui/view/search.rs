// search module for text search functionality
use super::View;
use crate::tui::{
    caret::{Caret, Position},
    terminal::Terminal,
};
use crate::core::selection::{Selection, TextPosition};
use crate::core::search::{find_all_occurrences, find_closest_match, SearchState};
use crossterm::event::{Event, KeyCode, KeyEventKind, read};
use std::io::Error;

pub fn search(view: &mut View, caret: &mut Caret) -> Result<(), Error> {
    // Show search prompt
    view.show_prompt(
        super::PromptKind::Search,
        "Search:".to_string(),
    );
    view.needs_redraw = true;
    view.render_if_needed(caret, false)?;
    Terminal::execute()?;

    let mut search_query = String::new();

    // Capture input for search query
    loop {
        match read()? {
            Event::Key(event) if event.kind == KeyEventKind::Press => {
                match event.code {
                    KeyCode::Char(c) => {
                        search_query.push(c);
                        view.append_prompt_char(c);
                        view.render_if_needed(caret, false)?;
                        Terminal::execute()?;
                    }
                    KeyCode::Backspace => {
                        search_query.pop();
                        view.backspace_prompt();
                        view.render_if_needed(caret, false)?;
                        Terminal::execute()?;
                    }
                    KeyCode::Enter => {
                        view.clear_prompt();
                        if !search_query.is_empty() {
                            perform_search(view, caret, &search_query)?;
                        }
                        break;
                    }
                    KeyCode::Esc => {
                        view.clear_prompt();
                        view.render_if_needed(caret, false)?;
                        Terminal::execute()?;
                        break;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn perform_search(view: &mut View, caret: &mut Caret, query: &str) -> Result<(), Error> {
    if query.is_empty() {
        return Ok(());
    }

    // Find all occurrences
    let matches = find_all_occurrences(&view.buffer.lines, query);

    if matches.is_empty() {
        // No match found - show error in prompt
        view.show_prompt(
            super::PromptKind::Error,
            format!("No matches found for '{}'", query),
        );
        view.render_if_needed(caret, false)?;
        Terminal::execute()?;
        
        std::thread::sleep(std::time::Duration::from_secs(2));
        view.clear_prompt();
        view.render_if_needed(caret, false)?;
        Terminal::execute()?;
        return Ok(());
    }

    // Find the match closest to current cursor position
    let current_pos = caret.get_position();
    let current_line = (current_pos.y.saturating_sub(Position::HEADER)) as usize + view.scroll_offset;
    let current_col = (current_pos.x as usize).saturating_sub(Position::MARGIN as usize);
    
    let closest_idx = find_closest_match(&matches, current_line, current_col);

    // Store search state in view
    let search_state = SearchState::new(query.to_string(), matches);
    view.set_search_state(Some(search_state));
    view.set_current_match(closest_idx);

    // Move to first match
    move_to_current_match(view, caret)?;

    Ok(())
}

fn move_to_current_match(view: &mut View, caret: &mut Caret) -> Result<(), Error> {
    if let Some(search_state) = &view.search_state {
        if let Some(m) = search_state.current_match() {
            let size = Terminal::get_size()?;
            let visible_rows = size.height.saturating_sub(Position::HEADER + 1) as usize;

            // Adjust scroll to show the match
            if m.line < view.scroll_offset {
                view.scroll_offset = m.line;
            } else if m.line >= view.scroll_offset + visible_rows {
                view.scroll_offset = m.line.saturating_sub(visible_rows / 2);
            }

            // Create selection for current match
            let start_pos = TextPosition {
                line: m.line,
                column: m.column,
            };
            let end_pos = TextPosition {
                line: m.line,
                column: m.column + m.length,
            };

            view.selection = Some(Selection {
                anchor: start_pos,
                cursor: end_pos,
            });

            // Update footer to show match info
            let total = search_state.matches.len();
            let current = search_state.current_match_idx + 1;
            view.show_prompt(
                super::PromptKind::SearchInfo,
                format!("Match {} of {} | ↑/↓ to navigate", current, total),
            );

            // Move caret to end of match
            let (screen_x, screen_y) = super::helpers::text_to_screen_pos(view, end_pos);
            
            view.needs_redraw = true;
            view.render(caret)?;
            caret.move_to(Position { x: screen_x, y: screen_y })?;
            Terminal::execute()?;
        }
    }

    Ok(())
}

pub fn next_search_match(view: &mut View, caret: &mut Caret) -> Result<(), Error> {
    if let Some(search_state) = &mut view.search_state {
        search_state.next_match();
        move_to_current_match(view, caret)?;
    }
    Ok(())
}

pub fn prev_search_match(view: &mut View, caret: &mut Caret) -> Result<(), Error> {
    if let Some(search_state) = &mut view.search_state {
        search_state.prev_match();
        move_to_current_match(view, caret)?;
    }
    Ok(())
}

pub fn clear_search(view: &mut View) {
    view.search_state = None;
    view.selection = None;
    view.clear_prompt();
    view.needs_redraw = true;
}