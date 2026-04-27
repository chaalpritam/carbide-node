# Changelog

All notable changes to the Carbide Network project will be documented in this file.

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
