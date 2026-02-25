//! Local file registry for tracking uploaded files across sessions.
//!
//! Persists file metadata, provider locations, and contract associations
//! in a local SQLite database so the client retains knowledge of its files
//! after restart.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

/// A record of a file stored in the Carbide network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    /// Content-hash file identifier (hex)
    pub file_id: String,
    /// Original filename
    pub original_name: String,
    /// Size in bytes
    pub file_size: u64,
    /// Whether the file was client-side encrypted
    pub is_encrypted: bool,
    /// Number of provider replicas
    pub replication_factor: u8,
    /// JSON array of provider locations
    pub providers: String,
    /// Status: "active", "expired", "deleted"
    pub status: String,
    /// ISO-8601 timestamp when stored
    pub stored_at: String,
}

/// A provider holding a copy of a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderLocation {
    /// Provider UUID
    pub provider_id: String,
    /// Provider HTTP endpoint
    pub endpoint: String,
    /// Associated contract ID on the discovery service
    pub contract_id: String,
}

/// Local SQLite-backed registry of files stored by this client.
pub struct FileRegistry {
    conn: Mutex<Connection>,
}

impl FileRegistry {
    /// Open (or create) a file registry database at the given path.
    pub fn open(path: &Path) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| format!("Failed to open DB: {e}"))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS file_registry (
                file_id TEXT PRIMARY KEY,
                original_name TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                is_encrypted INTEGER NOT NULL DEFAULT 0,
                replication_factor INTEGER NOT NULL DEFAULT 1,
                providers TEXT NOT NULL DEFAULT '[]',
                status TEXT NOT NULL DEFAULT 'active',
                stored_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )
        .map_err(|e| format!("Failed to create table: {e}"))?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Record a newly uploaded file.
    pub fn record_upload(&self, record: &FileRecord) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {e}"))?;
        conn.execute(
            "INSERT OR REPLACE INTO file_registry (file_id, original_name, file_size, is_encrypted, replication_factor, providers, status, stored_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                record.file_id,
                record.original_name,
                record.file_size as i64,
                record.is_encrypted as i32,
                record.replication_factor as i32,
                record.providers,
                record.status,
                record.stored_at,
            ],
        )
        .map_err(|e| format!("Insert failed: {e}"))?;
        Ok(())
    }

    /// Look up a single file by ID.
    pub fn get_file(&self, file_id: &str) -> Result<Option<FileRecord>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {e}"))?;
        let mut stmt = conn
            .prepare("SELECT file_id, original_name, file_size, is_encrypted, replication_factor, providers, status, stored_at FROM file_registry WHERE file_id = ?1")
            .map_err(|e| format!("Prepare failed: {e}"))?;

        let result = stmt
            .query_row(params![file_id], |row| {
                Ok(FileRecord {
                    file_id: row.get(0)?,
                    original_name: row.get(1)?,
                    file_size: row.get::<_, i64>(2)? as u64,
                    is_encrypted: row.get::<_, i32>(3)? != 0,
                    replication_factor: row.get::<_, i32>(4)? as u8,
                    providers: row.get(5)?,
                    status: row.get(6)?,
                    stored_at: row.get(7)?,
                })
            })
            .ok();

        Ok(result)
    }

    /// List files, optionally filtered by status.
    pub fn list_files(&self, status: Option<&str>) -> Result<Vec<FileRecord>, String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {e}"))?;

        let (query, param): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match status {
            Some(s) => (
                "SELECT file_id, original_name, file_size, is_encrypted, replication_factor, providers, status, stored_at FROM file_registry WHERE status = ?1 ORDER BY stored_at DESC".to_string(),
                vec![Box::new(s.to_string())],
            ),
            None => (
                "SELECT file_id, original_name, file_size, is_encrypted, replication_factor, providers, status, stored_at FROM file_registry ORDER BY stored_at DESC".to_string(),
                vec![],
            ),
        };

        let mut stmt = conn.prepare(&query).map_err(|e| format!("Prepare failed: {e}"))?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(param.iter()), |row| {
                Ok(FileRecord {
                    file_id: row.get(0)?,
                    original_name: row.get(1)?,
                    file_size: row.get::<_, i64>(2)? as u64,
                    is_encrypted: row.get::<_, i32>(3)? != 0,
                    replication_factor: row.get::<_, i32>(4)? as u8,
                    providers: row.get(5)?,
                    status: row.get(6)?,
                    stored_at: row.get(7)?,
                })
            })
            .map_err(|e| format!("Query failed: {e}"))?;

        let mut files = Vec::new();
        for row in rows {
            files.push(row.map_err(|e| format!("Row error: {e}"))?);
        }
        Ok(files)
    }

    /// Update a file's status.
    pub fn update_status(&self, file_id: &str, status: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("Lock error: {e}"))?;
        conn.execute(
            "UPDATE file_registry SET status = ?1 WHERE file_id = ?2",
            params![status, file_id],
        )
        .map_err(|e| format!("Update failed: {e}"))?;
        Ok(())
    }

    /// Get the provider locations for a file.
    pub fn get_providers_for_file(&self, file_id: &str) -> Result<Vec<ProviderLocation>, String> {
        let record = self.get_file(file_id)?;
        match record {
            Some(r) => {
                let providers: Vec<ProviderLocation> = serde_json::from_str(&r.providers)
                    .unwrap_or_default();
                Ok(providers)
            }
            None => Ok(vec![]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_registry() -> (FileRegistry, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test_files.db");
        let registry = FileRegistry::open(&db_path).unwrap();
        (registry, dir)
    }

    fn sample_record(file_id: &str) -> FileRecord {
        FileRecord {
            file_id: file_id.to_string(),
            original_name: "test.txt".to_string(),
            file_size: 1024,
            is_encrypted: false,
            replication_factor: 3,
            providers: serde_json::to_string(&vec![ProviderLocation {
                provider_id: "p1".to_string(),
                endpoint: "http://localhost:8080".to_string(),
                contract_id: "c1".to_string(),
            }])
            .unwrap(),
            status: "active".to_string(),
            stored_at: "2025-01-01T00:00:00".to_string(),
        }
    }

    #[test]
    fn open_creates_db_and_table() {
        let (registry, _dir) = temp_registry();
        // Should not panic and table should exist
        let files = registry.list_files(None).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn record_upload_and_get_file_roundtrip() {
        let (registry, _dir) = temp_registry();
        let record = sample_record("file-abc");

        registry.record_upload(&record).unwrap();
        let retrieved = registry.get_file("file-abc").unwrap().unwrap();

        assert_eq!(retrieved.file_id, "file-abc");
        assert_eq!(retrieved.original_name, "test.txt");
        assert_eq!(retrieved.file_size, 1024);
        assert_eq!(retrieved.status, "active");
    }

    #[test]
    fn list_files_with_status_filter() {
        let (registry, _dir) = temp_registry();

        registry.record_upload(&sample_record("f1")).unwrap();
        let mut expired = sample_record("f2");
        expired.status = "expired".to_string();
        registry.record_upload(&expired).unwrap();

        let active = registry.list_files(Some("active")).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].file_id, "f1");

        let all = registry.list_files(None).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn update_status_changes_status() {
        let (registry, _dir) = temp_registry();
        registry.record_upload(&sample_record("f1")).unwrap();

        registry.update_status("f1", "deleted").unwrap();

        let file = registry.get_file("f1").unwrap().unwrap();
        assert_eq!(file.status, "deleted");
    }

    #[test]
    fn get_providers_for_file_returns_locations() {
        let (registry, _dir) = temp_registry();
        registry.record_upload(&sample_record("f1")).unwrap();

        let providers = registry.get_providers_for_file("f1").unwrap();
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].provider_id, "p1");
        assert_eq!(providers[0].endpoint, "http://localhost:8080");
    }

    #[test]
    fn get_providers_for_missing_file_returns_empty() {
        let (registry, _dir) = temp_registry();
        let providers = registry.get_providers_for_file("nonexistent").unwrap();
        assert!(providers.is_empty());
    }
}
