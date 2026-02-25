//! TLS configuration and self-signed certificate generation.
//!
//! When `tls.enabled = true`, the provider binds with RusTLS instead of plain TCP.
//! If `tls.auto_generate = true`, a self-signed certificate is generated on first run
//! and saved to the configured paths.

use std::path::{Path, PathBuf};

use axum_server::tls_rustls::RustlsConfig;
use tracing::info;

/// TLS configuration
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// Whether TLS is enabled
    pub enabled: bool,
    /// Path to PEM certificate file
    pub cert_path: PathBuf,
    /// Path to PEM private key file
    pub key_path: PathBuf,
    /// Auto-generate a self-signed certificate if cert/key don't exist
    pub auto_generate: bool,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cert_path: PathBuf::from("certs/server.crt"),
            key_path: PathBuf::from("certs/server.key"),
            auto_generate: true,
        }
    }
}

/// Generate a self-signed certificate using rcgen and write it to the specified paths.
pub fn generate_self_signed(cert_path: &Path, key_path: &Path) -> anyhow::Result<()> {
    info!("Generating self-signed TLS certificate...");

    let cert = rcgen::generate_simple_self_signed(vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "0.0.0.0".to_string(),
    ])?;

    let cert_pem = cert.cert.pem();
    let key_pem = cert.key_pair.serialize_pem();

    // Create parent directories if needed
    if let Some(parent) = cert_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if let Some(parent) = key_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(cert_path, cert_pem)?;
    std::fs::write(key_path, key_pem)?;

    info!("Self-signed certificate written to {}", cert_path.display());
    info!("Private key written to {}", key_path.display());

    Ok(())
}

/// Load or generate TLS configuration for axum-server.
///
/// If `auto_generate` is true and cert/key files don't exist, generates self-signed certs first.
pub async fn load_rustls_config(tls_config: &TlsConfig) -> anyhow::Result<RustlsConfig> {
    if tls_config.auto_generate
        && (!tls_config.cert_path.exists() || !tls_config.key_path.exists())
    {
        generate_self_signed(&tls_config.cert_path, &tls_config.key_path)?;
    }

    let config =
        RustlsConfig::from_pem_file(&tls_config.cert_path, &tls_config.key_path).await?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_tls_config_default() {
        let config = TlsConfig::default();
        assert!(!config.enabled);
        assert!(config.auto_generate);
    }

    #[test]
    fn test_generate_self_signed() {
        let tmp = TempDir::new().unwrap();
        let cert_path = tmp.path().join("test.crt");
        let key_path = tmp.path().join("test.key");

        generate_self_signed(&cert_path, &key_path).unwrap();

        assert!(cert_path.exists());
        assert!(key_path.exists());

        let cert_contents = std::fs::read_to_string(&cert_path).unwrap();
        assert!(cert_contents.contains("BEGIN CERTIFICATE"));

        let key_contents = std::fs::read_to_string(&key_path).unwrap();
        assert!(key_contents.contains("BEGIN PRIVATE KEY"));
    }
}
