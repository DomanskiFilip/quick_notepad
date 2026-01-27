#!/bin/bash

set -e

VERSION="0.0.1"
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

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Header
echo "╔════════════════════════════════════════╗"
echo "║   Quick Notepad Installer v1.0        ║"
echo "╚════════════════════════════════════════╝"
echo ""

# Detect if running with sudo
if [ "$EUID" -eq 0 ]; then
    # Running as root - install to user's home directory
    if [ -n "$SUDO_USER" ]; then
        USER_HOME=$(getent passwd "$SUDO_USER" | cut -d: -f6)
        INSTALL_DIR="$USER_HOME/.local/bin"
        ACTUAL_USER="$SUDO_USER"
    else
        echo -e "${RED}✗ Please run without sudo, or use: sudo -E ./install.sh${NC}"
        exit 1
    fi
else
    # Running as regular user
    INSTALL_DIR="$HOME/.local/bin"
    ACTUAL_USER="$USER"
fi

# Create installation directory if it doesn't exist
if [ ! -d "$INSTALL_DIR" ]; then
    echo -e "${BLUE}📁 Creating installation directory...${NC}"
    mkdir -p "$INSTALL_DIR"
    
    if [ "$EUID" -eq 0 ]; then
        chown "$ACTUAL_USER:$ACTUAL_USER" "$INSTALL_DIR"
    fi
fi

# Remove existing symlink or file if it exists
TARGET="$INSTALL_DIR/quick"
if [ -L "$TARGET" ]; then
    echo -e "${YELLOW}⚠ Removing existing symlink...${NC}"
    rm "$TARGET"
elif [ -f "$TARGET" ]; then
    echo -e "${YELLOW}⚠ Removing existing installation...${NC}"
    rm "$TARGET"
fi

# Install binary
echo -e "${BLUE}📥 Installing binary...${NC}"
if cp quick "$TARGET"; then
    echo -e "${GREEN}✓ Binary installed to $TARGET${NC}"
else
    echo -e "${RED}✗ Failed to install binary${NC}"
    exit 1
fi

# Set ownership if installed with sudo
if [ "$EUID" -eq 0 ]; then
    chown "$ACTUAL_USER:$ACTUAL_USER" "$TARGET"
fi

# Make executable
chmod +x "$TARGET"

# Check if directory is in PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo ""
    echo -e "${YELLOW}⚠ Warning: $INSTALL_DIR is not in your PATH${NC}"
    echo ""
    echo "Add this line to your ~/.bashrc or ~/.zshrc:"
    echo -e "${BLUE}export PATH=\"\$HOME/.local/bin:\$PATH\"${NC}"
    echo ""
    echo "Then run: source ~/.bashrc"
else
    echo -e "${GREEN}✓ Installation directory is in PATH${NC}"
fi

echo ""
echo -e "${GREEN}✓ Installation complete!${NC}"
echo ""
echo "Run 'quick' to start Quick Notepad"
echo "Run 'quick --help' for options"
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