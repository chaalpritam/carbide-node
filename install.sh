#!/bin/bash
# Carbide Node Mac Installer
# Installs Carbide Node as a storage provider on macOS

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
CARBIDE_HOME="$HOME/.carbide"
CARBIDE_BIN="$CARBIDE_HOME/bin"
CARBIDE_DATA="$CARBIDE_HOME/data"
CARBIDE_CONFIG="$CARBIDE_HOME/config"
CARBIDE_LOGS="$CARBIDE_HOME/logs"

# Default provider settings
DEFAULT_STORAGE_SIZE="25GB"
DEFAULT_PORT="8080"
DEFAULT_PROVIDER_NAME="$(hostname)-carbide-provider"

print_header() {
    echo -e "${BLUE}"
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║                                                              ║"
    echo "║      🌟 Carbide Network - Storage Provider Installer 🌟     ║"
    echo "║                                                              ║"
    echo "║              Decentralized Storage Marketplace              ║"
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
    print_step "Checking system requirements..."
    
    # Check if running on macOS
    if [[ "$(uname)" != "Darwin" ]]; then
        print_error "This installer is for macOS only!"
        exit 1
    fi
    
    # Check for Apple Silicon (M1/M2)
    ARCH=$(uname -m)
    if [[ "$ARCH" != "arm64" ]]; then
        print_warning "Detected Intel Mac ($ARCH). This installer is optimized for Apple Silicon."
        echo "Continuing anyway..."
    else
        print_success "Detected Apple Silicon Mac - perfect for Carbide!"
    fi
    
    # Check available storage
    AVAILABLE_GB=$(df -g "$HOME" | awk 'NR==2{print $4}')
    if [[ $AVAILABLE_GB -lt 30 ]]; then
        print_error "Insufficient storage! Need at least 30GB free (25GB for provider + 5GB overhead)"
        echo "Available: ${AVAILABLE_GB}GB"
        exit 1
    fi
    
    print_success "Storage check passed: ${AVAILABLE_GB}GB available"
    
    # Check for Rust
    if ! command -v cargo &> /dev/null; then
        print_warning "Rust not found. Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source ~/.cargo/env
    fi
    
    print_success "System requirements met!"
}

create_directories() {
    print_step "Creating Carbide directories..."
    
    mkdir -p "$CARBIDE_HOME"
    mkdir -p "$CARBIDE_BIN"
    mkdir -p "$CARBIDE_DATA"
    mkdir -p "$CARBIDE_CONFIG"
    mkdir -p "$CARBIDE_LOGS"
    
    # Create storage directory with specified size limit
    mkdir -p "$CARBIDE_DATA/storage"
    
    print_success "Directory structure created at $CARBIDE_HOME"
}

build_carbide() {
    print_step "Building Carbide Node (this may take a few minutes)..."
    
    # Build in release mode for performance
    cargo build --release --bin carbide-provider
    cargo build --release --bin carbide-discovery
    cargo build --release --bin carbide-client
    
    # Copy binaries to install location
    cp target/release/carbide-provider "$CARBIDE_BIN/"
    cp target/release/carbide-discovery "$CARBIDE_BIN/"
    cp target/release/carbide-client "$CARBIDE_BIN/"
    
    # Make sure binaries are executable
    chmod +x "$CARBIDE_BIN"/*
    
    print_success "Carbide Node built and installed!"
}

generate_config() {
    print_step "Generating provider configuration..."
    
    # Get user input for configuration
    echo -e "\n${BLUE}Provider Configuration:${NC}"
    
    read -p "Provider name [$DEFAULT_PROVIDER_NAME]: " PROVIDER_NAME
    PROVIDER_NAME=${PROVIDER_NAME:-$DEFAULT_PROVIDER_NAME}
    
    read -p "Storage allocation [$DEFAULT_STORAGE_SIZE]: " STORAGE_SIZE
    STORAGE_SIZE=${STORAGE_SIZE:-$DEFAULT_STORAGE_SIZE}
    
    read -p "Port [$DEFAULT_PORT]: " PORT
    PORT=${PORT:-$DEFAULT_PORT}
    
    echo "Select provider tier:"
    echo "1) Home (recommended for Mac mini)"
    echo "2) Professional"
    echo "3) Enterprise"
    read -p "Choice [1]: " TIER_CHOICE
    TIER_CHOICE=${TIER_CHOICE:-1}
    
    case $TIER_CHOICE in
        1) TIER="Home" ;;
        2) TIER="Professional" ;;
        3) TIER="Enterprise" ;;
        *) TIER="Home" ;;
    esac
    
    # Generate provider config
    cat > "$CARBIDE_CONFIG/provider.toml" << EOF
[provider]
name = "$PROVIDER_NAME"
tier = "$TIER"
region = "NorthAmerica"  # Change if needed
port = $PORT
storage_path = "$CARBIDE_DATA/storage"
max_storage_gb = ${STORAGE_SIZE%GB}

[network]
discovery_endpoint = "http://localhost:3000"  # Local discovery for now
advertise_address = "127.0.0.1:$PORT"

[pricing]
price_per_gb_month = 0.005  # $0.005/GB/month - competitive rate

[logging]
level = "info"
file = "$CARBIDE_LOGS/provider.log"

[reputation]
enable_reporting = true
health_check_interval = 300  # 5 minutes
EOF

    print_success "Configuration generated at $CARBIDE_CONFIG/provider.toml"
}

create_launch_daemon() {
    print_step "Creating macOS launch daemon for auto-start..."
    
    PLIST_FILE="$HOME/Library/LaunchAgents/com.carbide.provider.plist"
    
    cat > "$PLIST_FILE" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.carbide.provider</string>
    <key>ProgramArguments</key>
    <array>
        <string>$CARBIDE_BIN/carbide-provider</string>
        <string>--config</string>
        <string>$CARBIDE_CONFIG/provider.toml</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>$CARBIDE_LOGS/provider.out</string>
    <key>StandardErrorPath</key>
    <string>$CARBIDE_LOGS/provider.err</string>
    <key>WorkingDirectory</key>
    <string>$CARBIDE_HOME</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin</string>
    </dict>
</dict>
</plist>
EOF

    print_success "Launch daemon created at $PLIST_FILE"
}

create_management_scripts() {
    print_step "Creating management scripts..."
    
    # Start script
    cat > "$CARBIDE_BIN/carbide-start" << 'EOF'
#!/bin/bash
echo "🚀 Starting Carbide Provider..."
launchctl load ~/Library/LaunchAgents/com.carbide.provider.plist
echo "✅ Carbide Provider started!"
echo "📊 Check status with: carbide-status"
EOF

    # Stop script
    cat > "$CARBIDE_BIN/carbide-stop" << 'EOF'
#!/bin/bash
echo "🛑 Stopping Carbide Provider..."
launchctl unload ~/Library/LaunchAgents/com.carbide.provider.plist
echo "✅ Carbide Provider stopped!"
EOF

    # Status script
    cat > "$CARBIDE_BIN/carbide-status" << EOF
#!/bin/bash
echo "📊 Carbide Provider Status"
echo "========================="

# Check if service is loaded
if launchctl list | grep -q "com.carbide.provider"; then
    echo "✅ Service: Running"
else
    echo "❌ Service: Stopped"
fi

# Check if port is listening
if lsof -i :$PORT &> /dev/null; then
    echo "✅ Network: Listening on port $PORT"
else
    echo "❌ Network: Not listening on port $PORT"
fi

# Check storage usage
STORAGE_USED=\$(du -sh "$CARBIDE_DATA/storage" 2>/dev/null | cut -f1)
echo "💾 Storage Used: \${STORAGE_USED:-0B} / $STORAGE_SIZE"

# Show recent logs
echo "📝 Recent Logs:"
tail -n 5 "$CARBIDE_LOGS/provider.log" 2>/dev/null || echo "No logs yet"
EOF

    # Uninstall script
    cat > "$CARBIDE_BIN/carbide-uninstall" << EOF
#!/bin/bash
echo "🗑️  Uninstalling Carbide Provider..."

# Stop service
launchctl unload ~/Library/LaunchAgents/com.carbide.provider.plist 2>/dev/null || true

# Remove files
rm -rf "$CARBIDE_HOME"
rm -f ~/Library/LaunchAgents/com.carbide.provider.plist

# Remove from PATH
sed -i '' '/carbide/d' ~/.zshrc ~/.bashrc ~/.bash_profile 2>/dev/null || true

echo "✅ Carbide Provider uninstalled!"
EOF

    # Make scripts executable
    chmod +x "$CARBIDE_BIN"/*
    
    print_success "Management scripts created!"
}

setup_path() {
    print_step "Setting up PATH..."
    
    # Add to shell profiles
    SHELL_CONFIGS=("$HOME/.zshrc" "$HOME/.bashrc" "$HOME/.bash_profile")
    
    for config in "${SHELL_CONFIGS[@]}"; do
        if [[ -f "$config" ]]; then
            # Remove existing carbide entries
            sed -i '' '/# Carbide Node/d' "$config"
            sed -i '' '/carbide/d' "$config"
            
            # Add new entry
            echo "" >> "$config"
            echo "# Carbide Node" >> "$config"
            echo "export PATH=\"$CARBIDE_BIN:\$PATH\"" >> "$config"
        fi
    done
    
    # Also export for current session
    export PATH="$CARBIDE_BIN:$PATH"
    
    print_success "PATH configured!"
}

show_completion() {
    print_step "Installation completed!"
    
    echo -e "\n${GREEN}🎉 Carbide Provider successfully installed!${NC}\n"
    
    echo -e "${BLUE}📁 Installation Location:${NC} $CARBIDE_HOME"
    echo -e "${BLUE}💾 Storage Allocated:${NC} $STORAGE_SIZE at $CARBIDE_DATA/storage"
    echo -e "${BLUE}⚙️  Configuration:${NC} $CARBIDE_CONFIG/provider.toml"
    echo -e "${BLUE}📝 Logs:${NC} $CARBIDE_LOGS/"
    
    echo -e "\n${YELLOW}🎮 Management Commands:${NC}"
    echo "  carbide-start     - Start the provider"
    echo "  carbide-stop      - Stop the provider" 
    echo "  carbide-status    - Check provider status"
    echo "  carbide-uninstall - Remove Carbide completely"
    
    echo -e "\n${GREEN}🚀 Next Steps:${NC}"
    echo "1. Start a new terminal (to load PATH)"
    echo "2. Run: carbide-start"
    echo "3. Check status: carbide-status"
    echo "4. Your Mac mini is now a Carbide storage provider!"
    
    echo -e "\n${BLUE}💡 Pro Tips:${NC}"
    echo "• Provider auto-starts on boot"
    echo "• Earnings depend on network activity" 
    echo "• Monitor logs for debugging: tail -f $CARBIDE_LOGS/provider.log"
    echo "• Update config anytime: $CARBIDE_CONFIG/provider.toml"
    
    echo -e "\n${GREEN}Happy storing! 🌟${NC}"
}

main() {
    print_header
    
    echo "This will install Carbide Node as a storage provider on your Mac."
    echo "You'll be contributing $DEFAULT_STORAGE_SIZE to the decentralized storage network!"
    echo ""
    
    read -p "Continue with installation? (y/N): " CONFIRM
    if [[ ! "$CONFIRM" =~ ^[Yy]$ ]]; then
        echo "Installation cancelled."
        exit 0
    fi
    
    check_requirements
    create_directories
    build_carbide
    generate_config
    create_launch_daemon
    create_management_scripts
    setup_path
    show_completion
}

# Run main function
main "$@"