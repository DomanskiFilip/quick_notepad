## Quick Notepad
A modern, dual-mode text editor written in Rust that runs both in the terminal (TUI) and as a native GUI application. Built from scratch as part of the "build your own X" philosophy to create a fast, reliable, and customizable editor that does exactly what you need.

![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)

## Features
# Core Functionality

 - Dual Mode: Run in terminal (TUI) or as native GUI
 
 - Full Unicode Support: Proper handling of graphemes, emojis, and multi-byte characters
 
 - Syntax Highlighting: 25+ languages with custom theme support
 
 - Multi-Tab Interface: Up to 10 tabs with session persistence
 
 - Auto-Save Sessions: Never lose your work
 
 - Search & Navigation: Fast text search with match highlighting
 
 - Undo/Redo: Full edit history with intelligent grouping

# Advanced Features

 - Mouse Support: Click, drag, double-click, triple-click selection
 
 - Wayland Clipboard Integration: Works seamlessly with system clipboard
 
 - Smart Selection: Word and line selection modes
 
 - Configurable Shortcuts: All shortcuts in one place (for now you need to change in: src/core/shortcuts.rs)

# Technical Highlights

 - Fast Rendering: Optimized for minimal redraws
 
 - Clean Architecture: Modular design across 25+ files and 6 packages
 
 - Zero Heavy Dependencies: Built with minimal external crates
 
 - **size about 4.4MB, 3 times smaller than "micro" (12MB<)**

## Commands

| Command | Description |
|---------|-------------|
| `quick` | Open empty editor in terminal |
| `quick <file>` | Open file in terminal |
| `quick --gui` | Open empty editor in GUI |
| `quick --gui <file>` | Open file in GUI |
| `quick <file> --gui` | Open file in GUI (alternative) |
| `quick --shortcuts` | Show all keyboard shortcuts |

## Installation

### Linux
```bash
# Download latest release
wget (no release yet -> build from source)

# Extract
tar xzf quick-notepad-1.0.0-linux-x86_64.tar.gz
cd quick-notepad-1.0.0-linux-x86_64

# Install
./install.sh

# Reload your shell
source ~/.bashrc
```

Done!

### Verify Download (Optional)
```bash
# Download checksum
wget (no release yet -> build from source)

# Verify
sha256sum -c quick-notepad-1.0.0-linux-x86_64.tar.gz.sha256
```

### Uninstall
```bash
cd quick-notepad-1.0.0-linux-x86_64
./uninstall.sh
```

### Build from Source
```bash
git clone https://github.com/DomanskiFilip/quick_notepad
cd quick-notepad
./build-dist.sh
cd quick-notepad-1.0.0-linux-x86_64
./install.sh
```

<img width="1024" height="1024" alt="image" src="https://github.com/user-attachments/assets/74ae2248-706e-4970-ada4-f67a48003c86" />
icon generated using nano banana
