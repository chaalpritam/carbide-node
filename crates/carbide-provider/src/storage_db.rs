//! SQLite-backed persistence for file metadata and storage contracts.
//!
//! On startup the in-memory caches (`files` and `contracts` HashMaps) are
//! warmed from the database so the hot path stays fast.  Every mutation
//! (store, upload, delete) is written through to SQLite so data survives
//! restarts.

use std::path::Path;
use std::sync::Mutex;

use carbide_core::{ContentHash, ContractStatus, StorageContract, Uuid};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};

use crate::server::StoredFile;

/// Thin wrapper around a SQLite connection for provider storage metadata.
///
/// The inner `Connection` is behind a `Mutex` because `rusqlite::Connection`
/// is `Send` but not `Sync`; the mutex makes `StorageDb` safe to share via
/// `Arc<StorageDb>` across Axum handlers.
pub struct StorageDb {
    conn: Mutex<Connection>,
}

impl std::fmt::Debug for StorageDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorageDb").finish()
    }
}

impl StorageDb {
    /// Open (or create) the database at the given path and run migrations.
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;

        // WAL mode for better concurrent read performance
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS stored_files (
                file_id       TEXT PRIMARY KEY,
                size          INTEGER NOT NULL,
                storage_path  TEXT NOT NULL,
                stored_at     TEXT NOT NULL,
                contract_id   TEXT NOT NULL,
                content_type  TEXT NOT NULL DEFAULT 'application/octet-stream',
                is_encrypted  INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS storage_contracts (
                id              TEXT PRIMARY KEY,
                request_id      TEXT NOT NULL,
                file_id         TEXT NOT NULL,
                provider_id     TEXT NOT NULL,
                price_per_gb    TEXT NOT NULL,
                duration_months INTEGER NOT NULL,
                started_at      TEXT NOT NULL,
                status          TEXT NOT NULL DEFAULT 'active',
                last_proof_at   TEXT,
                client_address  TEXT,
                provider_address TEXT,
                escrow_id       INTEGER,
                payment_status  TEXT,
                total_escrowed  TEXT,
                total_released  TEXT
            );",
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    // ------------------------------------------------------------------
    // Stored files
    // ------------------------------------------------------------------

    /// Persist a newly stored file record.
    pub fn insert_file(&self, file: &StoredFile) -> rusqlite::Result<()> {
        let conn = self.conn.lock().expect("db lock poisoned");
        conn.execute(
            "INSERT OR REPLACE INTO stored_files
                (file_id, size, storage_path, stored_at, contract_id, content_type, is_encrypted)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                file.file_id.to_hex(),
                file.size,
                file.storage_path,
                file.stored_at.to_rfc3339(),
                file.contract_id.to_string(),
                file.content_type,
                file.is_encrypted as i32,
            ],
        )?;
        Ok(())
    }

    /// Remove a file record by its content hash.
    pub fn delete_file(&self, file_id: &ContentHash) -> rusqlite::Result<()> {
        let conn = self.conn.lock().expect("db lock poisoned");
        conn.execute(
            "DELETE FROM stored_files WHERE file_id = ?1",
            params![file_id.to_hex()],
        )?;
        Ok(())
    }

    /// Load every stored file record (used to warm the in-memory cache on startup).
    pub fn load_all_files(&self) -> rusqlite::Result<Vec<StoredFile>> {
        let conn = self.conn.lock().expect("db lock poisoned");
        let mut stmt = conn.prepare(
            "SELECT file_id, size, storage_path, stored_at, contract_id, content_type, is_encrypted
             FROM stored_files",
        )?;

        let rows = stmt.query_map([], |row| {
            let file_id_hex: String = row.get(0)?;
            let stored_at_str: String = row.get(3)?;
            let contract_id_str: String = row.get(4)?;
            let is_encrypted_int: i32 = row.get(6)?;

            Ok(StoredFile {
                file_id: ContentHash::from_hex(&file_id_hex)
                    .unwrap_or_else(|_| ContentHash::from_data(b"invalid")),
                size: row.get(1)?,
                storage_path: row.get(2)?,
                stored_at: DateTime::parse_from_rfc3339(&stored_at_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                contract_id: Uuid::parse_str(&contract_id_str).unwrap_or_else(|_| Uuid::new_v4()),
                content_type: row.get(5)?,
                is_encrypted: is_encrypted_int != 0,
            })
        })?;

        rows.collect()
    }

    // ------------------------------------------------------------------
    // Storage contracts
    // ------------------------------------------------------------------

    /// Persist a new storage contract.
    pub fn insert_contract(&self, c: &StorageContract) -> rusqlite::Result<()> {
        let conn = self.conn.lock().expect("db lock poisoned");
        conn.execute(
            "INSERT OR REPLACE INTO storage_contracts
                (id, request_id, file_id, provider_id, price_per_gb, duration_months, started_at, status, last_proof_at,
                 client_address, provider_address, escrow_id, payment_status, total_escrowed, total_released)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                c.id.to_string(),
                c.request_id.to_string(),
                c.file_id.to_hex(),
                c.provider_id.to_string(),
                c.price_per_gb_month.to_string(),
                c.duration_months,
                c.started_at.to_rfc3339(),
                status_to_str(&c.status),
                c.last_proof_at.map(|t| t.to_rfc3339()),
                c.client_address,
                c.provider_address,
                c.escrow_id.map(|id| id as i64),
                c.payment_status,
                c.total_escrowed,
                c.total_released,
            ],
        )?;
        Ok(())
    }

    /// Load every contract (used to warm the in-memory cache on startup).
    pub fn load_all_contracts(&self) -> rusqlite::Result<Vec<StorageContract>> {
        let conn = self.conn.lock().expect("db lock poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, request_id, file_id, provider_id, price_per_gb,
                    duration_months, started_at, status, last_proof_at,
                    client_address, provider_address, escrow_id,
                    payment_status, total_escrowed, total_released
             FROM storage_contracts",
        )?;

        let rows = stmt.query_map([], |row| {
            let id_str: String = row.get(0)?;
            let request_id_str: String = row.get(1)?;
            let file_id_hex: String = row.get(2)?;
            let provider_id_str: String = row.get(3)?;
            let price_str: String = row.get(4)?;
            let started_at_str: String = row.get(6)?;
            let status_str: String = row.get(7)?;
            let last_proof_str: Option<String> = row.get(8)?;
            let escrow_id_raw: Option<i64> = row.get(11)?;

            Ok(StorageContract {
                id: Uuid::parse_str(&id_str).unwrap_or_else(|_| Uuid::new_v4()),
                request_id: Uuid::parse_str(&request_id_str).unwrap_or_else(|_| Uuid::new_v4()),
                file_id: ContentHash::from_hex(&file_id_hex)
                    .unwrap_or_else(|_| ContentHash::from_data(b"invalid")),
                provider_id: Uuid::parse_str(&provider_id_str).unwrap_or_else(|_| Uuid::new_v4()),
                price_per_gb_month: price_str
                    .parse()
                    .unwrap_or_else(|_| rust_decimal::Decimal::ZERO),
                duration_months: row.get(5)?,
                started_at: DateTime::parse_from_rfc3339(&started_at_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                status: str_to_status(&status_str),
                last_proof_at: last_proof_str.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .map(|dt| dt.with_timezone(&Utc))
                        .ok()
                }),
                client_address: row.get(9)?,
                provider_address: row.get(10)?,
                escrow_id: escrow_id_raw.map(|v| v as u64),
                payment_status: row.get(12)?,
                total_escrowed: row.get(13)?,
                total_released: row.get(14)?,
            })
        })?;

        rows.collect()
    }
}

// ------------------------------------------------------------------
// Helpers
// ------------------------------------------------------------------

fn status_to_str(status: &ContractStatus) -> &'static str {
    match status {
        ContractStatus::PendingDeposit => "pending_deposit",
        ContractStatus::Active => "active",
        ContractStatus::Completed => "completed",
        ContractStatus::Cancelled => "cancelled",
        ContractStatus::Failed => "failed",
        ContractStatus::Disputed => "disputed",
    }
}

fn str_to_status(s: &str) -> ContractStatus {
    match s {
        "pending_deposit" => ContractStatus::PendingDeposit,
        "completed" => ContractStatus::Completed,
        "cancelled" => ContractStatus::Cancelled,
        "failed" => ContractStatus::Failed,
        "disputed" => ContractStatus::Disputed,
        _ => ContractStatus::Active,
    }
}

#[cfg(test)]
mod tests {
    use carbide_core::{ContentHash, ContractStatus, StorageContract, Uuid};
    use chrono::Utc;
    use rust_decimal::Decimal;

    use super::*;

    #[test]
    fn test_roundtrip_file() {
        let dir = tempfile::tempdir().unwrap();
        let db = StorageDb::open(&dir.path().join("test.db")).unwrap();

        let file = StoredFile {
            file_id: ContentHash::from_data(b"hello"),
            size: 42,
            storage_path: "/tmp/hello.dat".to_string(),
            stored_at: Utc::now(),
            contract_id: Uuid::new_v4(),
            content_type: "text/plain".to_string(),
            is_encrypted: false,
        };

        db.insert_file(&file).unwrap();
        let loaded = db.load_all_files().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].file_id, file.file_id);
        assert_eq!(loaded[0].size, 42);

        db.delete_file(&file.file_id).unwrap();
        let loaded = db.load_all_files().unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_roundtrip_contract() {
        let dir = tempfile::tempdir().unwrap();
        let db = StorageDb::open(&dir.path().join("test.db")).unwrap();

        let contract = StorageContract {
            id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            file_id: ContentHash::from_data(b"contract_file"),
            provider_id: Uuid::new_v4(),
            price_per_gb_month: Decimal::new(5, 3),
            duration_months: 6,
            started_at: Utc::now(),
            status: ContractStatus::Active,
            last_proof_at: None,
            client_address: None,
            provider_address: None,
            escrow_id: None,
            payment_status: None,
            total_escrowed: None,
            total_released: None,
        };

        db.insert_contract(&contract).unwrap();
        let loaded = db.load_all_contracts().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, contract.id);
        assert_eq!(loaded[0].duration_months, 6);
    }
}
