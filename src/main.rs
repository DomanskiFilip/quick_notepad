mod core;
mod tui;

use std::env;
use tui::TerminalEditor;
use core::shortcuts::Shortcuts;

fn main() {
    let args: Vec<String> = env::args().collect();
        
    // Check for flags
    if args.iter().any(|arg| arg == "--shortcuts") {
        Shortcuts::print_all();
        return;
    }
    
    // Create editor with or without file
    let mut editor = if args.len() > 1 {
        let raw_path = &args[1];
            // Convert to absolute path so the TabManager doesn't lose it later
            let path = std::fs::canonicalize(raw_path)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| raw_path.clone());
        
            match TerminalEditor::new_with_file(&path) {
                Ok(mut ed) => {
                    ed.set_filename(path);
                    ed
                },
                Err(e) => {
                    eprintln!("Error opening file {}: {}", path, e);
                    eprintln!("Starting with empty editor instead");
                    TerminalEditor::new(tui::view::Buffer::default())
                }
            }
        } else {
            // No file argument - load previous session or start fresh
            TerminalEditor::new(tui::view::Buffer::default())
        };
    
    editor.run();
}