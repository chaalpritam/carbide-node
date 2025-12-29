//! # Carbide Provider Library
//!
//! Core functionality for storage providers in the Carbide Network.
//! This library provides the HTTP server and business logic for
//! accepting storage requests and serving files.

#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic)]

pub mod server;

// Re-exports for convenience
pub use server::{ProviderServer, ServerConfig};