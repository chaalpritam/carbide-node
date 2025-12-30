#!/bin/bash
# Professional DMG Creator for Carbide Provider
# Creates a beautiful, distributable DMG installer

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
NC='\033[0m' # No Color

# Configuration
APP_NAME="Carbide Provider"
DMG_NAME="CarbideProvider-Installer"
VERSION="1.0.0"
BACKGROUND_NAME="dmg-background.png"
APP_BUNDLE="gui/src-tauri/target/release/bundle/macos/Carbide Provider.app"

print_header() {
    echo -e "${BLUE}"
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║                                                              ║"
    echo "║       🖥️  Carbide Provider DMG Creator 🖥️                  ║"
    echo "║                                                              ║"
    echo "║            Professional Installer Package                   ║"
    echo "║                                                              ║"
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

print_step() {
    echo -e "\n${GREEN}▶ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

check_requirements() {
    print_step "Checking requirements..."
    
    # Check if on macOS
    if [[ "$(uname)" != "Darwin" ]]; then
        print_error "This script requires macOS"
        exit 1
    fi
    
    # Check if app bundle exists
    if [[ ! -d "$APP_BUNDLE" ]]; then
        print_error "App bundle not found at: $APP_BUNDLE"
        echo "Please build the app first with: ./build-gui.sh"
        exit 1
    fi
    
    # Check for required tools
    if ! command -v create-dmg &> /dev/null; then
        print_warning "create-dmg not found. Installing via Homebrew..."
        if command -v brew &> /dev/null; then
            brew install create-dmg
        else
            print_error "Homebrew not found. Please install create-dmg manually:"
            echo "brew install create-dmg"
            exit 1
        fi
    fi
    
    print_success "All requirements met"
}

create_dmg_assets() {
    print_step "Creating DMG assets..."
    
    # Create assets directory
    mkdir -p dmg-assets
    
    # Create beautiful DMG background using ImageMagick or built-in tools
    if command -v magick &> /dev/null || command -v convert &> /dev/null; then
        create_background_with_imagemagick
    else
        create_background_with_sips
    fi
    
    # Create installer script
    create_installer_script
    
    # Create README for DMG
    create_dmg_readme
    
    print_success "DMG assets created"
}

create_background_with_imagemagick() {
    print_step "Creating DMG background with ImageMagick..."
    
    # Use ImageMagick or GraphicsMagick to create a beautiful background
    MAGICK_CMD="magick"
    if command -v convert &> /dev/null && ! command -v magick &> /dev/null; then
        MAGICK_CMD="convert"
    fi
    
    $MAGICK_CMD -size 900x600 \
        gradient:'#0ea5e9-#075985' \
        -font 'SF-Pro-Display-Bold' -pointsize 48 -fill white \
        -gravity center -annotate +0-150 'Carbide Provider' \
        -font 'SF-Pro-Display' -pointsize 24 -fill 'rgba(255,255,255,0.8)' \
        -gravity center -annotate +0-100 'Decentralized Storage Marketplace' \
        -font 'SF-Pro-Display' -pointsize 18 -fill 'rgba(255,255,255,0.7)' \
        -gravity center -annotate +0+200 'Drag Carbide Provider to Applications' \
        -font 'SF-Pro-Display' -pointsize 16 -fill 'rgba(255,255,255,0.6)' \
        -gravity center -annotate +0+230 'Then run the app to start earning!' \
        dmg-assets/$BACKGROUND_NAME
}

create_background_with_sips() {
    print_step "Creating simple DMG background..."
    
    # Create a simple gradient background using built-in tools
    # This is a fallback when ImageMagick isn't available
    
    # Create a simple blue background (fallback)
    cat > dmg-assets/background.html << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <style>
        body {
            margin: 0;
            width: 900px;
            height: 600px;
            background: linear-gradient(135deg, #0ea5e9 0%, #075985 100%);
            display: flex;
            flex-direction: column;
            justify-content: center;
            align-items: center;
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
            color: white;
            text-align: center;
        }
        h1 {
            font-size: 48px;
            font-weight: bold;
            margin: 0 0 20px 0;
        }
        h2 {
            font-size: 24px;
            font-weight: normal;
            margin: 0 0 100px 0;
            opacity: 0.8;
        }
        .instructions {
            font-size: 18px;
            opacity: 0.7;
        }
    </style>
</head>
<body>
    <h1>Carbide Provider</h1>
    <h2>Decentralized Storage Marketplace</h2>
    <div class="instructions">
        <div>Drag Carbide Provider to Applications</div>
        <div style="margin-top: 10px;">Then run the app to start earning!</div>
    </div>
</body>
</html>
EOF
    
    # Convert HTML to image (requires wkhtmltopdf or similar)
    # For now, we'll skip the custom background and let create-dmg use defaults
    print_warning "Using default DMG background (install ImageMagick for custom background)"
}

create_installer_script() {
    print_step "Creating installer script..."
    
    cat > dmg-assets/Install.command << 'EOF'
#!/bin/bash
# Carbide Provider Installer Script
# This script helps users install Carbide Provider correctly

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

clear
echo -e "${BLUE}"
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║                                                              ║"
echo "║       🌟 Carbide Provider Installation Helper 🌟           ║"
echo "║                                                              ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo -e "${NC}"

echo ""
echo "This installer will help you set up Carbide Provider on your Mac."
echo ""

# Check if app is already in Applications
if [[ -d "/Applications/Carbide Provider.app" ]]; then
    echo -e "${GREEN}✅ Carbide Provider is already installed!${NC}"
    echo ""
    read -p "Would you like to launch it now? (y/N): " launch
    if [[ $launch =~ ^[Yy]$ ]]; then
        open "/Applications/Carbide Provider.app"
    fi
    exit 0
fi

echo -e "${YELLOW}📋 Installation Steps:${NC}"
echo "1. Drag 'Carbide Provider.app' to the Applications folder"
echo "2. Open Applications folder and double-click Carbide Provider"
echo "3. Follow the setup wizard to configure your storage provider"
echo "4. Start earning passive income!"
echo ""

echo -e "${BLUE}💡 Tip:${NC} The app will guide you through setting up 25GB of storage"
echo "and help you start earning approximately \$0.125/month when fully utilized."
echo ""

read -p "Press Enter to open the Applications folder..."
open /Applications

echo ""
echo -e "${GREEN}🎉 Ready to install!${NC}"
echo "Drag Carbide Provider from this window to Applications, then launch it."

# Keep terminal open
read -p "Press Enter when installation is complete..."
EOF

    # Make installer script executable
    chmod +x dmg-assets/Install.command
}

create_dmg_readme() {
    print_step "Creating DMG README..."
    
    cat > dmg-assets/README.txt << 'EOF'
🌟 CARBIDE PROVIDER - DECENTRALIZED STORAGE MARKETPLACE 🌟

Thank you for downloading Carbide Provider!

INSTALLATION INSTRUCTIONS:
==========================

1. INSTALL THE APP:
   • Drag "Carbide Provider.app" to the Applications folder
   • Alternatively, double-click "Install.command" for guided installation

2. LAUNCH THE APP:
   • Open Applications folder
   • Double-click "Carbide Provider"
   • Grant any permission requests (Carbide needs file system access)

3. SETUP WIZARD:
   • The app will launch a beautiful setup wizard
   • Configure your provider name and storage allocation (25GB recommended)
   • Set your pricing (default: $0.005/GB/month is competitive)
   • Click "Install Carbide" to complete setup

4. START EARNING:
   • Your Mac will automatically start accepting storage requests
   • Monitor earnings and performance from the dashboard
   • The app runs in the background and starts automatically on boot

FEATURES:
=========

✅ Beautiful Desktop Interface - Native macOS app with real-time dashboard
✅ Automatic Setup - One-click installation and configuration
✅ Earnings Tracking - Live monitoring of daily/monthly earnings
✅ Storage Management - Visual storage usage with easy folder access
✅ System Monitoring - CPU, memory, and performance tracking
✅ Settings Panel - Complete provider configuration
✅ Live Logs - Real-time log viewer with filtering
✅ System Tray - Background operation with quick controls
✅ Auto-start - Automatically starts earning on boot

EARNING POTENTIAL:
==================

With 25GB allocated storage at $0.005/GB/month:
• Maximum monthly earnings: $0.125
• Daily potential: ~$0.004
• Actual earnings depend on network demand and your uptime

Your earnings will grow as:
• Network adoption increases
• Your reputation improves (better uptime = more clients)
• You maintain high availability and performance

SYSTEM REQUIREMENTS:
====================

• macOS 10.15+ (Catalina or newer)
• Apple Silicon (M1/M2) recommended, Intel supported
• 30GB available storage (25GB for sharing + 5GB overhead)
• Stable internet connection
• Administrator access for installation

SUPPORT:
========

• Documentation: Check the app's built-in help
• Logs: View detailed logs from the app's Logs panel
• Issues: Report issues with log files for faster resolution
• Community: Join the Carbide Network community for tips and support

GET STARTED:
============

1. Install the app (drag to Applications)
2. Launch and follow the setup wizard
3. Start earning passive income!

Your Mac mini is about to become a profitable storage provider! 🚀

Welcome to the Carbide Network! 🌟
EOF
}

create_dmg_package() {
    print_step "Creating DMG package..."
    
    # Clean up any existing DMG
    rm -f "${DMG_NAME}-${VERSION}.dmg"
    
    # Create temporary directory for DMG contents
    TEMP_DMG_DIR="temp-dmg"
    rm -rf "$TEMP_DMG_DIR"
    mkdir -p "$TEMP_DMG_DIR"
    
    # Copy app bundle
    cp -R "$APP_BUNDLE" "$TEMP_DMG_DIR/"
    
    # Copy installer script
    cp dmg-assets/Install.command "$TEMP_DMG_DIR/"
    
    # Copy README
    cp dmg-assets/README.txt "$TEMP_DMG_DIR/"
    
    # Create Applications symlink for easy drag-and-drop
    ln -s /Applications "$TEMP_DMG_DIR/Applications"
    
    # DMG creation parameters
    DMG_OPTIONS=(
        --volname "Carbide Provider Installer"
        --volicon "dmg-assets/dmg-icon.icns"
        --window-pos 200 120
        --window-size 900 600
        --icon-size 128
        --icon "Carbide Provider.app" 200 280
        --icon "Applications" 700 280
        --icon "Install.command" 450 450
        --icon "README.txt" 450 520
        --hide-extension "Carbide Provider.app"
        --app-drop-link 700 280
        --hdiutil-quiet
    )
    
    # Add background if it exists
    if [[ -f "dmg-assets/$BACKGROUND_NAME" ]]; then
        DMG_OPTIONS+=(--background "dmg-assets/$BACKGROUND_NAME")
    fi
    
    # Create the DMG
    create-dmg "${DMG_OPTIONS[@]}" \
        "${DMG_NAME}-${VERSION}.dmg" \
        "$TEMP_DMG_DIR"
    
    # Clean up temporary directory
    rm -rf "$TEMP_DMG_DIR"
    
    print_success "DMG package created: ${DMG_NAME}-${VERSION}.dmg"
}

create_release_notes() {
    print_step "Creating release notes..."
    
    cat > "Release-Notes-${VERSION}.md" << EOF
# 🚀 Carbide Provider v${VERSION} - Release Notes

## 🎉 **What's New**

### 🖥️ **Beautiful Desktop Application**
- **Native macOS App**: Built with Tauri for optimal performance
- **Installation Wizard**: Guided setup with real-time progress
- **Live Dashboard**: Real-time monitoring of earnings and performance
- **Settings Panel**: Complete provider configuration
- **Logs Viewer**: Live log streaming with filtering

### 💰 **Earnings Features**
- **Real-time Tracking**: Live earnings calculation and display
- **Beautiful Charts**: Visual earnings trends and projections
- **Reputation Score**: Track your provider reputation
- **Performance Metrics**: Monitor system health and optimize earnings

### 🔧 **Easy Installation**
1. **Download**: Get the DMG installer
2. **Drag & Drop**: Install to Applications folder
3. **Setup**: Follow the beautiful setup wizard
4. **Earn**: Start earning passive income immediately!

## 📋 **System Requirements**
- macOS 10.15+ (Catalina or newer)
- Apple Silicon (M1/M2) recommended
- 30GB available storage
- Stable internet connection

## 💡 **Quick Start**
1. Open the DMG file
2. Drag Carbide Provider to Applications
3. Launch the app
4. Follow setup wizard (allocate 25GB storage)
5. Start earning! 🎉

## 🎯 **Earning Potential**
- **Storage**: 25GB allocated
- **Rate**: \$0.005/GB/month (competitive market rate)
- **Max Monthly**: \$0.125 when fully utilized
- **Passive Income**: Runs 24/7 automatically

## 🔄 **What's Next**
- Enhanced reputation system
- Mobile companion app
- Advanced analytics
- Multi-region support

---

**Download now and start earning with your Mac! 💰**

*Built with ❤️ for the Carbide Network community*
EOF
}

create_download_page() {
    print_step "Creating download page template..."
    
    cat > download-page.html << 'EOF'
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Download Carbide Provider - Turn Your Mac Into a Storage Provider</title>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }
        
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
            line-height: 1.6;
            color: #333;
            background: linear-gradient(135deg, #0ea5e9 0%, #075985 100%);
            min-height: 100vh;
        }
        
        .container {
            max-width: 1200px;
            margin: 0 auto;
            padding: 2rem;
        }
        
        .hero {
            text-align: center;
            color: white;
            padding: 4rem 0;
        }
        
        .hero h1 {
            font-size: 3.5rem;
            font-weight: bold;
            margin-bottom: 1rem;
        }
        
        .hero p {
            font-size: 1.5rem;
            opacity: 0.9;
            margin-bottom: 3rem;
        }
        
        .download-card {
            background: white;
            border-radius: 20px;
            padding: 3rem;
            box-shadow: 0 20px 40px rgba(0,0,0,0.1);
            max-width: 600px;
            margin: 0 auto;
        }
        
        .download-btn {
            display: inline-flex;
            align-items: center;
            gap: 12px;
            background: #0ea5e9;
            color: white;
            padding: 1rem 2rem;
            border-radius: 12px;
            text-decoration: none;
            font-weight: bold;
            font-size: 1.2rem;
            transition: all 0.3s ease;
            margin-bottom: 2rem;
        }
        
        .download-btn:hover {
            background: #0284c7;
            transform: translateY(-2px);
            box-shadow: 0 10px 25px rgba(14, 165, 233, 0.3);
        }
        
        .features {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
            gap: 2rem;
            margin-top: 3rem;
        }
        
        .feature {
            text-align: center;
            padding: 1.5rem;
        }
        
        .feature-icon {
            font-size: 3rem;
            margin-bottom: 1rem;
        }
        
        .stats {
            display: grid;
            grid-template-columns: repeat(3, 1fr);
            gap: 2rem;
            margin-top: 2rem;
            text-align: center;
        }
        
        .stat-value {
            font-size: 2rem;
            font-weight: bold;
            color: #0ea5e9;
        }
        
        .requirements {
            background: #f8fafc;
            padding: 2rem;
            border-radius: 12px;
            margin-top: 2rem;
        }
        
        .step {
            display: flex;
            align-items: center;
            gap: 1rem;
            padding: 1rem 0;
            border-bottom: 1px solid #e2e8f0;
        }
        
        .step:last-child {
            border-bottom: none;
        }
        
        .step-number {
            background: #0ea5e9;
            color: white;
            width: 32px;
            height: 32px;
            border-radius: 50%;
            display: flex;
            align-items: center;
            justify-content: center;
            font-weight: bold;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="hero">
            <h1>🌟 Carbide Provider</h1>
            <p>Turn Your Mac Into a Profitable Storage Provider</p>
            
            <div class="download-card">
                <a href="CarbideProvider-Installer-1.0.0.dmg" class="download-btn">
                    📦 Download for macOS
                    <span style="opacity: 0.8; font-size: 0.9rem;">(Universal)</span>
                </a>
                
                <div class="stats">
                    <div>
                        <div class="stat-value">25GB</div>
                        <div>Storage Allocation</div>
                    </div>
                    <div>
                        <div class="stat-value">$0.125</div>
                        <div>Max Monthly Earnings</div>
                    </div>
                    <div>
                        <div class="stat-value">24/7</div>
                        <div>Passive Income</div>
                    </div>
                </div>
                
                <div class="features">
                    <div class="feature">
                        <div class="feature-icon">🖥️</div>
                        <h3>Beautiful Desktop App</h3>
                        <p>Native macOS application with real-time dashboard</p>
                    </div>
                    <div class="feature">
                        <div class="feature-icon">⚡</div>
                        <h3>One-Click Setup</h3>
                        <p>Guided installation wizard with automated configuration</p>
                    </div>
                    <div class="feature">
                        <div class="feature-icon">💰</div>
                        <h3>Live Earnings</h3>
                        <p>Real-time tracking with beautiful charts and projections</p>
                    </div>
                </div>
                
                <div class="requirements">
                    <h3 style="margin-bottom: 1rem;">📋 System Requirements</h3>
                    <ul style="list-style: none;">
                        <li>✅ macOS 10.15+ (Catalina or newer)</li>
                        <li>✅ Apple Silicon (M1/M2) or Intel Mac</li>
                        <li>✅ 30GB available storage</li>
                        <li>✅ Stable internet connection</li>
                    </ul>
                </div>
                
                <div style="margin-top: 2rem;">
                    <h3>🚀 Quick Installation</h3>
                    <div class="step">
                        <div class="step-number">1</div>
                        <div>Download and open the DMG file</div>
                    </div>
                    <div class="step">
                        <div class="step-number">2</div>
                        <div>Drag Carbide Provider to Applications</div>
                    </div>
                    <div class="step">
                        <div class="step-number">3</div>
                        <div>Launch app and follow setup wizard</div>
                    </div>
                    <div class="step">
                        <div class="step-number">4</div>
                        <div>Start earning passive income! 🎉</div>
                    </div>
                </div>
            </div>
        </div>
    </div>
</body>
</html>
EOF
    
    print_success "Download page created: download-page.html"
}

add_code_signing() {
    print_step "Setting up code signing (optional)..."
    
    if [[ -n "$APPLE_DEVELOPER_ID" ]]; then
        print_step "Code signing with Apple Developer ID..."
        
        # Sign the app bundle
        codesign --force --deep --sign "$APPLE_DEVELOPER_ID" "$APP_BUNDLE"
        
        # Verify signing
        if codesign --verify --verbose "$APP_BUNDLE"; then
            print_success "Code signing successful"
        else
            print_warning "Code signing verification failed"
        fi
    else
        print_warning "No APPLE_DEVELOPER_ID set - skipping code signing"
        echo "To enable code signing, set: export APPLE_DEVELOPER_ID='Developer ID Application: Your Name'"
    fi
}

cleanup() {
    print_step "Cleaning up temporary files..."
    rm -rf dmg-assets temp-dmg
}

main() {
    print_header
    
    echo "This script will create a professional DMG installer for Carbide Provider."
    echo "The DMG will include:"
    echo "• Carbide Provider.app"
    echo "• Installation helper script"
    echo "• README with instructions"
    echo "• Beautiful DMG design"
    echo ""
    
    read -p "Continue? (Y/n): " confirm
    if [[ "$confirm" =~ ^[Nn]$ ]]; then
        echo "Cancelled."
        exit 0
    fi
    
    check_requirements
    create_dmg_assets
    add_code_signing
    create_dmg_package
    create_release_notes
    create_download_page
    cleanup
    
    echo ""
    print_success "🎉 DMG Installer Package Complete!"
    echo ""
    echo "📦 Created Files:"
    echo "  • ${DMG_NAME}-${VERSION}.dmg - Main installer"
    echo "  • Release-Notes-${VERSION}.md - Release documentation"
    echo "  • download-page.html - Download page template"
    echo ""
    echo "🚀 Distribution:"
    echo "  • Upload DMG to your website or GitHub releases"
    echo "  • Share download-page.html as landing page"
    echo "  • Users can simply download and drag-to-install"
    echo ""
    echo "💡 Next Steps:"
    echo "  1. Test the DMG on a clean Mac"
    echo "  2. Upload to distribution platform"
    echo "  3. Share with Mac mini users!"
    echo ""
    echo "🎯 Your Carbide Provider is ready for easy distribution!"
}

# Handle script termination
trap cleanup EXIT

# Run main function
main "$@"