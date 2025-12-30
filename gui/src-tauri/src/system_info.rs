// System information utilities

use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os_version: String,
    pub architecture: String,
    pub total_memory: String,
    pub processor: String,
    pub available_storage: u64,
}

impl SystemInfo {
    pub fn get() -> anyhow::Result<Self> {
        let os_version = get_macos_version()?;
        let architecture = std::env::consts::ARCH.to_string();
        let (total_memory, processor) = get_hardware_info()?;
        let available_storage = get_available_storage()?;

        Ok(Self {
            os_version,
            architecture,
            total_memory,
            processor,
            available_storage,
        })
    }
}

fn get_macos_version() -> anyhow::Result<String> {
    let output = Command::new("sw_vers")
        .arg("-productVersion")
        .output()?;
    
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn get_hardware_info() -> anyhow::Result<(String, String)> {
    let output = Command::new("system_profiler")
        .arg("SPHardwareDataType")
        .output()?;
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    
    let mut memory = "Unknown".to_string();
    let mut processor = "Unknown".to_string();
    
    for line in output_str.lines() {
        let line = line.trim();
        if line.starts_with("Memory:") {
            memory = line.replace("Memory:", "").trim().to_string();
        } else if line.starts_with("Chip:") || line.starts_with("Processor Name:") {
            processor = line.split(':').nth(1)
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "Unknown".to_string());
        }
    }
    
    Ok((memory, processor))
}

fn get_available_storage() -> anyhow::Result<u64> {
    let output = Command::new("df")
        .arg("-g")
        .arg("/")
        .output()?;
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    
    for line in output_str.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            if let Ok(available_gb) = parts[3].parse::<u64>() {
                return Ok(available_gb);
            }
        }
    }
    
    Ok(0)
}