// shortcuts module to handle key mappings
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind, MouseButton};
use crate::core::actions::Action;

pub struct Shortcuts {
    last_click_time: std::time::Instant,
    last_click_pos: Option<(u16, u16)>,
    click_count: u8,
}

impl Shortcuts {
    pub fn new() -> Self {
        Self {
            last_click_time: std::time::Instant::now(),
            last_click_pos: None,
            click_count: 0,
        }
    }

    // THE ONE PLACE TO CHANGE SHORTCUTS
    // Format: (KeyCode, Modifiers, Action, Description)
    fn get_mappings() -> Vec<(KeyCode, KeyModifiers, Action, &'static str)> {
        vec![
            (KeyCode::Left, KeyModifiers::empty(), Action::Left, "Move caret left"),
            (KeyCode::Right, KeyModifiers::empty(), Action::Right, "Move caret right"),
            (KeyCode::Up, KeyModifiers::empty(), Action::Up, "Move caret up"),
            (KeyCode::Down, KeyModifiers::empty(), Action::Down, "Move caret down"),
            (KeyCode::PageUp, KeyModifiers::empty(), Action::Top, "Move to top of view"),
            (KeyCode::PageDown, KeyModifiers::empty(), Action::Bottom, "Move to bottom of view"),
            (KeyCode::Home, KeyModifiers::empty(), Action::MaxLeft, "Move to start of line"),
            (KeyCode::End, KeyModifiers::empty(), Action::MaxRight, "Move to end of line"),
            (KeyCode::Enter, KeyModifiers::empty(), Action::NextLine, "Insert new line"),
            (KeyCode::Tab, KeyModifiers::empty(), Action::Print, "Insert tab (4 spaces)"),
            (KeyCode::Backspace, KeyModifiers::empty(), Action::Backspace, "Delete before cursor"),
            (KeyCode::Delete, KeyModifiers::empty(), Action::Delete, "Delete at cursor"),
            (KeyCode::Char('g'), KeyModifiers::CONTROL, Action::ToggleCtrlShortcuts, "Toggle ctrl shortcuts footer"),
            (KeyCode::Char('q'), KeyModifiers::CONTROL, Action::Quit, "Quit"),
            (KeyCode::Char('s'), KeyModifiers::CONTROL, Action::Save, "Save"),
            (KeyCode::Char('n'), KeyModifiers::CONTROL, Action::New, "New"),
            (KeyCode::Char('c'), KeyModifiers::CONTROL, Action::Copy, "Copy"),
            (KeyCode::Char('v'), KeyModifiers::CONTROL, Action::Paste, "Paste"),
            (KeyCode::Char('x'), KeyModifiers::CONTROL, Action::Cut, "Cut"),
            (KeyCode::Char('a'), KeyModifiers::CONTROL, Action::SelectAll, "Select all"),
            (KeyCode::Char('z'), KeyModifiers::CONTROL, Action::Undo, "Undo"),
            (KeyCode::Char('y'), KeyModifiers::CONTROL, Action::Redo, "Redo"),
            (KeyCode::Char('f'), KeyModifiers::CONTROL, Action::Search, "Search"),
            (KeyCode::Char('u'), KeyModifiers::CONTROL, Action::CheckUpdate, "Check for updates"),
        ]
    }
        
    pub fn resolve(&mut self, event: &KeyEvent) -> Option<Action> {
        match (event.code, event.modifiers) {
            // Movement with Shift = Selection
            (KeyCode::Left, m) if m.contains(KeyModifiers::SHIFT) => Some(Action::SelectLeft),
            (KeyCode::Right, m) if m.contains(KeyModifiers::SHIFT) => Some(Action::SelectRight),
            (KeyCode::Up, m) if m.contains(KeyModifiers::SHIFT) => Some(Action::SelectUp),
            (KeyCode::Down, m) if m.contains(KeyModifiers::SHIFT) => Some(Action::SelectDown),
            (KeyCode::PageUp, m) if m.contains(KeyModifiers::SHIFT) => Some(Action::SelectTop),
            (KeyCode::PageDown, m) if m.contains(KeyModifiers::SHIFT) => Some(Action::SelectBottom),
            (KeyCode::Home, m) if m.contains(KeyModifiers::SHIFT) => Some(Action::SelectMaxLeft),
            (KeyCode::End, m) if m.contains(KeyModifiers::SHIFT) => Some(Action::SelectMaxRight),
            
            // Regular movement (clears selection)
            (KeyCode::Left, _) => Some(Action::Left),
            (KeyCode::Right, _) => Some(Action::Right),
            (KeyCode::Up, _) => Some(Action::Up),
            (KeyCode::Down, _) => Some(Action::Down),
            (KeyCode::PageUp, _) => Some(Action::Top),
            (KeyCode::PageDown, _) => Some(Action::Bottom),
            (KeyCode::Home, _) => Some(Action::MaxLeft),
            (KeyCode::End, _) => Some(Action::MaxRight),
            
            (KeyCode::Enter, _) => Some(Action::NextLine),
            (KeyCode::Tab, _) => Some(Action::Print),  // Tab inserts 4 spaces
            (KeyCode::Backspace, _) => Some(Action::Backspace),
            (KeyCode::Delete, _) => Some(Action::Delete),
            (KeyCode::Char('g'), KeyModifiers::CONTROL) => Some(Action::ToggleCtrlShortcuts),
            (KeyCode::Char('s'), KeyModifiers::CONTROL) => Some(Action::Save),
            (KeyCode::Char('n'), KeyModifiers::CONTROL) => Some(Action::New),
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => Some(Action::Quit),
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(Action::Copy),
            (KeyCode::Char('v'), KeyModifiers::CONTROL) => Some(Action::Paste),
            (KeyCode::Char('x'), KeyModifiers::CONTROL) => Some(Action::Cut),
            (KeyCode::Char('a'), KeyModifiers::CONTROL) => Some(Action::SelectAll),
            (KeyCode::Char('z'), KeyModifiers::CONTROL) => Some(Action::Undo),
            (KeyCode::Char('y'), KeyModifiers::CONTROL) => Some(Action::Redo),
            (KeyCode::Char('f'), KeyModifiers::CONTROL) => Some(Action::Search),
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => Some(Action::CheckUpdate),
            // Tab switching - Ctrl+Number (existing)
            (KeyCode::Char('1'), KeyModifiers::CONTROL) => Some(Action::SwitchTab(1)),
            (KeyCode::Char('2'), KeyModifiers::CONTROL) => Some(Action::SwitchTab(2)),
            (KeyCode::Char('3'), KeyModifiers::CONTROL) => Some(Action::SwitchTab(3)),
            (KeyCode::Char('4'), KeyModifiers::CONTROL) => Some(Action::SwitchTab(4)),
            (KeyCode::Char('5'), KeyModifiers::CONTROL) => Some(Action::SwitchTab(5)),
            (KeyCode::Char('6'), KeyModifiers::CONTROL) => Some(Action::SwitchTab(6)),
            (KeyCode::Char('7'), KeyModifiers::CONTROL) => Some(Action::SwitchTab(7)),
            (KeyCode::Char('8'), KeyModifiers::CONTROL) => Some(Action::SwitchTab(8)),
            (KeyCode::Char('9'), KeyModifiers::CONTROL) => Some(Action::SwitchTab(9)),
            (KeyCode::Char('0'), KeyModifiers::CONTROL) => Some(Action::SwitchTab(10)),
            // Tab switching - Alt+Number (alternative)
            (KeyCode::Char('1'), m) if m.contains(KeyModifiers::ALT) => Some(Action::SwitchTab(1)),
            (KeyCode::Char('2'), m) if m.contains(KeyModifiers::ALT) => Some(Action::SwitchTab(2)),
            (KeyCode::Char('3'), m) if m.contains(KeyModifiers::ALT) => Some(Action::SwitchTab(3)),
            (KeyCode::Char('4'), m) if m.contains(KeyModifiers::ALT) => Some(Action::SwitchTab(4)),
            (KeyCode::Char('5'), m) if m.contains(KeyModifiers::ALT) => Some(Action::SwitchTab(5)),
            (KeyCode::Char('6'), m) if m.contains(KeyModifiers::ALT) => Some(Action::SwitchTab(6)),
            (KeyCode::Char('7'), m) if m.contains(KeyModifiers::ALT) => Some(Action::SwitchTab(7)),
            (KeyCode::Char('8'), m) if m.contains(KeyModifiers::ALT) => Some(Action::SwitchTab(8)),
            (KeyCode::Char('9'), m) if m.contains(KeyModifiers::ALT) => Some(Action::SwitchTab(9)),
            (KeyCode::Char('0'), m) if m.contains(KeyModifiers::ALT) => Some(Action::SwitchTab(10)),
            // Tab switching - F1..F10 (no modifier)
            (KeyCode::F(1), _) => Some(Action::SwitchTab(1)),
            (KeyCode::F(2), _) => Some(Action::SwitchTab(2)),
            (KeyCode::F(3), _) => Some(Action::SwitchTab(3)),
            (KeyCode::F(4), _) => Some(Action::SwitchTab(4)),
            (KeyCode::F(5), _) => Some(Action::SwitchTab(5)),
            (KeyCode::F(6), _) => Some(Action::SwitchTab(6)),
            (KeyCode::F(7), _) => Some(Action::SwitchTab(7)),
            (KeyCode::F(8), _) => Some(Action::SwitchTab(8)),
            (KeyCode::F(9), _) => Some(Action::SwitchTab(9)),
            (KeyCode::F(10), _) => Some(Action::SwitchTab(10)),
            (KeyCode::Char(_c), m) if m.is_empty() || m == KeyModifiers::SHIFT => {
                Some(Action::Print)
            }
            _ => None,
        }
    }
    
    pub fn resolve_mouse(&mut self, event: &MouseEvent) -> Option<Action> {
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let now = std::time::Instant::now();
                let pos = (event.column, event.row);
                
                // Detect double/triple click (within 500ms and same position)
                if let Some(last_pos) = self.last_click_pos {
                    if now.duration_since(self.last_click_time).as_millis() < 500 
                        && last_pos == pos {
                        self.click_count += 1;
                    } else {
                        self.click_count = 1;
                    }
                } else {
                    self.click_count = 1;
                }
                
                self.last_click_time = now;
                self.last_click_pos = Some(pos);
                
                match self.click_count {
                    2 => Some(Action::MouseDoubleClick(event.column, event.row)),
                    3 => {
                        self.click_count = 0; // Reset after triple
                        Some(Action::MouseTripleClick(event.column, event.row))
                    },
                    _ => Some(Action::MouseDown(event.column, event.row)),
                }
            },
            MouseEventKind::Drag(MouseButton::Left) => {
                Some(Action::MouseDrag(event.column, event.row))
            }
            MouseEventKind::Up(MouseButton::Left) => Some(Action::MouseUp(event.column, event.row)),
            _ => None,
        }
    }
    
    // Returns Ctrl shortcuts for footer display
    pub fn get_ctrl_shortcuts() -> Vec<(String, &'static str)> {
        Self::get_mappings()
            .into_iter()
            .filter(|(_, mods, _, _)| mods.contains(KeyModifiers::CONTROL))
            .map(|(code, mods, _, desc)| (Self::key_to_short_string(code, mods), desc))
            .collect()
    }
    
    // print all shortcuts
    pub fn print_all() {        
        for (code, mods, _, desc) in Self::get_mappings() {
            println!("  {:<15} : {}", Self::key_to_string(code, mods), desc);
        }
    }
    
    // used to display shortcuts with flag --shortcuts
    fn key_to_string(code: KeyCode, mods: KeyModifiers) -> String {
        let mut string = String::new();
        if mods.contains(KeyModifiers::CONTROL) { string.push_str("Ctrl+"); }
        if mods.contains(KeyModifiers::ALT) { string.push_str("Alt+"); }
        if mods.contains(KeyModifiers::SHIFT) { string.push_str("Shift+"); }
        match code {
            KeyCode::Char(character) => string.push(character.to_ascii_uppercase()),
            KeyCode::Left => string.push_str("Left Arrow"),
            KeyCode::Right => string.push_str("Right Arrow"),
            KeyCode::Up => string.push_str("Up Arrow"),
            KeyCode::Down => string.push_str("Down Arrow"),
            KeyCode::Enter => string.push_str("Enter"),
            KeyCode::Tab => string.push_str("Tab"),
            KeyCode::Backspace => string.push_str("Backspace"),
            KeyCode::Delete => string.push_str("Delete"),
            KeyCode::Home => string.push_str("Home"),
            KeyCode::End => string.push_str("End"),
            KeyCode::PageUp => string.push_str("Page Up"),
            KeyCode::PageDown => string.push_str("Page Down"),
            _ => string.push_str("Unknown"),
        }
        string
    }
    
    // Short format for footer display (^Q, ^S, etc.)
    fn key_to_short_string(code: KeyCode, mods: KeyModifiers) -> String {
        let mut string = String::new();
        if mods.contains(KeyModifiers::CONTROL) { string.push('^'); }
        if mods.contains(KeyModifiers::ALT) { string.push_str("A-"); }
        if mods.contains(KeyModifiers::SHIFT) { string.push_str("S-"); }
        match code {
            KeyCode::Char(character) => string.push(character.to_ascii_uppercase()),
            _ => string.push_str("?"),
        }
        string
    }
}