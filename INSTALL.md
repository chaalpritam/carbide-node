# 🍎 Carbide Node - Mac Installation Guide

Complete setup guide for running a Carbide storage provider on your Mac mini with 25GB allocated storage.

## 🚀 Quick Installation

### Option 1: Homebrew (Recommended for Mac mini)

The fastest path on macOS. Homebrew installs the binaries, writes a
default config, and registers a launchd service that auto-starts the
provider on boot.

```bash
# One-liner (installs Homebrew if missing, then the formula)
curl -fsSL https://raw.githubusercontent.com/chaalpritam/homebrew-carbide/master/install.sh | bash
```

Or manually:

```bash
brew tap chaalpritam/carbide https://github.com/chaalpritam/homebrew-carbide
brew install --HEAD chaalpritam/carbide/carbide-node
brew services start carbide-node
```

Edit `$(brew --prefix)/etc/carbide/provider.toml` (storage allocation,
price, region), then `brew services restart carbide-node` to apply.

### Option 2: Bundled Bash Installer

```bash
git clone https://github.com/chaalpritam/carbide-node.git
cd carbide-node
./install.sh
```

The installer will:
- ✅ Check system requirements (macOS, Apple Silicon recommended)
- ✅ Install Rust if needed
- ✅ Build Carbide binaries
- ✅ Create configuration with 25GB storage
- ✅ Set up auto-start on boot
- ✅ Create management commands

### Option 3: Manual Installation

```bash
# 1. Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 2. Build Carbide Node
cargo build --release

# 3. Create directory structure
mkdir -p ~/.carbide/{bin,data/storage,config,logs}

# 4. Copy binaries
cp target/release/carbide-* ~/.carbide/bin/

# 5. Generate default config
./scripts/generate-config.sh

# 6. Set up auto-start
cp scripts/com.carbide.provider.plist ~/Library/LaunchAgents/
```

## 📋 System Requirements

- **OS**: macOS 10.15+ (Catalina or newer)
- **Architecture**: Apple Silicon (M1/M2) recommended, Intel supported
- **Storage**: 30GB free space (25GB for provider + 5GB overhead)
- **Memory**: 4GB RAM minimum, 8GB recommended
- **Network**: Stable internet connection

## ⚙️ Configuration

Your provider configuration is stored at `~/.carbide/config/provider.toml`:

```toml
[provider]
name = "your-mac-mini-carbide-provider"
tier = "Home"
region = "NorthAmerica"
port = 8080
storage_path = "/Users/yourusername/.carbide/data/storage"
max_storage_gb = 25

[network]
discovery_endpoint = "http://localhost:3000"
advertise_address = "127.0.0.1:8080"

[pricing]
price_per_gb_month = 0.005  # $0.005/GB/month

[logging]
level = "info"
file = "/Users/yourusername/.carbide/logs/provider.log"

[reputation]
enable_reporting = true
health_check_interval = 300  # 5 minutes
```

### 🔧 Customization

Edit the configuration file to:
- Change storage allocation: `max_storage_gb = 50`
- Adjust pricing: `price_per_gb_month = 0.010`
- Change provider tier: `tier = "Professional"`
- Modify region: `region = "Europe"`

## 🎮 Management Commands

After installation, you'll have these commands available:

### Start Provider
```bash
carbide-start
```

### Stop Provider
```bash
carbide-stop
```

### Check Status
```bash
carbide-status
```

### Monitor Dashboard
```bash
./monitor.sh
```

### Uninstall
```bash
carbide-uninstall
```

## 📊 Monitoring Your Provider

### Real-time Dashboard
```bash
# Launch interactive monitoring dashboard
./monitor.sh
```

The dashboard shows:
- 🔧 Service status (running/stopped)
- 💾 Storage usage and available space
- 🌐 Network activity and connections
- 📊 Performance metrics (CPU, memory)
- 💰 Earnings estimates
- 📝 Recent logs

### Manual Status Check
```bash
# Check if provider is running
carbide-status

# View logs
tail -f ~/.carbide/logs/provider.log

# Check storage usage
du -sh ~/.carbide/data/storage

# Test API endpoint
curl http://localhost:8080/api/v1/provider/status
```

## 💰 Earnings Potential

### Calculation Example
- **Storage allocated**: 25GB
- **Price**: $0.005/GB/month
- **Max monthly earnings**: $0.125 (if fully utilized)
- **Daily potential**: ~$0.004

### Factors Affecting Earnings
- **Network demand**: More clients = more earnings
- **Uptime**: Higher uptime = better reputation = more selection
- **Performance**: Faster response times = higher reputation
- **Storage utilization**: Earnings only on stored data

## 🔧 Troubleshooting

### Provider Won't Start

1. **Check configuration**:
   ```bash
   cat ~/.carbide/config/provider.toml
   ```

2. **Verify storage directory**:
   ```bash
   ls -la ~/.carbide/data/storage
   ```

3. **Check logs**:
   ```bash
   tail -20 ~/.carbide/logs/provider.log
   ```

4. **Port conflicts**:
   ```bash
   lsof -i :8080
   ```

### macOS "App is Damaged" Error (Gatekeeper Issue)

**Problem**: When installing the DMG or .app bundle, macOS shows "Carbide Provider is damaged and can't be opened. You should move it to the Trash."

**Cause**: This is a macOS Gatekeeper security feature blocking unsigned applications. The app isn't actually damaged - it's just not code-signed with an Apple Developer certificate.

**Solution Options**:

#### Option 1: Remove Quarantine Flag (Recommended)
```bash
# If using the DMG installer
sudo xattr -cr "/Applications/Carbide Provider.app"

# If the app is in a different location
sudo xattr -cr "/path/to/Carbide Provider.app"

# Now try opening the app again
open "/Applications/Carbide Provider.app"
```

#### Option 2: Bypass Gatekeeper Temporarily
```bash
# Remove quarantine and allow the app to run
sudo spctl --master-disable  # Disable Gatekeeper temporarily
open "/Applications/Carbide Provider.app"
# After opening once, re-enable Gatekeeper:
sudo spctl --master-enable
```

#### Option 3: Right-Click Method
1. Locate "Carbide Provider.app" in Applications
2. **Right-click** (or Control-click) on the app
3. Select **"Open"** from the context menu
4. Click **"Open"** again in the security dialog
5. macOS will remember this and allow future launches

#### Option 4: System Settings (macOS Ventura 13+)
1. Try to open the app (it will be blocked)
2. Go to **System Settings** > **Privacy & Security**
3. Scroll down to **Security** section
4. Click **"Open Anyway"** next to the Carbide Provider message
5. Confirm by entering your password

#### Option 5: Build from Source (Most Secure)
```bash
# Building from source avoids all Gatekeeper issues
cargo build --release
# The locally built binary is automatically trusted
```

**After Using Any Solution**: The app should launch successfully. For production use, we recommend building from source or waiting for a code-signed release.

### Low Storage Utilization

1. **Check network connectivity**
2. **Verify discovery service registration**
3. **Monitor reputation score**
4. **Consider adjusting pricing**

### Auto-start Issues

```bash
# Check if launch daemon is loaded
launchctl list | grep carbide

# Manually load the service
launchctl load ~/Library/LaunchAgents/com.carbide.provider.plist

# Check service logs
cat ~/.carbide/logs/provider.err
```

## 🔒 Security Considerations

### Network Security
- Provider runs on localhost by default
- For external access, update `advertise_address` in config
- Consider firewall configuration for public providers

### Data Security
- All stored data is encrypted
- Private keys stored in `~/.carbide/keys/`
- Regular backups recommended

### Access Control
- Provider API has no authentication by default
- For production use, enable API authentication
- Monitor access logs regularly

## 📈 Optimizing Performance

### Mac Mini Specific Tips

1. **Enable Power Nap**: System Preferences > Energy Saver
2. **Disable Sleep**: Keep Mac mini always on
3. **SSD Storage**: Use SSD for better I/O performance
4. **Network**: Use wired Ethernet for stable connection
5. **Cooling**: Ensure adequate ventilation

### Provider Optimization

1. **Storage Path**: Use fastest available disk
2. **Log Rotation**: Configure log rotation to save space
3. **Health Checks**: Monitor with dashboard regularly
4. **Updates**: Keep Carbide Node updated

## 🆘 Support

### Getting Help
- **Logs**: Always include logs when asking for help
- **Configuration**: Share anonymized config if needed
- **System Info**: Include macOS version and hardware

### Useful Commands
```bash
# System info
system_profiler SPHardwareDataType SPSoftwareDataType

# Carbide info
carbide-provider --version
cat ~/.carbide/config/provider.toml

# Network info
ifconfig | grep inet
```

## 🔄 Updates

### Updating Carbide Node
```bash
# Pull latest changes
git pull origin main

# Rebuild
cargo build --release

# Update binaries
cp target/release/carbide-* ~/.carbide/bin/

# Restart provider
carbide-stop
carbide-start
```

### Backup Configuration
```bash
# Backup config and keys
tar -czf carbide-backup-$(date +%Y%m%d).tar.gz ~/.carbide/config ~/.carbide/keys
```

## 📜 File Locations

| Type | Location | Description |
|------|----------|-------------|
| Binaries | `~/.carbide/bin/` | Carbide executables |
| Config | `~/.carbide/config/provider.toml` | Provider configuration |
| Storage | `~/.carbide/data/storage/` | Actual stored files |
| Logs | `~/.carbide/logs/` | Application logs |
| Keys | `~/.carbide/keys/` | Encryption keys |
| Launch Agent | `~/Library/LaunchAgents/com.carbide.provider.plist` | Auto-start configuration |

---

🎉 **Congratulations!** Your Mac mini is now a Carbide Network storage provider, contributing to the decentralized storage marketplace!

**Next Steps:**
1. Run `./monitor.sh` to watch your provider in action
2. Monitor earnings and reputation growth
3. Consider upgrading to Professional tier as you gain reputation
4. Join the Carbide community for support and updates