// Carbide Provider GUI - Tauri Backend
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{
    CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu,
    SystemTrayMenuItem,
};

mod commands;
mod provider_manager;
mod system_info;

use commands::*;
use provider_manager::ProviderManager;

#[derive(Debug)]
pub struct AppState {
    pub carbide_home: PathBuf,
    pub provider_manager: Arc<Mutex<ProviderManager>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub running: bool,
    pub port: Option<u16>,
    pub name: String,
    pub storage_used_gb: f64,
    pub storage_total_gb: u64,
    pub earnings_today: f64,
    pub earnings_month: f64,
    pub uptime_hours: f64,
    pub connections: u32,
    pub reputation_score: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub disk_usage: f64,
    pub network_in: u64,
    pub network_out: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstallProgress {
    pub step: String,
    pub progress: u8,
    pub message: String,
    pub completed: bool,
    pub error: Option<String>,
}

fn create_system_tray() -> SystemTray {
    let quit = CustomMenuItem::new("quit".to_string(), "Quit Carbide Provider");
    let show = CustomMenuItem::new("show".to_string(), "Show Dashboard");
    let start = CustomMenuItem::new("start".to_string(), "Start Provider");
    let stop = CustomMenuItem::new("stop".to_string(), "Stop Provider");
    
    let tray_menu = SystemTrayMenu::new()
        .add_item(show)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(start)
        .add_item(stop)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);
    
    SystemTray::new().with_menu(tray_menu)
}

fn handle_system_tray_event(app: &tauri::AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::LeftClick {
            position: _,
            size: _,
            ..
        } => {
            // Show main window on left click
            if let Some(window) = app.get_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        SystemTrayEvent::MenuItemClick { id, .. } => {
            match id.as_str() {
                "quit" => {
                    std::process::exit(0);
                }
                "show" => {
                    if let Some(window) = app.get_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "start" => {
                    // TODO: Start provider
                }
                "stop" => {
                    // TODO: Stop provider
                }
                _ => {}
            }
        }
        _ => {}
    }
}

fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    let context = tauri::generate_context!();
    
    tauri::Builder::default()
        .system_tray(create_system_tray())
        .on_system_tray_event(handle_system_tray_event)
        .manage({
            let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            let carbide_home = PathBuf::from(&home_dir).join(".carbide");
            
            AppState {
                carbide_home: carbide_home.clone(),
                provider_manager: Arc::new(Mutex::new(ProviderManager::new(carbide_home))),
            }
        })
        .invoke_handler(tauri::generate_handler![
            check_installation,
            install_carbide,
            get_provider_status,
            get_system_metrics,
            start_provider,
            stop_provider,
            get_config,
            save_config,
            get_logs,
            open_storage_folder,
            get_system_info,
            check_port_available,
            estimate_earnings,
            send_notification,
            reset_provider,
            reinstall_provider
        ])
        .run(context)
        .expect("error while running tauri application");
}