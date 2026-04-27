//! # Carbide Crypto
//!
//! Cryptographic functions for the Carbide Network including:
//! - Content-addressed storage (like IPFS)
//! - File encryption/decryption
//! - Proof-of-storage mechanisms
//! - Key derivation and management

#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::doc_markdown,
    clippy::unnecessary_wraps,
    clippy::unused_self,
    clippy::return_self_not_must_use,
    clippy::match_same_arms,
    clippy::needless_pass_by_value,
    clippy::similar_names,
    clippy::too_many_lines
)]

pub mod content_hash;
pub mod encryption;
pub mod proofs;
pub mod signing;
pub mod wallet;

// Re-exports for convenience
pub use content_hash::*;
pub use encryption::*;
pub use proofs::*;
pub use signing::*;
