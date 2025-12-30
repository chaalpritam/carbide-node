# 🖥️ Carbide Provider Desktop App

Beautiful, native desktop application for managing your Carbide Network storage provider.

## ✨ Features

### 🎯 **Installation Wizard**
- **Welcome Screen**: Beautiful introduction to Carbide Network
- **Configuration**: Easy setup for storage allocation, pricing, and provider details
- **Automated Installation**: One-click installation with real-time progress
- **Ready to Earn**: Seamless transition from setup to active earning

### 📊 **Real-Time Dashboard**
- **Provider Status**: Live status monitoring with visual indicators
- **Earnings Tracking**: Daily/monthly earnings with beautiful charts
- **Storage Management**: Visual storage usage with folder access
- **System Metrics**: CPU, memory, and disk usage monitoring
- **Reputation Score**: Live reputation tracking and history

### ⚙️ **Settings Management**
- **Provider Configuration**: Edit name, tier, region, and storage allocation
- **Network Settings**: Configure discovery endpoints and advertise addresses
- **Advanced Options**: Log levels, health check intervals, and reporting
- **Real-time Validation**: Port availability checking and configuration validation

### 📝 **Logs & Monitoring**
- **Live Log Viewer**: Real-time log streaming with syntax highlighting
- **Log Filtering**: Search and filter logs by level or content
- **Export Logs**: Download logs for debugging and support
- **Auto-refresh**: Configurable auto-refresh for real-time monitoring

### 🔔 **System Integration**
- **System Tray**: Minimize to system tray with quick controls
- **Auto-start**: Automatic startup on system boot
- **Native Look**: Native macOS design with beautiful animations
- **Notifications**: System notifications for important events

## 🚀 Getting Started

### Prerequisites
- macOS 10.15+ (Catalina or newer)
- Node.js 18+
- Rust toolchain
- Xcode Command Line Tools

### Development Setup

1. **Install Dependencies**
   ```bash
   cd gui
   npm install
   ```

2. **Install Tauri CLI**
   ```bash
   npm install -g @tauri-apps/cli
   ```

3. **Run Development Server**
   ```bash
   npm run tauri:dev
   ```

### Building for Production

1. **Build the App**
   ```bash
   npm run tauri:build
   ```

2. **Find Your App**
   The built app will be in `src-tauri/target/release/bundle/macos/`

### Installing the Built App

1. **Copy to Applications**
   ```bash
   cp -r src-tauri/target/release/bundle/macos/Carbide\ Provider.app /Applications/
   ```

2. **Launch**
   - Open from Applications folder
   - Or double-click the app bundle

## 🎨 UI Components

### Installation Wizard
- **Progress Tracking**: Real-time installation progress with detailed steps
- **Error Handling**: Clear error messages with recovery options
- **Configuration Preview**: Live preview of earnings potential
- **Beautiful Animations**: Smooth transitions and engaging visuals

### Dashboard Overview
- **Status Cards**: Quick glance at key metrics
- **Earnings Chart**: Historical earnings with trend analysis
- **Storage Visualization**: Interactive storage usage visualization
- **System Health**: Real-time system performance monitoring

### Settings Panel
- **Form Validation**: Real-time validation with helpful error messages
- **Smart Defaults**: Intelligent default values for optimal performance
- **Change Detection**: Visual indicators for unsaved changes
- **Port Checking**: Automatic port availability validation

### Logs Panel
- **Syntax Highlighting**: Color-coded log levels for easy reading
- **Real-time Streaming**: Live log updates with auto-scroll
- **Advanced Filtering**: Multiple filter options for log analysis
- **Export Functionality**: Easy log export for debugging

## 🛠️ Architecture

### Frontend (React + TypeScript)
```
src/
├── components/           # React components
│   ├── Dashboard.tsx     # Main dashboard
│   ├── InstallWizard.tsx # Installation wizard
│   ├── StatusCard.tsx    # Status display components
│   ├── EarningsChart.tsx # Earnings visualization
│   ├── StorageCard.tsx   # Storage management
│   ├── SystemMetricsCard.tsx # System monitoring
│   ├── LogsPanel.tsx     # Log viewer
│   └── SettingsPanel.tsx # Configuration management
├── types.ts              # TypeScript interfaces
├── App.tsx               # Main app component
├── main.tsx              # App entry point
└── styles.css            # Global styles
```

### Backend (Rust + Tauri)
```
src-tauri/src/
├── main.rs               # App initialization & system tray
├── commands.rs           # Tauri command handlers
├── provider_manager.rs   # Provider process management
└── system_info.rs        # System information utilities
```

## 🎯 Key Features Explained

### Installation Wizard
The installation wizard provides a guided setup experience:

1. **Welcome**: Introduces users to Carbide Network benefits
2. **Configuration**: Allows customization of provider settings
3. **Installation**: Automated installation with progress tracking
4. **Completion**: Success confirmation with next steps

### Real-time Monitoring
The dashboard provides comprehensive monitoring:

- **Live Updates**: Data refreshes every 5 seconds
- **Visual Indicators**: Color-coded status indicators
- **Interactive Charts**: Hover effects and detailed tooltips
- **Responsive Design**: Adapts to different screen sizes

### System Tray Integration
The app integrates seamlessly with macOS:

- **Background Operation**: Runs in system tray when minimized
- **Quick Actions**: Start/stop provider from tray menu
- **Native Menus**: Standard macOS menu structure
- **Notifications**: System notifications for events

## 🔧 Configuration

### Environment Variables
```bash
# Development
TAURI_DEBUG=true

# Production  
TAURI_BUNDLE_IDENTIFIER=com.carbide.provider
```

### Build Configuration
The app can be customized through `tauri.conf.json`:

- **App Identity**: Bundle identifier and metadata
- **Permissions**: File system and network access
- **Icons**: App icons for different sizes
- **System Tray**: Tray icon and menu configuration

## 🐛 Debugging

### Common Issues

1. **Build Failures**
   ```bash
   # Clear cache and rebuild
   rm -rf node_modules target
   npm install
   npm run tauri:build
   ```

2. **Permission Errors**
   ```bash
   # macOS may require permission approval
   # Go to System Preferences > Security & Privacy
   ```

3. **Port Conflicts**
   ```bash
   # Check for port conflicts
   lsof -i :8080
   ```

### Debug Mode
```bash
# Run with debug logging
RUST_LOG=debug npm run tauri:dev
```

## 📦 Distribution

### Mac App Store
To distribute through the Mac App Store:

1. **Code Signing**: Set up Apple Developer certificates
2. **Entitlements**: Configure app sandbox entitlements
3. **Validation**: Use Xcode to validate the build
4. **Submission**: Submit through App Store Connect

### Direct Distribution
For direct distribution:

1. **Notarization**: Notarize the app bundle
2. **DMG Creation**: Create installer DMG
3. **Distribution**: Host on website or GitHub releases

## 🤝 Contributing

1. **Setup**: Follow development setup instructions
2. **Branch**: Create feature branch from main
3. **Develop**: Make changes with tests
4. **Test**: Ensure all functionality works
5. **Submit**: Create pull request with description

## 📄 License

MIT License - see the project root for full license text.

---

## 🌟 Screenshots

### Installation Wizard
![Installation Wizard](screenshots/installer.png)

### Dashboard Overview  
![Dashboard](screenshots/dashboard.png)

### Settings Panel
![Settings](screenshots/settings.png)

### System Tray
![System Tray](screenshots/tray.png)

---

**Ready to turn your Mac mini into a profitable Carbide storage provider!** 🚀