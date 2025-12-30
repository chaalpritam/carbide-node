#!/bin/bash
# Build script for Carbide Provider Desktop App

set -e

echo "🖥️  Building Carbide Provider Desktop App"
echo "========================================"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if we're in the right directory
if [ ! -d "gui" ]; then
    echo "❌ Error: Please run this script from the carbide-node root directory"
    exit 1
fi

# Navigate to GUI directory
cd gui

echo -e "${BLUE}📦 Step 1: Installing Node.js dependencies${NC}"
if command -v npm &> /dev/null; then
    npm install
else
    echo "❌ npm not found. Please install Node.js first."
    exit 1
fi

echo -e "${BLUE}🦀 Step 2: Checking Rust toolchain${NC}"
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust not found. Please install Rust first."
    echo "Visit: https://rustup.rs/"
    exit 1
fi

echo -e "${BLUE}📱 Step 3: Installing Tauri CLI${NC}"
if ! command -v tauri &> /dev/null; then
    echo "Installing Tauri CLI..."
    npm install -g @tauri-apps/cli
fi

echo -e "${BLUE}🏗️  Step 4: Building Carbide provider binaries${NC}"
cd ..
cargo build --release --bin carbide-provider
cd gui

echo -e "${BLUE}🖨️  Step 5: Building desktop application${NC}"
npm run tauri:build

# Check if build was successful
if [ -d "src-tauri/target/release/bundle/macos" ]; then
    echo -e "\n${GREEN}✅ Build completed successfully!${NC}"
    echo -e "📂 App bundle location: gui/src-tauri/target/release/bundle/macos/"
    
    # Find the app bundle
    APP_BUNDLE=$(find src-tauri/target/release/bundle/macos -name "*.app" | head -1)
    
    if [ -n "$APP_BUNDLE" ]; then
        APP_NAME=$(basename "$APP_BUNDLE")
        echo -e "🎉 Built: ${YELLOW}$APP_NAME${NC}"

        # Remove quarantine flag to prevent "app is damaged" error
        echo -e "${BLUE}🔓 Removing macOS quarantine flag${NC}"
        xattr -cr "$APP_BUNDLE" 2>/dev/null || true

        echo ""
        echo "📋 Next steps:"
        echo "1. Install: cp -r \"$APP_BUNDLE\" /Applications/"
        echo "2. Remove quarantine (if needed): sudo xattr -cr \"/Applications/$APP_NAME\""
        echo "3. Launch from Applications or run: open \"/Applications/$APP_NAME\""
        echo ""
        echo -e "${YELLOW}⚠️  macOS Security Note:${NC}"
        echo "If you see 'app is damaged' error, this is macOS Gatekeeper blocking unsigned apps."
        echo "The app is NOT damaged - just run: sudo xattr -cr \"/Applications/$APP_NAME\""
        echo "Or right-click the app and select 'Open' to bypass."
        echo ""
        echo "🚀 Your Carbide Provider desktop app is ready!"
    fi
else
    echo -e "${RED}❌ Build failed - app bundle not found${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}🎯 Build Summary${NC}"
echo "================"
echo "✅ Dependencies installed"
echo "✅ Carbide provider compiled"
echo "✅ Desktop app built"
echo "✅ Ready for installation"

# Optional: Create installer DMG
echo ""
read -p "🤔 Create installer DMG? (y/N): " CREATE_DMG
if [[ $CREATE_DMG =~ ^[Yy]$ ]]; then
    if [ -n "$APP_BUNDLE" ]; then
        DMG_NAME="CarbideProvider-$(date +%Y%m%d).dmg"
        echo -e "${BLUE}📦 Creating installer DMG: $DMG_NAME${NC}"
        
        # Create DMG (requires macOS)
        if command -v hdiutil &> /dev/null; then
            hdiutil create -volname "Carbide Provider" -srcfolder "$APP_BUNDLE" -ov -format UDZO "$DMG_NAME"
            echo -e "${GREEN}✅ Created installer: $DMG_NAME${NC}"
        else
            echo -e "${YELLOW}⚠️  DMG creation requires macOS${NC}"
        fi
    fi
fi