#!/bin/bash
# Carbide Provider Complete Installer
# This script handles the complete installation of Carbide Provider
# from source code to running desktop application

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
NC='\033[0m' # No Color

# Configuration
CARBIDE_HOME="$HOME/.carbide"
PROJECT_NAME="carbide-node"
GITHUB_REPO="https://github.com/your-org/carbide-node.git"
TEMP_DIR="/tmp/carbide-installer-$$"

print_header() {
    clear
    echo -e "${BLUE}"
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║                                                              ║"
    echo "║      🌟 Carbide Provider Complete Installer 🌟             ║"
    echo "║                                                              ║"
    echo "║         Turn Your Mac Into a Storage Provider               ║"
    echo "║                                                              ║"
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
    echo ""
    echo "This installer will:"
    echo "• Download and compile Carbide Provider"
    echo "• Build the beautiful desktop application"
    echo "• Install to your Applications folder"
    echo "• Set up automatic startup"
    echo "• Configure 25GB storage allocation"
    echo ""
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

check_system_requirements() {
    print_step "Checking system requirements..."
    
    # Check macOS version
    if [[ "$(uname)" != "Darwin" ]]; then
        print_error "This installer is for macOS only!"
        exit 1
    fi
    
    # Get macOS version
    MACOS_VERSION=$(sw_vers -productVersion)
    echo "  macOS version: $MACOS_VERSION"
    
    # Check architecture
    ARCH=$(uname -m)
    if [[ "$ARCH" == "arm64" ]]; then
        echo "  Architecture: Apple Silicon (M1/M2) - Optimal performance! 🚀"
    elif [[ "$ARCH" == "x86_64" ]]; then
        echo "  Architecture: Intel Mac - Supported ✅"
    else
        print_warning "Unknown architecture: $ARCH"
    fi
    
    # Check available storage
    AVAILABLE_GB=$(df -g "$HOME" | awk 'NR==2{print $4}')
    echo "  Available storage: ${AVAILABLE_GB}GB"
    
    if [[ $AVAILABLE_GB -lt 35 ]]; then
        print_error "Insufficient storage! Need at least 35GB free (25GB for provider + 10GB for build)"
        echo "Available: ${AVAILABLE_GB}GB"
        exit 1
    fi
    
    print_success "System requirements met!"
}

install_dependencies() {
    print_step "Installing dependencies..."
    
    # Check for Xcode Command Line Tools
    if ! xcode-select -p &> /dev/null; then
        print_step "Installing Xcode Command Line Tools..."
        xcode-select --install
        echo "Please complete the Xcode Command Line Tools installation and run this script again."
        exit 1
    fi
    
    # Check for Homebrew
    if ! command -v brew &> /dev/null; then
        print_step "Installing Homebrew..."
        /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
        
        # Add to PATH
        if [[ "$ARCH" == "arm64" ]]; then
            echo 'eval "$(/opt/homebrew/bin/brew shellenv)"' >> ~/.zshrc
            eval "$(/opt/homebrew/bin/brew shellenv)"
        else
            echo 'eval "$(/usr/local/bin/brew shellenv)"' >> ~/.zshrc
            eval "$(/usr/local/bin/brew shellenv)"
        fi
    fi
    
    print_success "Homebrew available"
    
    # Install required tools
    print_step "Installing required packages..."
    brew_packages=(
        "git"
        "node"
        "rust"
        "create-dmg"
    )
    
    for package in "${brew_packages[@]}"; do
        if ! brew list "$package" &> /dev/null; then
            echo "  Installing $package..."
            brew install "$package"
        else
            echo "  ✅ $package already installed"
        fi
    done
    
    # Install Rust if needed
    if ! command -v cargo &> /dev/null; then
        print_step "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source ~/.cargo/env
    fi
    
    # Install Tauri CLI
    if ! command -v tauri &> /dev/null; then
        print_step "Installing Tauri CLI..."
        cargo install tauri-cli
    fi
    
    print_success "All dependencies installed!"
}

download_carbide() {
    print_step "Downloading Carbide Provider source code..."
    
    # Create temporary directory
    mkdir -p "$TEMP_DIR"
    cd "$TEMP_DIR"
    
    # Check if we're already in a carbide repo
    if [[ -f "../Cargo.toml" ]] && grep -q "carbide" "../Cargo.toml"; then
        print_step "Using existing Carbide source code..."
        cd ..
        PROJECT_DIR="$(pwd)"
    else
        # Clone or download source
        if [[ -n "$GITHUB_REPO" ]]; then
            print_step "Cloning from repository..."
            git clone "$GITHUB_REPO" "$PROJECT_NAME"
            cd "$PROJECT_NAME"
            PROJECT_DIR="$(pwd)"
        else
            print_error "No source repository configured"
            print_step "Please either:"
            echo "1. Run this script from the carbide-node directory, or"
            echo "2. Set GITHUB_REPO to your repository URL"
            exit 1
        fi
    fi
    
    print_success "Source code ready at: $PROJECT_DIR"
}

build_carbide() {
    print_step "Building Carbide Provider..."
    
    cd "$PROJECT_DIR"
    
    # Build the core Rust binaries
    print_step "Compiling Rust binaries (this may take 10-15 minutes)..."
    cargo build --release --bin carbide-provider
    
    # Build the desktop GUI
    print_step "Building desktop application..."
    cd gui
    
    # Install npm dependencies
    if [[ -f "package.json" ]]; then
        print_step "Installing Node.js dependencies..."
        npm install
    else
        print_error "GUI package.json not found"
        exit 1
    fi
    
    # Build the Tauri app
    print_step "Building Tauri desktop app (this may take 5-10 minutes)..."
    npm run tauri:build
    
    cd ..
    
    # Verify build
    APP_BUNDLE="gui/src-tauri/target/release/bundle/macos/Carbide Provider.app"
    if [[ -d "$APP_BUNDLE" ]]; then
        print_success "Desktop application built successfully!"
    else
        print_error "Failed to build desktop application"
        exit 1
    fi
}

install_desktop_app() {
    print_step "Installing Carbide Provider desktop application..."
    
    cd "$PROJECT_DIR"
    
    APP_BUNDLE="gui/src-tauri/target/release/bundle/macos/Carbide Provider.app"
    APP_DESTINATION="/Applications/Carbide Provider.app"
    
    # Remove existing installation
    if [[ -d "$APP_DESTINATION" ]]; then
        print_step "Removing existing installation..."
        rm -rf "$APP_DESTINATION"
    fi
    
    # Copy to Applications
    print_step "Installing to Applications folder..."
    cp -R "$APP_BUNDLE" "/Applications/"
    
    # Verify installation
    if [[ -d "$APP_DESTINATION" ]]; then
        print_success "Desktop app installed to Applications!"
    else
        print_error "Failed to install desktop app"
        exit 1
    fi
}

setup_cli_tools() {
    print_step "Setting up command-line tools..."
    
    # Create carbide home directory
    mkdir -p "$CARBIDE_HOME/bin"
    
    # Copy CLI binaries
    cp "$PROJECT_DIR/target/release/carbide-provider" "$CARBIDE_HOME/bin/"
    
    # Create management scripts
    cat > "$CARBIDE_HOME/bin/carbide" << 'EOF'
#!/bin/bash
# Carbide Provider CLI Tool

case "$1" in
    "start")
        echo "🚀 Starting Carbide Provider..."
        open "/Applications/Carbide Provider.app"
        ;;
    "status")
        if pgrep -f "Carbide Provider" > /dev/null; then
            echo "✅ Carbide Provider is running"
        else
            echo "❌ Carbide Provider is not running"
        fi
        ;;
    "stop")
        echo "🛑 Stopping Carbide Provider..."
        pkill -f "Carbide Provider"
        ;;
    "logs")
        if [[ -f "$HOME/.carbide/logs/provider.log" ]]; then
            tail -f "$HOME/.carbide/logs/provider.log"
        else
            echo "No logs found. Start the provider first."
        fi
        ;;
    "config")
        open "$HOME/.carbide/config/provider.toml"
        ;;
    *)
        echo "Carbide Provider CLI"
        echo "Commands:"
        echo "  start   - Start the desktop application"
        echo "  stop    - Stop the provider"
        echo "  status  - Check if provider is running"
        echo "  logs    - View provider logs"
        echo "  config  - Edit configuration"
        ;;
esac
EOF
    
    chmod +x "$CARBIDE_HOME/bin/carbide"
    
    # Add to PATH
    SHELL_CONFIGS=("$HOME/.zshrc" "$HOME/.bashrc" "$HOME/.bash_profile")
    
    for config in "${SHELL_CONFIGS[@]}"; do
        if [[ -f "$config" ]]; then
            # Remove existing carbide entries
            sed -i '' '/# Carbide CLI/d' "$config"
            sed -i '' '/carbide\/bin/d' "$config"
            
            # Add new entry
            echo "" >> "$config"
            echo "# Carbide CLI" >> "$config"
            echo "export PATH=\"$CARBIDE_HOME/bin:\$PATH\"" >> "$config"
        fi
    done
    
    print_success "CLI tools installed!"
}

create_documentation() {
    print_step "Creating documentation..."
    
    cat > "$CARBIDE_HOME/README.txt" << 'EOF'
🌟 CARBIDE PROVIDER INSTALLED! 🌟

Congratulations! Carbide Provider is now installed on your Mac.

GETTING STARTED:
================

1. LAUNCH THE APP:
   • Open Applications folder
   • Double-click "Carbide Provider"
   • The beautiful setup wizard will guide you through configuration

2. SETUP YOUR PROVIDER:
   • Allocate storage (25GB recommended)
   • Set your pricing ($0.005/GB/month is competitive)
   • Choose your provider tier (Home tier for Mac mini)
   • Complete the automated setup

3. START EARNING:
   • Your provider will automatically start accepting storage requests
   • Monitor earnings from the real-time dashboard
   • The app runs in background and auto-starts on boot

COMMAND LINE TOOLS:
===================

You can also control Carbide from Terminal:

• carbide start   - Launch the desktop app
• carbide stop    - Stop the provider
• carbide status  - Check if running
• carbide logs    - View live logs
• carbide config  - Edit configuration

FEATURES:
=========

✅ Beautiful Desktop Interface
✅ Real-time Earnings Tracking
✅ Storage Usage Monitoring
✅ System Performance Metrics
✅ Live Log Viewer
✅ Complete Settings Panel
✅ System Tray Integration
✅ Auto-start on Boot

SUPPORT:
========

• Use the app's built-in help and logs viewer
• Check ~/.carbide/logs/provider.log for debugging
• Visit the Carbide Network community for support

EARNINGS:
=========

With 25GB allocated at $0.005/GB/month:
• Maximum monthly earnings: $0.125
• Actual earnings depend on network demand and uptime
• Better reputation = more client selection = more earnings

Your Mac mini is ready to earn! 🚀

Welcome to the Carbide Network! 🌟
EOF
    
    print_success "Documentation created at $CARBIDE_HOME/README.txt"
}

cleanup_installation() {
    print_step "Cleaning up installation files..."
    
    # Remove temporary directory
    rm -rf "$TEMP_DIR"
    
    # Clean cargo cache to save space
    cargo cache --remove-dir all &> /dev/null || true
    
    print_success "Installation cleanup complete!"
}

show_completion_message() {
    clear
    echo -e "${GREEN}"
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║                                                              ║"
    echo "║           🎉 INSTALLATION COMPLETE! 🎉                      ║"
    echo "║                                                              ║"
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
    echo ""
    echo -e "${BLUE}🖥️  Desktop App:${NC} Installed to Applications folder"
    echo -e "${BLUE}🛠️  CLI Tools:${NC} Available in Terminal (restart Terminal to use)"
    echo -e "${BLUE}📁 Documentation:${NC} ~/.carbide/README.txt"
    echo ""
    echo -e "${GREEN}🚀 Next Steps:${NC}"
    echo "1. Open Applications folder"
    echo "2. Launch 'Carbide Provider'"
    echo "3. Follow the setup wizard"
    echo "4. Start earning passive income!"
    echo ""
    echo -e "${YELLOW}💰 Earning Potential:${NC}"
    echo "• Storage: 25GB recommended allocation"
    echo "• Rate: \$0.005/GB/month (competitive)"
    echo "• Max Monthly: ~\$0.125 when fully utilized"
    echo "• Passive Income: Runs 24/7 automatically"
    echo ""
    echo -e "${PURPLE}🎮 Quick Commands:${NC}"
    echo "• carbide start  - Launch desktop app"
    echo "• carbide status - Check if running"
    echo "• carbide logs   - View live logs"
    echo ""
    
    read -p "🚀 Would you like to launch Carbide Provider now? (Y/n): " launch
    if [[ ! "$launch" =~ ^[Nn]$ ]]; then
        print_step "Launching Carbide Provider..."
        open "/Applications/Carbide Provider.app"
        echo ""
        echo -e "${GREEN}🎉 Carbide Provider is starting up!${NC}"
        echo "Follow the setup wizard to configure your 25GB storage provider."
    fi
    
    echo ""
    echo -e "${GREEN}Welcome to the Carbide Network! Happy earning! 🌟${NC}"
}

# Main installation flow
main() {
    # Check if running as root
    if [[ $EUID -eq 0 ]]; then
        print_error "Please don't run this installer as root (sudo)"
        exit 1
    fi
    
    print_header
    
    read -p "🤔 Ready to install Carbide Provider? This will take 20-30 minutes. (Y/n): " confirm
    if [[ "$confirm" =~ ^[Nn]$ ]]; then
        echo "Installation cancelled."
        exit 0
    fi
    
    echo ""
    echo "🚀 Starting Carbide Provider installation..."
    echo "This will:"
    echo "• Check system requirements"
    echo "• Install dependencies (Homebrew, Rust, Node.js, etc.)"
    echo "• Download and compile Carbide Provider"
    echo "• Build the desktop application"
    echo "• Install to Applications folder"
    echo "• Set up CLI tools and documentation"
    echo ""
    
    read -p "Continue? (Y/n): " confirm2
    if [[ "$confirm2" =~ ^[Nn]$ ]]; then
        echo "Installation cancelled."
        exit 0
    fi
    
    # Installation steps
    check_system_requirements
    install_dependencies
    download_carbide
    build_carbide
    install_desktop_app
    setup_cli_tools
    create_documentation
    cleanup_installation
    show_completion_message
}

# Handle script interruption
trap 'echo -e "\n${RED}Installation interrupted!${NC}"; cleanup_installation; exit 1' INT TERM

# Run main installation
main "$@"