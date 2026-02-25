//! # Carbide Core
//!
//! Core data structures, types, and utilities shared across the Carbide Network.
//! This crate contains the fundamental building blocks for the decentralized storage marketplace.

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
    clippy::items_after_statements,
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

// Re-export commonly used types for convenience
pub use chrono::{DateTime, Utc};
pub use rust_decimal::Decimal;
pub use uuid::Uuid;

// Core modules
pub mod crypto;
pub mod error;
pub mod network;
pub mod payment;
pub mod types;

// Re-exports for easy access
pub use error::{CarbideError, Result};
pub use types::*;
// pub use crypto::*;  // Will enable in Step 3
