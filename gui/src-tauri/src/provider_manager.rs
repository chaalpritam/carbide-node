// Provider management for the GUI backend

use crate::ProviderStatus;
use anyhow::Result;
use carbide_provider::ProviderConfig;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use tokio::fs;
use tokio::process::Command as AsyncCommand;

#[derive(Debug)]
pub struct ProviderManager {
    carbide_home: PathBuf,
    provider_process: Option<Child>,
}

impl ProviderManager {
    pub fn new(carbide_home: PathBuf) -> Self {
        Self {
            carbide_home,
            provider_process: None,
        }
    }

    pub async fn start(&mut self) -> Result<bool> {
        if self.is_running().await? {
            return Ok(true); // Already running
        }

        let config_path = self.carbide_home.join("config").join("provider.toml");
        let binary_path = self.carbide_home.join("bin").join("carbide-provider");

        if !config_path.exists() || !binary_path.exists() {
            return Err(anyhow::anyhow!("Carbide not installed"));
        }

        // Start the provider process
        let child = Command::new(&binary_path)
            .arg("--config")
            .arg(&config_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        self.provider_process = Some(child);

        // Wait a moment to ensure it starts properly
        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

        Ok(self.is_running().await?)
    }

    pub async fn stop(&mut self) -> Result<bool> {
        if let Some(mut child) = self.provider_process.take() {
            child.kill()?;
            let _ = child.wait();
        }

        // Also try to stop via launchctl if running as service
        let _ = AsyncCommand::new("launchctl")
            .arg("unload")
            .arg(format!("{}/Library/LaunchAgents/com.carbide.provider.plist", 
                        std::env::var("HOME").unwrap_or_default()))
            .output()
            .await;

        Ok(true)
    }

    pub async fn is_running(&self) -> Result<bool> {
        let config_path = self.carbide_home.join("config").join("provider.toml");
        
        if !config_path.exists() {
            return Ok(false);
        }

        // Load config to get port
        let config_content = fs::read_to_string(&config_path).await?;
        let config: ProviderConfig = toml::from_str(&config_content)?;
        let port = config.provider.port;

        // Check if port is being used
        use std::net::TcpStream;
        use std::time::Duration;
        
        match TcpStream::connect_timeout(
            &format!("127.0.0.1:{}", port).parse()?,
            Duration::from_millis(1000)
        ) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    pub async fn get_status(&self) -> Result<ProviderStatus> {
        let config_path = self.carbide_home.join("config").join("provider.toml");
        let storage_path = self.carbide_home.join("data").join("storage");

        // Load configuration
        let config = if config_path.exists() {
            let config_content = fs::read_to_string(&config_path).await?;
            toml::from_str::<ProviderConfig>(&config_content)?
        } else {
            return Err(anyhow::anyhow!("Configuration not found"));
        };

        let running = self.is_running().await?;

        // Calculate storage usage
        let (storage_used_gb, _files_count) = if storage_path.exists() {
            self.calculate_storage_usage(&storage_path).await?
        } else {
            (0.0, 0)
        };

        // Calculate earnings (simplified)
        let price_per_gb = config.pricing.price_per_gb_month;
        let earnings_month = storage_used_gb * price_per_gb;
        let earnings_today = earnings_month / 30.0;

        // Get uptime (simplified - in real implementation, track start time)
        let uptime_hours = if running { 1.0 } else { 0.0 };

        // Mock values for now
        let connections = if running { 2 } else { 0 };
        let reputation_score = 0.85; // Would come from reputation system

        Ok(ProviderStatus {
            running,
            port: Some(config.provider.port),
            name: config.provider.name,
            storage_used_gb,
            storage_total_gb: config.provider.max_storage_gb,
            earnings_today,
            earnings_month,
            uptime_hours,
            connections,
            reputation_score,
        })
    }

    async fn calculate_storage_usage(&self, storage_path: &PathBuf) -> Result<(f64, u32)> {
        let mut total_size = 0u64;
        let mut file_count = 0u32;

        let mut entries = fs::read_dir(storage_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let metadata = entry.metadata().await?;
            if metadata.is_file() {
                total_size += metadata.len();
                file_count += 1;
            }
        }

        let size_gb = total_size as f64 / (1024.0 * 1024.0 * 1024.0);
        Ok((size_gb, file_count))
    }

    pub async fn get_logs(&self, lines: usize) -> Result<Vec<String>> {
        let log_file = self.carbide_home.join("logs").join("provider.log");
        
        if !log_file.exists() {
            return Ok(vec!["No logs available yet".to_string()]);
        }

        let content = fs::read_to_string(&log_file).await?;
        let all_lines: Vec<&str> = content.lines().collect();
        let log_lines: Vec<String> = all_lines
            .iter()
            .rev()
            .take(lines)
            .rev()
            .map(|s| s.to_string())
            .collect();

        Ok(log_lines)
    }
}