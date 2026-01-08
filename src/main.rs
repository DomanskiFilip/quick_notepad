mod core;
mod tui;
mod gui;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    // Check if GUI mode is requested
    let gui_mode = args.iter().any(|arg| arg == "--gui");
    
    // Check for shortcuts flag (works in both modes)
    if args.iter().any(|arg| arg == "--shortcuts") {
        core::shortcuts::Shortcuts::print_all();
        return;
    }
    
    if gui_mode {
        // Launch GUI mode
        let file_path = args.iter()
            .position(|arg| arg != "--gui" && arg != &args[0])
            .map(|i| args[i].clone());
        
        gui::run(file_path);
    } else {
        // Launch TUI mode (existing code)
        let mut editor = if args.len() > 1 {
            let raw_path = &args[1];
            let path = std::fs::canonicalize(raw_path)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| raw_path.clone());
            
            match tui::TerminalEditor::new_with_file(&path) {
                Ok(mut ed) => {
                    ed.set_filename(path);
                    ed
                },
                Err(e) => {
                    eprintln!("Error opening file {}: {}", path, e);
                    eprintln!("Starting with empty editor instead");
                    tui::TerminalEditor::new(core::buffer::Buffer::default())
                }
            }
        } else {
            tui::TerminalEditor::new(core::buffer::Buffer::default())
        };
        
        editor.run();
    }
}