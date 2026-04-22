//! # Carbide Provider Library
//!
//! Core functionality for storage providers in the Carbide Network.
//! This library provides the HTTP server and business logic for
//! accepting storage requests and serving files.

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

pub mod auth;
pub mod config;
pub mod discovery_client;
pub mod metrics;
pub mod proof_scheduler;
pub mod rate_limit;
pub mod reputation_emitter;
pub mod server;
pub mod storage_db;
pub mod tls;

#[cfg(feature = "blockchain")]
pub mod contracts;
#[cfg(feature = "blockchain")]
pub mod payment;
#[cfg(feature = "blockchain")]
pub mod registry;

// Re-exports for convenience
pub use config::ProviderConfig;
pub use server::{ProviderServer, ServerConfig};
