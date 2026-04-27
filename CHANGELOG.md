# Changelog

All notable changes to the Carbide Network project will be documented in this file.

## [Unreleased]

### Migration to Solana

- carbide-crypto wallet rebuilt around Ed25519 with SLIP-0010 derivation
  along Solana's standard BIP-44 path `m/44'/501'/0'/0'`. Wallets save in
  the encrypted Carbide format or the standard `solana-keygen` JSON layout.
- carbide-core gains a `SolanaConfig` (cluster, RPC URL, registry/escrow
  program IDs, USDC mint) with devnet defaults and `CARBIDE_SOLANA_*` env
  overrides.
- carbide-client adds a `RegistryClient` that walks the on-chain
  `carbide_registry` program with `getProgramAccounts` (filtered on the
  Anchor 8-byte discriminator) and borsh-decodes each provider record.
  The CLI regains `wallet {create,show,import}`.
- carbide-provider boots an on-chain auto-register flow: when the program
  ID is configured and `CARBIDE_WALLET_PASSWORD` is set, the node
  publishes (or refreshes) its provider PDA on startup. Failures are
  non-fatal — the HTTP API still serves.
- All previous Ethereum integration (ethers, secp256k1, EIP-712,
  Arbitrum) was removed in the prior cleanup pass.

## [1.0.0] - 2026-02-25

### Initial Production Release

First official release of Carbide Network - a decentralized storage marketplace built in Rust.

### Core Infrastructure

- **carbide-core**: Shared data structures, content addressing, basic encryption, and payment types
- **carbide-provider**: Full-featured storage provider node with HTTP API server
- **carbide-discovery**: Network discovery service with provider registry and marketplace
- **carbide-client**: Client SDK for mobile/desktop integration with CLI tools
- **carbide-reputation**: Multi-dimensional reputation and trust scoring system
- **carbide-crypto**: Cryptographic primitives including Ed25519 signing, AES-GCM encryption, and proof-of-storage helpers

### Storage Provider Features

- HTTP API server with health checks and file operations
- Storage management with configurable capacity (default 25GB)
- SQLite-backed file and contract metadata persistence
- Auth middleware with JWT and API key support
- TLS support with auto-generated self-signed certificates
- Per-IP rate limiting and input validation
- Prometheus metrics endpoint for monitoring
- Proof-of-storage verification and periodic scheduling
- Graceful shutdown with connection draining
- Structured JSON logging for production
- Config validation with environment variable overrides
- Discovery service registration with heartbeat
- Reputation event emission

### Client SDK Features

- Storage quote requests and provider health checking
- Multi-provider download with fallback via local file registry
- Discovery-mediated upload flow with contract creation
- Client-side encryption with AES-GCM

### Desktop GUI Application

- Native macOS app built with Tauri + React + TypeScript
- Installation wizard with guided provider setup
- Live dashboard with real-time earnings and performance monitoring
- Settings panel for complete provider configuration
- Log viewer with live streaming and filtering
- System tray integration for background operation
- Auto-start via macOS LaunchAgent

### DevOps & Tooling

- GitHub Actions CI pipeline (check, test, fmt, clippy, audit, release build)
- Dockerfile for containerized deployment
- DMG installer for macOS distribution
- Installation, monitoring, and cleanup scripts
- Comprehensive test suite across all crates
