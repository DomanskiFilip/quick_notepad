#!/bin/bash

set -e

VERSION="1.0.0"
DIST_NAME="quick-notepad-${VERSION}-linux-x86_64"

echo "╔════════════════════════════════════════╗"
echo "║  Building Quick Notepad Distribution  ║"
echo "╚════════════════════════════════════════╝"
echo ""

# Build release binary
echo "🔨 Building release binary..."
cargo build --release

# Create distribution directory
echo "📁 Creating distribution package..."
rm -rf "$DIST_NAME"
mkdir -p "$DIST_NAME"

# Copy binary
cp target/release/quick "$DIST_NAME/"

# Copy assets
mkdir -p "$DIST_NAME/assets"
cp assets/icon.png "$DIST_NAME/assets/"
cp assets/quick-notepad.desktop "$DIST_NAME/assets/"

# Create installer script
cat > "$DIST_NAME/install.sh" << 'EOF'
#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "╔════════════════════════════════════════╗"
echo "║   Quick Notepad Installer v1.0        ║"
echo "╚════════════════════════════════════════╝"
echo ""

INSTALL_DIR="$HOME/.local/bin"
APPS_DIR="$HOME/.local/share/applications"
ICONS_DIR="$HOME/.local/share/icons/hicolor/512x512/apps"

# Create directories
mkdir -p "$INSTALL_DIR"
mkdir -p "$APPS_DIR"
mkdir -p "$ICONS_DIR"

# Install binary
echo "📥 Installing binary..."
cp "$SCRIPT_DIR/quick" "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/quick"

# Create 'quick' symlink
echo "🔗 Creating 'quick' command..."
ln -sf "$INSTALL_DIR/quick-notepad" "$INSTALL_DIR/quick"

# Install desktop entry
echo "🖥️  Installing desktop entry..."
cp "$SCRIPT_DIR/assets/quick-notepad.desktop" "$APPS_DIR/"
chmod +x "$APPS_DIR/quick-notepad.desktop"

# Install icon
echo "🎨 Installing icon..."
cp "$SCRIPT_DIR/assets/icon.png" "$ICONS_DIR/icon.png"

# Update databases
if command -v update-desktop-database &> /dev/null; then
    echo "🔄 Updating desktop database..."
    update-desktop-database "$APPS_DIR" 2>/dev/null || true
fi

if command -v gtk-update-icon-cache &> /dev/null; then
    echo "🔄 Updating icon cache..."
    gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
fi

# Check if in PATH
echo ""
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo "⚠️  Add to your ~/.bashrc or ~/.zshrc:"
    echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo ""
    echo "Then reload:"
    echo "    source ~/.bashrc"
    echo ""
fi

echo "╔════════════════════════════════════════╗"
echo "║     Installation Complete! 🎉         ║"
echo "╚════════════════════════════════════════╝"
echo ""
echo "Terminal (TUI) Mode:"
echo "  quick                    # Start empty"
echo "  quick file.txt           # Open file"
echo ""
echo "GUI Mode:"
echo "  quick --gui              # Start GUI"
echo "  quick --gui file.txt     # Open in GUI"
echo ""
echo "Other:"
echo "  quick --shortcuts        # Show shortcuts"
echo ""
EOF

chmod +x "$DIST_NAME/install.sh"

# Create uninstaller script
cat > "$DIST_NAME/uninstall.sh" << 'EOF'
#!/bin/bash

set -e

echo "🗑️  Uninstalling Quick Notepad..."

INSTALL_DIR="$HOME/.local/bin"
APPS_DIR="$HOME/.local/share/applications"
ICONS_DIR="$HOME/.local/share/icons/hicolor/512x512/apps"

# Remove binaries
rm -f "$INSTALL_DIR/quick_notepad"
rm -f "$INSTALL_DIR/quick"
echo "✓ Removed binaries"

# Remove desktop entry
rm -f "$APPS_DIR/quick_notepad.desktop"
echo "✓ Removed desktop entry"

# Remove icon
rm -f "$ICONS_DIR/quick_notepad.png"
echo "✓ Removed icon"

# Update databases
if command -v update-desktop-database &> /dev/null; then
    update-desktop-database "$APPS_DIR" 2>/dev/null || true
fi

if command -v gtk-update-icon-cache &> /dev/null; then
    gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
fi

echo ""
echo "✅ Quick Notepad uninstalled!"
EOF

chmod +x "$DIST_NAME/uninstall.sh"

# Create README
cat > "$DIST_NAME/README.txt" << 'EOF'
╔════════════════════════════════════════════════════════════╗
║              Quick Notepad - Installation                  ║
╚════════════════════════════════════════════════════════════╝

QUICK START:

  ./install.sh

Then reload your shell:

  source ~/.bashrc

USAGE:

  Terminal (TUI):
    quick                    # Start empty editor
    quick file.txt           # Open file
    quick-notepad file.txt   # Same as 'quick'

  GUI:
    quick --gui              # Start GUI
    quick --gui file.txt     # Open file in GUI
    quick file.txt --gui     # Same as above

  Other:
    quick --shortcuts        # Show all shortcuts

DESKTOP INTEGRATION:

  After installation:
  - Click "Quick Notepad" icon in application menu
  - Right-click files → "Open with Quick Notepad"
  - Right-click files → "Open in Terminal Mode"

UNINSTALL:

  ./uninstall.sh

REQUIREMENTS:

  - Linux (x86_64)
  - No additional dependencies

© 2024 Filip Domanski
EOF

# Copy LICENSE if exists
if [ -f "LICENSE" ]; then
    cp LICENSE "$DIST_NAME/"
fi

# Create tarball
echo "📦 Creating tarball..."
tar czf "${DIST_NAME}.tar.gz" "$DIST_NAME"

# Calculate size and checksum
SIZE=$(du -h "${DIST_NAME}.tar.gz" | cut -f1)
SHA256=$(sha256sum "${DIST_NAME}.tar.gz" | cut -d' ' -f1)

echo ""
echo "╔════════════════════════════════════════╗"
echo "║      Distribution Created! 🎉         ║"
echo "╚════════════════════════════════════════╝"
echo ""
echo "📦 Package: ${DIST_NAME}.tar.gz"
echo "📏 Size: $SIZE"
echo "🔒 SHA256: $SHA256"
echo ""
echo "📂 Contents:"
ls -lh "$DIST_NAME"
echo ""
echo "To test locally:"
echo "  cd $DIST_NAME"
echo "  ./install.sh"
echo ""
echo "To distribute:"
echo "  Share ${DIST_NAME}.tar.gz"
echo ""

# Create checksum file
echo "$SHA256  ${DIST_NAME}.tar.gz" > "${DIST_NAME}.tar.gz.sha256"
echo "✓ Created checksum file: ${DIST_NAME}.tar.gz.sha256"