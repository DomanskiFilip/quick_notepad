mod core;
mod tui;
mod gui;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    // Flags parsed up front
    let want_uninstall  = args.iter().any(|a| a == "--uninstall");
    let want_shortcuts  = args.iter().any(|a| a == "--shortcuts");
    let want_gui        = args.iter().any(|a| a == "--gui");

    if want_uninstall {
        uninstall();
        return;
    }

    if want_shortcuts {
        core::shortcuts::Shortcuts::print_all();
        return;
    }

    // Self-install guard
    //
    // Only fire when ALL of the following are true:
    //   1. No file/flag arguments (bare `quick` invocation)
    //   2. Not running as root / via sudo (those sessions have no user display)
    //   3. Not already running from the canonical install path
    //   4. Not running from a system prefix (/usr/, /opt/, …)
    //   5. The canonical install path does not yet exist on disk
    if args.len() == 1 && !want_gui && !is_elevated() {
        if let Some(true) = should_self_install() {
            println!(" Quick Notepad: First-time setup detected...");
            install();
            gui::run(None);
            return;
        }
    }

    if want_gui {
        // First non-flag argument after argv[0] is the optional file path
        let file_path = args.iter()
            .skip(1)
            .find(|a| !a.starts_with("--"))
            .cloned();
        gui::run(file_path);
    } else {
        // TUI mode
        let file_arg = args.iter()
            .skip(1)
            .find(|a| !a.starts_with("--"))
            .map(String::as_str);

        let mut editor = match file_arg {
            Some(raw_path) => {
                // Resolve the path — but do NOT use canonicalize() if the file
                // might not exist yet (e.g. `quick newfile.txt`)
                let path_buf = std::fs::canonicalize(raw_path)
                    .unwrap_or_else(|_| std::path::PathBuf::from(raw_path));
                let full_path = path_buf.to_string_lossy().into_owned();

                match tui::TerminalEditor::open_file(&full_path) {
                    Ok(ed) => ed,
                    Err(e) => {
                        eprintln!("Error opening file {}: {}", full_path, e);
                        tui::TerminalEditor::open_fresh()
                    }
                }
            }
            None => tui::TerminalEditor::open_fresh(),
        };

        editor.run();
    }
}

// Detect elevated / root execution without external crates
//
// Checks (in order of reliability):
//   1. $SUDO_USER is set  → we were launched via `sudo`
//   2. $USER == "root"    → running as the root account directly
//   3. $HOME == "/root"   → home directory is root's home
//   4. $EUID == "0"       → effective UID env var (set by some shells)
fn is_elevated() -> bool {
    // `sudo quick` always sets SUDO_USER to the original user
    if env::var("SUDO_USER").is_ok() {
        return true;
    }
    if env::var("USER").as_deref() == Ok("root") {
        return true;
    }
    if env::var("HOME").as_deref() == Ok("/root") {
        return true;
    }
    if env::var("EUID").as_deref() == Ok("0") {
        return true;
    }
    false
}

// Decide whether to self-install.
//   Some(true)  → should install
//   Some(false) → already installed / should not install
//   None        → can't determine, skip install
fn should_self_install() -> Option<bool> {
    let home = env::var("HOME").ok()?;
    let canonical = format!("{}/.local/bin/quick", home);

    let current_exe = env::current_exe().ok()?;
    let current_str = current_exe.to_string_lossy();

    // Already running from the install path
    if current_str.as_ref() == canonical {
        return Some(false);
    }

    // Running from a system-wide location (package manager, live medium, etc.)
    let system_prefixes = ["/usr/", "/opt/", "/bin/", "/sbin/", "/snap/",
                           "/run/", "/mnt/", "/media/", "/live/"];
    if system_prefixes.iter().any(|p| current_str.starts_with(p)) {
        return Some(false);
    }

    // Canonical path already exists — already installed, no need to reinstall
    if std::path::Path::new(&canonical).exists() {
        return Some(false);
    }

    Some(true)
}

fn install() {
    use std::fs;
    use std::path::Path;

    let home = match env::var("HOME") {
        Ok(h) => h,
        Err(_) => {
            eprintln!("❌ Cannot install: $HOME is not set.");
            return;
        }
    };

    let bin_dir      = format!("{}/.local/bin", home);
    let target_path  = format!("{}/quick", bin_dir);
    let icon_dir     = format!("{}/.local/share/icons/hicolor/512x512/apps", home);
    let desktop_dir  = format!("{}/.local/share/applications", home);

    let current_exe = match env::current_exe() {
        Ok(p) => p,
        Err(e) => { eprintln!("❌ Cannot determine executable path: {}", e); return; }
    };

    let _ = fs::create_dir_all(&bin_dir);

    if current_exe != Path::new(&target_path) {
        match fs::copy(&current_exe, &target_path) {
            Ok(_) => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = fs::set_permissions(&target_path,
                        fs::Permissions::from_mode(0o755));
                }
                println!("✓ Binary installed to {}", target_path);
            }
            Err(e) => eprintln!("❌ Failed to install binary: {}", e),
        }
    }

    // Add ~/.local/bin to PATH in shell rc files if it isn't already there.
    // This is the fix for "command not found" on fresh Arch installs.
    add_to_path_if_needed(&bin_dir, &home);

    create_system_symlink(&target_path);

    // Icon
    let icon_bytes = include_bytes!("../assets/icon.png");
    let _ = fs::create_dir_all(&icon_dir);
    let _ = fs::write(format!("{}/quick_notepad.png", icon_dir), icon_bytes);

    // Desktop entry
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
        bin = target_path
    );
    let _ = fs::write(format!("{}/quick-notepad.desktop", desktop_dir), desktop_entry);

    // Attempt to refresh the desktop database (non-fatal if missing)
    let _ = std::process::Command::new("update-desktop-database")
        .arg(&desktop_dir)
        .status();

    println!("✓ Desktop integration complete. You can now find Quick Notepad in your menu.");
    println!("\nℹ  If 'quick' is not found in a new terminal, run:\n");
    println!("    source ~/.bashrc   (or ~/.zshrc / ~/.profile)");
    println!("\n   or open a new terminal session.\n");
}

fn create_system_symlink(target_path: &str) {
    let system_link = "/usr/local/bin/quick";

    let _ = std::fs::remove_file(system_link);

    #[cfg(unix)]
    if std::os::unix::fs::symlink(target_path, system_link).is_ok() {
        println!("✓ Symlinked {} → {} (enables 'sudo quick')", system_link, target_path);
        return;
    }

    // sudo/root compatibility
    let pkexec_ok = std::process::Command::new("pkexec")
        .args(["ln", "-sf", target_path, system_link])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if pkexec_ok {
        println!("✓ Symlinked {} → {} via pkexec (enables 'sudo quick')", system_link, target_path);
        return;
    }

    let sudo_ok = std::process::Command::new("sudo")
        .args(["-A", "ln", "-sf", target_path, system_link])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if sudo_ok {
        println!("✓ Symlinked {} → {} via sudo (enables 'sudo quick')", system_link, target_path);
        return;
    }

    println!(
        "ℹ  One extra step needed to enable 'sudo quick':

            sudo ln -sf {} {}

            (this only needs to be done once)",
        target_path, system_link
    );
}

fn add_to_path_if_needed(bin_dir: &str, home: &str) {
    use std::fs;
    use std::io::Write;

    let export_line = format!("\nexport PATH=\"{}:$PATH\"\n", bin_dir);

    // Shell rc files to check, in preference order
    let rc_files = [
        format!("{}/.bashrc", home),
        format!("{}/.zshrc", home),
        format!("{}/.profile", home),
    ];

    let mut added = false;
    for rc_path in &rc_files {
        // Read the current content; if the file doesn't exist, treat as empty.
        let content = fs::read_to_string(rc_path).unwrap_or_default();

        // Skip if the bin_dir is already mentioned in this file
        if content.contains(bin_dir) {
            return; // Already configured — nothing to do globally
        }

        // Append to ~/.bashrc (primary) and note for the user
        if !added {
            match fs::OpenOptions::new().append(true).create(true).open(rc_path) {
                Ok(mut f) => {
                    if f.write_all(export_line.as_bytes()).is_ok() {
                        println!("✓ Added {} to PATH in {}", bin_dir, rc_path);
                        added = true;
                    }
                }
                Err(e) => eprintln!("⚠  Could not update {}: {}", rc_path, e),
            }
        }
    }
}

fn uninstall() {
    use std::fs;
    use std::io::{self, Write};

    println!("  Quick Notepad Uninstaller");
    println!("\n  This action cannot be undone!");
    print!("\nAre you sure you want to uninstall? (yes/no): ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    if !matches!(input.trim().to_lowercase().as_str(), "yes" | "y") {
        println!("Uninstall cancelled.");
        return;
    }

    let home = match env::var("HOME") {
        Ok(h) => h,
        Err(_) => { eprintln!("Could not find HOME directory"); return; }
    };

    let paths = vec![
        (format!("{}/.local/bin/quick", home),                                          "Binary"),
        (format!("{}/.local/bin/quick.old", home),                                      "Binary backup"),
        (format!("{}/.local/share/applications/quick-notepad.desktop", home),           "Desktop entry"),
        (format!("{}/.local/share/icons/hicolor/512x512/apps/quick_notepad.png", home), "Icon"),
        (format!("{}/.quicknotepad/tabs.json", home),                                   "Session data"),
    ];

    println!("\n Removing files...");
    let mut removed = 0;
    let mut errors: Vec<String> = Vec::new();

    for (path, desc) in &paths {
        match fs::remove_file(path) {
            Ok(_) => { println!("   Removed: {}", desc); removed += 1; }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => errors.push(format!("Failed to remove {}: {}", desc, e)),
        }
    }

    let config_dir = format!("{}/.quicknotepad", home);
    match fs::remove_dir(&config_dir) {
        Ok(_) => { println!("   Removed: Configuration directory"); removed += 1; }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => println!("  ℹ Configuration directory kept (may contain other files)"),
    }

    if !errors.is_empty() {
        println!("\n⚠  Warnings:");
        for e in &errors { println!("  {}", e); }
    }

    println!("\n   Uninstall complete! {} item(s) removed.", removed);

    let _ = std::process::Command::new("update-desktop-database")
        .arg(format!("{}/.local/share/applications", home))
        .status();

    println!("   Thank you for using Quick Notepad!");
}