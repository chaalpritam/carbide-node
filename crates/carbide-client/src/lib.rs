//! # Carbide Client Library
//!
//! HTTP client library for interacting with Carbide Network providers
//! and marketplace services.

#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic)]

pub mod client;

// Re-exports for convenience
pub use client::{CarbideClient, ClientConfig};