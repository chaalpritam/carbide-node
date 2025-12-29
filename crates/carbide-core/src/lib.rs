//! # Carbide Core
//!
//! Core data structures, types, and utilities shared across the Carbide Network.
//! This crate contains the fundamental building blocks for the decentralized storage marketplace.

#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic)]

// Re-export commonly used types for convenience
pub use chrono::{DateTime, Utc};
pub use rust_decimal::Decimal;
pub use uuid::Uuid;

// Core modules - we'll implement these in the next steps
pub mod crypto;
pub mod error;
pub mod network;
pub mod types;

// Re-exports for easy access
pub use error::{CarbideError, Result};
pub use types::*;
// pub use crypto::*;  // Will enable in Step 3
