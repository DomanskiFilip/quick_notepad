mod core;
mod tui;
mod gui;

use std::env;

fn main() {
    
    let args: Vec<String> = env::args().collect();
    let home = env::var("HOME").unwrap_or_default();
    let expected_path = format!("{}/.local/bin/quick", home);
    
    // Get current executable path
    let current_exe = env::current_exe().expect("Failed to get current path");

    // Check for uninstall flag first
    if args.iter().any(|arg| arg == "--uninstall") {
        uninstall();
        return;
    }

    // SELF-INSTALL LOGIC
    // If we are NOT running from ~/.local/bin and no specific flags are passed
    if !current_exe.to_string_lossy().contains(&expected_path) && args.len() == 1 {
        println!(" Quick Notepad: First-time setup detected...");
        install();
        
        // After installing, launch the GUI
        gui::run(None);
        return;
    }
    
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
            let path_buf = std::fs::canonicalize(raw_path).unwrap_or_else(|_| std::path::PathBuf::from(raw_path));
            
            // Extract full path for the backend
            let full_path = path_buf.to_string_lossy().into_owned();
            
            // Extract just the file name for the display/UI
            let display_name = path_buf
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| full_path.clone());
            
            let extension = path_buf.extension().map(|ext| ext.to_string_lossy().into_owned());
            let friendly_type = core::tabs::get_friendly_filetype(extension);
            
            // Open editor with selected file
            match tui::TerminalEditor::new_with_file(&full_path) {
                Ok(mut ed) => {
                    ed.set_filename_and_filetype(Some(display_name), friendly_type);
                    ed
                },
                Err(e) => {
                    eprintln!("Error opening file {}: {}", full_path, e);
                    tui::TerminalEditor::new(core::buffer::Buffer::default())
                }
            }
        } else {
            // Default to empty editor if no file specified
            tui::TerminalEditor::new(core::buffer::Buffer::default())
        };
        
        editor.run();
    }
    
    fn install() {
        use std::fs;
        use std::env;
        use std::path::Path;
    
        let home = env::var("HOME").expect("Could not find HOME directory");
        let bin_dir = format!("{}/.local/bin", home);
        let target_bin_path = format!("{}/quick", bin_dir);
        let icon_dir = format!("{}/.local/share/icons/hicolor/512x512/apps", home);
        let desktop_dir = format!("{}/.local/share/applications", home);
    
        // Move the binary to ~/.local/bin
        let current_exe = env::current_exe().expect("Failed to get current path");
        let _ = fs::create_dir_all(&bin_dir);
    
        // only copy if we aren't already the target
        if current_exe != Path::new(&target_bin_path) {
            if let Err(e) = fs::copy(&current_exe, &target_bin_path) {
                eprintln!("❌ Failed to move binary to bin: {}", e);
            } else {
                // Set executable permissions on the new copy
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = fs::set_permissions(&target_bin_path, fs::Permissions::from_mode(0o755));
                }
                println!(" Binary installed to ~/.local/bin");
            }
        }
    
        // Extract Baked-in Icon
        let icon_bytes = include_bytes!("../assets/icon.png");
        let _ = fs::create_dir_all(&icon_dir);
        let icon_path = format!("{}/quick_notepad.png", icon_dir);
        let _ = fs::write(&icon_path, icon_bytes);
    
        // Create Desktop Entry
        let _ = fs::create_dir_all(&desktop_dir);
        let desktop_entry = format!(
            "[Desktop Entry]\n\
            Name=Quick Notepad\n\
            Comment=Fast TUI/GUI Text Editor\n\
            Exec={bin} --gui %F\n\
            Icon=quick_notepad\n\
            Type=Application\n\
            Categories=Utility;TextEditor;\n\
            Terminal=false\n\
            MimeType=text/plain;\n",
            bin = target_bin_path
        );
    
        let _ = fs::write(format!("{}/quick-notepad.desktop", desktop_dir), desktop_entry);
        
        println!(" Desktop integration complete! You can now find Quick Notepad in your menu.");
    }

    fn uninstall() {
        use std::fs;
        use std::env;
        use std::io::{self, Write};

        println!("  Quick Notepad Uninstaller");
        println!("\n  This action cannot be undone!");
        
        print!("\nAre you sure you want to uninstall? (yes/no): ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim().to_lowercase();
        
        if input != "yes" && input != "y" {
            println!("Quick Uninstall cancelled.");
            return;
        }

        let home = match env::var("HOME") {
            Ok(h) => h,
            Err(_) => {
                eprintln!("Could not find HOME directory");
                return;
            }
        };

        let mut errors = Vec::new();
        let mut removed_count = 0;

        // Define all paths to remove
        let paths_to_remove = vec![
            // Binary
            (format!("{}/.local/bin/quick", home), "Binary"),
            // Backup binary (if exists)
            (format!("{}/.local/bin/quick.old", home), "Binary backup"),
            // Desktop entry
            (format!("{}/.local/share/applications/quick-notepad.desktop", home), "Desktop entry"),
            // Icon
            (format!("{}/.local/share/icons/hicolor/512x512/apps/quick_notepad.png", home), "Application icon"),
            // Session data (tabs.json)
            (format!("{}/.quicknotepad/tabs.json", home), "Session data"),
        ];

        println!("\n Removing files...");
        
        for (path, description) in paths_to_remove {
            match fs::remove_file(&path) {
                Ok(_) => {
                    println!("   Removed: {}", description);
                    removed_count += 1;
                }
                Err(e) => {
                    if e.kind() != std::io::ErrorKind::NotFound {
                        errors.push(format!("  ⚠️  Failed to remove {}: {}", description, e));
                    }
                    // Silently ignore if file doesn't exist
                }
            }
        }

        // Try to remove the .quicknotepad directory
        let config_dir = format!("{}/.quicknotepad", home);
        match fs::remove_dir(&config_dir) {
            Ok(_) => {
                println!("   Removed: Configuration directory");
                removed_count += 1;
            }
            Err(e) => {
                if e.kind() != std::io::ErrorKind::NotFound {
                    // Only warn if directory exists but couldn't be removed (might have other files)
                    if e.kind() == std::io::ErrorKind::Other {
                        println!("  ℹ️  Note: Configuration directory not removed (may contain other files)");
                    } else {
                        errors.push(format!("  ⚠️  Failed to remove configuration directory: {}", e));
                    }
                }
            }
        }

        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        
        if !errors.is_empty() {
            println!("\n⚠️  Warnings:");
            for error in errors {
                println!("{}", error);
            }
        }

        println!("\n   Uninstall complete!");
        println!("   {} item(s) removed", removed_count);
        
        // Update desktop database if available
        if let Ok(status) = std::process::Command::new("update-desktop-database")
            .arg(format!("{}/.local/share/applications", home))
            .status()
        {
            if status.success() {
                println!("   Desktop database updated");
            }
        }

        println!("\n   Thank you for using Quick Notepad!");
    }
}