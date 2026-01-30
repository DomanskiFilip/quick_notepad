## Quick Notepad
A modern, dual-mode text editor written in Rust that runs both in the terminal (TUI) and as a native GUI application. Built from scratch as part of the "build your own X" philosophy to create a fast, reliable, and customizable editor that does exactly what you need.

![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)

# Features
## Core Functionality

 - Dual Mode: Run in terminal (TUI) or as native GUI
 
 - Full Unicode Support: Proper handling of graphemes, emojis, and multi-byte characters
 
 - Syntax Highlighting: 25+ languages with custom theme support
 
 - Multi-Tab Interface: Up to 10 tabs with session persistence
 
 - Auto-Save Sessions: Never lose your work
 
 - Search & Navigation: Fast text search with match highlighting
 
 - Undo/Redo: Full edit history with intelligent grouping

 - Mouse Support: Click, drag, double-click, triple-click selection
 
 - Wayland Clipboard Integration: Works seamlessly with system clipboard
 
 - Configurable Shortcuts: All shortcuts in one place (for now you need to change in: src/core/shortcuts.rs)

## Technical Highlights

 - Fast Rendering: Optimized for minimal redraws
 
 - Clean Architecture: Modular design across 25+ files and 6 packages
 
 - Zero Heavy Dependencies: Built with minimal external crates
 
 - **size about 8MB, compared to "micro" 12MB<**

## Commands

| Command | Description |
|---------|-------------|
| `quick` | Open empty editor in terminal |
| `quick <file>` | Open file in terminal |
| `quick --gui` | Open empty editor in GUI |
| `quick --gui <file>` | Open file in GUI |
| `quick <file> --gui` | Open file in GUI (alternative) |
| `quick --shortcuts` | Show all keyboard shortcuts |

## Installation (LINUX)
### step by step:
download the app:
```bash
wget https://github.com/DomanskiFilip/quick_notepad/releases/latest/download/quick
```

make it executable:
```bash
chmod +x quick
```

run it first time for it to automatically install:
```bash
./quick
```
### one liner:
```bash
curl -L https://github.com/DomanskiFilip/quick_notepad/releases/latest/download/quick -o quick && chmod +x quick && ./quick
```

### if the app doesnt show up try reloading your shell:
```bash
# For Bash/Zsh:
source ~/.bashrc
# For Fish:
exec fish
```

### Uninstall
```bash
rm ~/.local/bin/quick
```

### Build from Source
```bash
git clone https://github.com/DomanskiFilip/quick_notepad
cd quick-notepad
./build-dist.sh
cd quick-notepad-[VERSION]-linux-x86_64
./install.sh
```

as you can see source has my build script: ./build-dist.sh which creates these scripts: ./install.sh and ./uninstall.sh in quick-notepad-[VERSION]-linux-x86_64

<img width="1024" height="1024" alt="image" src="https://github.com/user-attachments/assets/74ae2248-706e-4970-ada4-f67a48003c86" />
icon generated using nano banana

# LICENSE

Copyright (c) 2026 Filip Domanski

quick notepad is provided for personal, non-commercial use only. You may view, download, and run the application or source code for your own personal purposes.

You may not:
- Redistribute, host, or publish the code or application in any form, whether modified or unmodified.
- Monetize, sell, or offer the code or application as a service.
- Create forks or derivative works for distribution or public hosting.
- Use the code or application in any commercial context.

Filip Domanski is the exclusive host and distributor of quick notepad. For commercial or redistribution inquiries, contact the copyright holder.

All rights reserved.
