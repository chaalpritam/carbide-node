// Tauri commands for the GUI backend

use crate::{AppState, InstallProgress, ProviderStatus, SystemMetrics};
use anyhow::Result;
use tokio;
use carbide_provider::ProviderConfig;
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;
use tauri::{command, State, Window, Manager};
use tokio::fs;

#[command]
pub async fn check_installation(state: State<'_, AppState>) -> Result<bool, String> {
    let carbide_home = &state.carbide_home;
    let config_file = carbide_home.join("config").join("provider.toml");
    let binary_file = carbide_home.join("bin").join("carbide-provider");
    
    Ok(config_file.exists() && binary_file.exists())
}

#[command]
pub async fn install_carbide(
    window: Window,
    storage_gb: u64,
    provider_name: String,
    price_per_gb: f64,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let carbide_home = &state.carbide_home;
    
    // Create progress tracker
    let mut progress = InstallProgress {
        step: "Starting installation".to_string(),
        progress: 0,
        message: "Initializing Carbide installation...".to_string(),
        completed: false,
        error: None,
    };
    
    // Emit initial progress
    let _ = window.emit("install-progress", &progress);
    
    // Step 1: Create directories
    progress.step = "Creating directories".to_string();
    progress.progress = 10;
    progress.message = "Setting up directory structure...".to_string();
    let _ = window.emit("install-progress", &progress);
    
    let dirs = [
        carbide_home.join("bin"),
        carbide_home.join("config"),
        carbide_home.join("data").join("storage"),
        carbide_home.join("logs"),
        carbide_home.join("keys"),
    ];
    
    for dir in &dirs {
        if let Err(e) = fs::create_dir_all(dir).await {
            progress.error = Some(format!("Failed to create directory: {}", e));
            let _ = window.emit("install-progress", &progress);
            return Err(format!("Failed to create directory: {}", e));
        }
    }
    
    // Step 2: Check for Rust/Cargo
    progress.step = "Checking Rust installation".to_string();
    progress.progress = 20;
    progress.message = "Verifying Rust toolchain...".to_string();
    let _ = window.emit("install-progress", &progress);
    
    let cargo_check = Command::new("cargo").arg("--version").output();
    if cargo_check.is_err() {
        progress.step = "Installing Rust".to_string();
        progress.progress = 25;
        progress.message = "Installing Rust toolchain...".to_string();
        let _ = window.emit("install-progress", &progress);
        
        // Install Rust (simplified - in real implementation, you'd download and run rustup)
        progress.error = Some("Rust not found. Please install Rust manually and restart.".to_string());
        let _ = window.emit("install-progress", &progress);
        return Err("Rust not found".to_string());
    }
    
    // Step 3: Build Carbide binaries
    progress.step = "Building Carbide binaries".to_string();
    progress.progress = 40;
    progress.message = "Compiling Carbide Node (this may take several minutes)...".to_string();
    let _ = window.emit("install-progress", &progress);
    
    // Get the project root (parent of gui directory)
    let project_root = carbide_home.parent()
        .and_then(|p| p.parent())
        .unwrap_or_else(|| std::env::current_dir().unwrap().as_path());
    
    let build_output = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("--bin")
        .arg("carbide-provider")
        .current_dir(project_root)
        .output();
    
    match build_output {
        Ok(output) if output.status.success() => {
            progress.progress = 60;
            progress.message = "Build completed successfully".to_string();
            let _ = window.emit("install-progress", &progress);
        }
        Ok(output) => {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            progress.error = Some(format!("Build failed: {}", error_msg));
            let _ = window.emit("install-progress", &progress);
            return Err("Build failed".to_string());
        }
        Err(e) => {
            progress.error = Some(format!("Failed to run cargo: {}", e));
            let _ = window.emit("install-progress", &progress);
            return Err("Failed to run cargo".to_string());
        }
    }
    
    // Step 4: Copy binaries
    progress.step = "Installing binaries".to_string();
    progress.progress = 70;
    progress.message = "Installing Carbide binaries...".to_string();
    let _ = window.emit("install-progress", &progress);
    
    let source_binary = project_root.join("target").join("release").join("carbide-provider");
    let dest_binary = carbide_home.join("bin").join("carbide-provider");
    
    if let Err(e) = fs::copy(&source_binary, &dest_binary).await {
        progress.error = Some(format!("Failed to copy binary: {}", e));
        let _ = window.emit("install-progress", &progress);
        return Err("Failed to copy binary".to_string());
    }
    
    // Make binary executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&dest_binary).await
            .map_err(|e| format!("Failed to get binary permissions: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dest_binary, perms).await
            .map_err(|e| format!("Failed to set binary permissions: {}", e))?;
    }
    
    // Step 5: Generate configuration
    progress.step = "Configuring provider".to_string();
    progress.progress = 80;
    progress.message = "Generating provider configuration...".to_string();
    let _ = window.emit("install-progress", &progress);
    
    let mut config = ProviderConfig::default();
    config.provider.name = provider_name;
    config.provider.max_storage_gb = storage_gb;
    config.pricing.price_per_gb_month = price_per_gb;
    config.provider.storage_path = carbide_home.join("data").join("storage");
    config.logging.file = carbide_home.join("logs").join("provider.log");
    
    let config_path = carbide_home.join("config").join("provider.toml");
    if let Err(e) = config.save_to_file(&config_path).await {
        progress.error = Some(format!("Failed to save config: {}", e));
        let _ = window.emit("install-progress", &progress);
        return Err("Failed to save config".to_string());
    }
    
    // Step 6: Setup auto-start (macOS)
    progress.step = "Setting up auto-start".to_string();
    progress.progress = 90;
    progress.message = "Configuring auto-start service...".to_string();
    let _ = window.emit("install-progress", &progress);
    
    // Create launch daemon plist
    let plist_content = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.carbide.provider</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}/bin/carbide-provider</string>
        <string>--config</string>
        <string>{}/config/provider.toml</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>{}/logs/provider.out</string>
    <key>StandardErrorPath</key>
    <string>{}/logs/provider.err</string>
</dict>
</plist>"#, 
        carbide_home.display(),
        carbide_home.display(),
        carbide_home.display(),
        carbide_home.display()
    );
    
    let home_dir = std::env::var("HOME").unwrap_or_default();
    let plist_path = PathBuf::from(&home_dir)
        .join("Library")
        .join("LaunchAgents")
        .join("com.carbide.provider.plist");
    
    // Create LaunchAgents directory if it doesn't exist
    if let Some(parent) = plist_path.parent() {
        let _ = fs::create_dir_all(parent).await;
    }
    
    if let Err(e) = fs::write(&plist_path, plist_content).await {
        // Non-fatal error - continue without auto-start
        tracing::warn!("Failed to create launch daemon: {}", e);
    }
    
    // Step 7: Complete
    progress.step = "Installation complete".to_string();
    progress.progress = 100;
    progress.message = "Carbide Provider installed successfully!".to_string();
    progress.completed = true;
    let _ = window.emit("install-progress", &progress);
    
    Ok(true)
}

#[command]
pub async fn get_provider_status(state: State<'_, AppState>) -> Result<ProviderStatus, String> {
    let manager = state.provider_manager.lock().map_err(|e| e.to_string())?;
    manager.get_status().await.map_err(|e| e.to_string())
}

#[command]
pub async fn get_system_metrics() -> Result<SystemMetrics, String> {
    // Get system metrics using system commands
    let cpu_usage = get_cpu_usage().unwrap_or(0.0);
    let memory_usage = get_memory_usage().unwrap_or(0.0);
    let disk_usage = get_disk_usage().unwrap_or(0.0);
    
    Ok(SystemMetrics {
        cpu_usage,
        memory_usage,
        disk_usage,
        network_in: 0,  // TODO: Implement network metrics
        network_out: 0,
    })
}

#[command]
pub async fn start_provider(state: State<'_, AppState>) -> Result<bool, String> {
    let mut manager = state.provider_manager.lock().map_err(|e| e.to_string())?;
    manager.start().await.map_err(|e| e.to_string())
}

#[command]
pub async fn stop_provider(state: State<'_, AppState>) -> Result<bool, String> {
    let mut manager = state.provider_manager.lock().map_err(|e| e.to_string())?;
    manager.stop().await.map_err(|e| e.to_string())
}

#[command]
pub async fn get_config(state: State<'_, AppState>) -> Result<Value, String> {
    let config_path = state.carbide_home.join("config").join("provider.toml");
    
    if !config_path.exists() {
        return Err("Configuration file not found".to_string());
    }
    
    let config_content = fs::read_to_string(&config_path).await
        .map_err(|e| format!("Failed to read config: {}", e))?;
    
    let config: ProviderConfig = toml::from_str(&config_content)
        .map_err(|e| format!("Failed to parse config: {}", e))?;
    
    serde_json::to_value(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))
}

#[command]
pub async fn save_config(config: Value, state: State<'_, AppState>) -> Result<bool, String> {
    let config: ProviderConfig = serde_json::from_value(config)
        .map_err(|e| format!("Invalid config format: {}", e))?;
    
    let config_path = state.carbide_home.join("config").join("provider.toml");
    
    config.save_to_file(&config_path).await
        .map_err(|e| format!("Failed to save config: {}", e))?;
    
    Ok(true)
}

#[command]
pub async fn get_logs(lines: Option<usize>, state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let log_file = state.carbide_home.join("logs").join("provider.log");
    
    if !log_file.exists() {
        return Ok(vec!["No logs available yet".to_string()]);
    }
    
    let content = fs::read_to_string(&log_file).await
        .map_err(|e| format!("Failed to read logs: {}", e))?;
    
    let lines = lines.unwrap_or(50);
    let log_lines: Vec<String> = content
        .lines()
        .rev()
        .take(lines)
        .rev()
        .map(|s| s.to_string())
        .collect();
    
    Ok(log_lines)
}

#[command]
pub async fn open_storage_folder(state: State<'_, AppState>) -> Result<(), String> {
    let storage_path = state.carbide_home.join("data").join("storage");
    
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&storage_path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    
    Ok(())
}

#[command]
pub async fn get_system_info() -> Result<Value, String> {
    let output = Command::new("system_profiler")
        .args(&["SPHardwareDataType", "SPSoftwareDataType"])
        .output()
        .map_err(|e| format!("Failed to get system info: {}", e))?;
    
    let info_text = String::from_utf8_lossy(&output.stdout);
    
    // Parse relevant information
    let mut info = serde_json::Map::new();
    info.insert("raw".to_string(), Value::String(info_text.to_string()));
    
    Ok(Value::Object(info))
}

#[command]
pub async fn check_port_available(port: u16) -> Result<bool, String> {
    use std::net::TcpListener;
    
    match TcpListener::bind(format!("127.0.0.1:{}", port)) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

#[command]
pub async fn estimate_earnings(
    storage_used_gb: f64,
    price_per_gb: f64,
) -> Result<(f64, f64), String> {
    let monthly_earnings = storage_used_gb * price_per_gb;
    let daily_earnings = monthly_earnings / 30.0;
    
    Ok((daily_earnings, monthly_earnings))
}

#[command]
pub async fn send_notification(
    window: Window,
    title: String,
    body: String,
) -> Result<(), String> {
    use tauri::api::notification::Notification;
    
    Notification::new(&window.app_handle().config().tauri.bundle.identifier)
        .title(&title)
        .body(&body)
        .show()
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

// Helper functions
fn get_cpu_usage() -> Result<f64> {
    // Use top command to get CPU usage
    let output = Command::new("top")
        .args(&["-l", "1", "-n", "0"])
        .output()?;
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    
    // Parse CPU usage from top output
    for line in output_str.lines() {
        if line.starts_with("CPU usage:") {
            // Extract percentage
            if let Some(usage_str) = line.split(' ').find(|s| s.ends_with('%')) {
                if let Ok(usage) = usage_str.trim_end_matches('%').parse::<f64>() {
                    return Ok(usage);
                }
            }
        }
    }
    
    Ok(0.0)
}

fn get_memory_usage() -> Result<f64> {
    let output = Command::new("vm_stat").output()?;
    let output_str = String::from_utf8_lossy(&output.stdout);
    
    // Parse memory statistics
    let mut pages_free = 0;
    let mut pages_wired = 0;
    let mut pages_active = 0;
    let mut pages_inactive = 0;
    
    for line in output_str.lines() {
        if line.starts_with("Pages free:") {
            pages_free = parse_pages(line);
        } else if line.starts_with("Pages wired down:") {
            pages_wired = parse_pages(line);
        } else if line.starts_with("Pages active:") {
            pages_active = parse_pages(line);
        } else if line.starts_with("Pages inactive:") {
            pages_inactive = parse_pages(line);
        }
    }
    
    let total_pages = pages_free + pages_wired + pages_active + pages_inactive;
    let used_pages = pages_wired + pages_active;
    
    if total_pages > 0 {
        Ok((used_pages as f64 / total_pages as f64) * 100.0)
    } else {
        Ok(0.0)
    }
}

fn get_disk_usage() -> Result<f64> {
    let output = Command::new("df")
        .args(&["-h", "/"])
        .output()?;
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    
    for line in output_str.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 5 {
            if let Ok(usage) = parts[4].trim_end_matches('%').parse::<f64>() {
                return Ok(usage);
            }
        }
    }
    
    Ok(0.0)
}

fn parse_pages(line: &str) -> u64 {
    line.split_whitespace()
        .nth(2)
        .and_then(|s| s.trim_end_matches('.').parse().ok())
        .unwrap_or(0)
}