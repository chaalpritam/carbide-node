//! # Carbide Client Library
//!
//! HTTP client library for interacting with Carbide Network providers
//! and marketplace services. Provides both low-level client operations
//! and high-level storage management for easy application integration.

#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic)]

pub mod client;
pub mod storage;
pub mod discovery;

// Re-exports for convenience
pub use client::{CarbideClient, ClientConfig, ProviderTestResult};
pub use storage::{
    StorageManager, StoragePreferences, StoreResult, RetrieveResult,
    StorageLocation, StorageProgress, ProgressCallback, simple,
};
pub use discovery::{DiscoveryClient, MarketplaceQuery, ProviderFilter};