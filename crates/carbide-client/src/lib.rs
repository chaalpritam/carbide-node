//! # Carbide Client Library
//!
//! HTTP client library for interacting with Carbide Network providers
//! and marketplace services. Provides both low-level client operations
//! and high-level storage management for easy application integration.

#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::wildcard_imports,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::doc_markdown,
    clippy::unnecessary_wraps,
    clippy::unused_self,
    clippy::unused_async,
    clippy::return_self_not_must_use,
    clippy::match_same_arms,
    clippy::needless_pass_by_value,
    clippy::similar_names,
    clippy::too_many_lines
)]

pub mod client;
pub mod discovery;
pub mod file_registry;
pub mod payment;
pub mod registry;
pub mod storage;
pub mod wallet;

// Re-exports for convenience
pub use client::{CarbideClient, ClientConfig, ProviderTestResult};
pub use discovery::{DiscoveryClient, MarketplaceQuery, ProviderFilter};
pub use registry::{ProviderRecord, RegistryClient};
pub use wallet::ClientWallet;
pub use storage::{
    simple, ProgressCallback, RetrieveResult, StorageLocation, StorageManager, StoragePreferences,
    StorageProgress, StoreResult,
};

// Re-export file registry types
pub use file_registry::{FileRecord, FileRegistry, ProviderLocation};

// Re-export crypto types for encryption support
pub use carbide_crypto::{EncryptionKey, KeyManager};
