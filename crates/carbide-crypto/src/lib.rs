//! # Carbide Crypto
//!
//! Cryptographic functions for the Carbide Network including:
//! - Content-addressed storage (like IPFS)
//! - File encryption/decryption
//! - Proof-of-storage mechanisms
//! - Key derivation and management

#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic)]

pub mod content_hash;
pub mod encryption;
pub mod proofs;

// Re-exports for convenience
pub use content_hash::*;
pub use encryption::*;
pub use proofs::*;
