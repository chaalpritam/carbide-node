# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Carbide Network is a **production-ready decentralized storage marketplace** built in Rust where anyone can provide storage capacity and earn rewards, while users get affordable, secure, and customizable data storage with user-defined replication factors and pricing.

**Current Status**: v1.0.0 - Production release with working Mac provider node, GUI installer, and complete backend infrastructure.

## Architecture Evolution

The project has evolved from a centralized approach to a working **decentralized marketplace**:

### Original Vision (Legacy)
- Centralized storage nodes controlled by user
- Client-server architecture with fixed replication
- Inspired by [rqbit](https://github.com/ikatson/rqbit) for networking patterns

### Current Implementation (v1.0.0)
- **Working storage provider node** - Mac mini optimized with 25GB allocation
- **Beautiful desktop GUI** - Tauri-based installer and dashboard
- **Complete backend** - All core crates implemented and tested
- **Professional installer** - DMG package with automated setup
- **Production monitoring** - Real-time dashboard and metrics
- **Auto-start capability** - macOS LaunchAgent integration

## Core Components Architecture

### Primary Crates Structure
```
carbide-node/crates/
├── carbide-core/              # ✅ Shared data structures and utilities
├── carbide-provider/          # ✅ Storage provider node implementation
├── carbide-discovery/         # ✅ Network discovery and marketplace
├── carbide-client/            # ✅ Client SDK for mobile/desktop apps
├── carbide-reputation/        # ✅ Reputation and trust system
└── carbide-crypto/           # ✅ Cryptographic proofs and encryption
```

### Desktop GUI Application
```
carbide-node/gui/
├── src/                       # ✅ React + TypeScript frontend
├── src-tauri/                 # ✅ Rust backend with Tauri
├── Installation Wizard        # ✅ Guided provider setup
├── Live Dashboard            # ✅ Real-time monitoring
├── Settings Panel            # ✅ Configuration management
└── Logs Viewer               # ✅ Live log streaming
```

### Implemented Components

#### 1. Storage Provider Node (`carbide-provider`) ✅
```bash
# Provider setup commands (WORKING)
cargo run --bin carbide-provider -- init --storage-path /data --capacity 25GB
cargo run --bin carbide-provider -- start --price-per-gb-month 0.005 --port 8080

# Or use the GUI installer
./build-gui.sh && open "gui/src-tauri/target/release/bundle/macos/Carbide Provider.app"
```

**Features Implemented**:
- HTTP API server with health checks
- Storage management and file operations
- Reputation tracking and reporting
- Configuration management
- Auto-start on macOS boot

#### 2. Discovery Service (`carbide-discovery`) ✅
**Features Implemented**:
- Provider registration and announcement
- Client provider selection algorithms
- Market pricing and reputation tracking
- Health check aggregation

#### 3. Client SDK (`carbide-client`) ✅
**Features Implemented**:
- Storage quote requests
- Provider health checking
- File upload/download protocols
- Status monitoring

## Development Setup

### Essential Commands
- `cargo build` - Build all crates
- `cargo test` - Run comprehensive test suite
- `cargo clippy` - Lint checking with strict rules
- `cargo fmt` - Code formatting
- `cargo nextest run` - Fast parallel testing (when available)

### Workspace Configuration
Use Cargo workspace for multi-crate management:
```toml
# Cargo.toml (workspace root)
[workspace]
members = [
    "crates/carbide-core",
    "crates/carbide-provider", 
    "crates/carbide-discovery",
    "crates/carbide-client",
    "crates/carbide-reputation",
    "crates/carbide-crypto",
    "crates/carbide-contracts"
]
```

### Key Dependencies
- **axum**: Modern async web framework for HTTP APIs
- **tokio**: Async runtime for networking
- **sqlx**: Async database operations
- **serde**: Serialization for data exchange
- **ring**: Cryptographic operations
- **prometheus**: Metrics collection
- **tracing**: Structured logging

## Development Patterns

### 1. Mobile-First Design
Always consider mobile constraints:
```rust
// Example: Battery-aware replication
match network_monitor.connection_type() {
    NetworkType::WiFi => replicate_to_all_providers(),
    NetworkType::Cellular => replicate_essential_only(),
    NetworkType::Offline => queue_for_later(),
}
```

### 2. Economic Model Integration
Build economic incentives into core logic:
```rust
struct StorageRequest {
    file: File,
    replication_factor: u8,      // User choice: 1-10
    max_price_per_gb_month: f64, // User budget
    provider_requirements: ProviderRequirements,
}
```

### 3. Reputation-Driven Selection
Provider selection based on multi-dimensional reputation:
```rust
struct ReputationScore {
    uptime: f64,           // Historical reliability
    data_integrity: f64,   // Proof-of-storage compliance
    response_time: f64,    // Performance metrics
    community_feedback: f64, // Peer ratings
}
```

## Current Implementation Status (v1.0.0)

### ✅ Production Release Complete
- [x] Core data structures (`carbide-core`)
- [x] Storage provider node (`carbide-provider`) with HTTP API
- [x] Discovery service (`carbide-discovery`) with provider registry
- [x] Client SDK (`carbide-client`) with CLI tools
- [x] Reputation system (`carbide-reputation`) with scoring
- [x] Cryptographic primitives (`carbide-crypto`)
- [x] Desktop GUI application (Tauri + React + TypeScript)
- [x] DMG installer for macOS
- [x] Installation and monitoring scripts
- [x] Comprehensive test suite
- [x] Working multi-provider demo

### 🎯 Current Capabilities
- **Mac Mini Provider**: Fully functional storage provider with 25GB allocation
- **GUI Dashboard**: Real-time monitoring with earnings tracking
- **Auto-Start**: macOS LaunchAgent integration for 24/7 operation
- **Multi-Provider Demo**: Working network with multiple providers and clients
- **Professional Installer**: DMG package with automated setup wizard

## Key Design Goals

### Decentralized Marketplace Features
- **User-configurable replication**: 1-10 copies based on user needs
- **Economic incentives**: Providers earn, users save 60-80% vs. centralized
- **Reputation system**: Multi-dimensional trust scoring
- **Mobile optimization**: Battery and bandwidth awareness
- **Proof systems**: Cryptographic verification of data integrity

### Technical Excellence
- **Performance**: Rust's zero-cost abstractions and memory safety
- **Scalability**: Modular architecture supporting horizontal scaling  
- **Security**: Client-side encryption and zero-knowledge storage
- **Reliability**: Byzantine fault tolerance and automated failover

## Documentation References

- **[DECENTRALIZED_ARCHITECTURE.md](./DECENTRALIZED_ARCHITECTURE.md)** - Complete marketplace system design
- **[README.md](./README.md)** - Project overview and getting started
- **[REPLICATION.md](./REPLICATION.md)** - Multi-provider replication strategies
- **[ARCHITECTURE.md](./ARCHITECTURE.md)** - Original centralized design (legacy)

## Related Components

- **Mobile Client**: Located at `/Users/chaalpritam/Blockbase/Carbide` (iOS/Android storage app)
- **Desktop Client**: Located at `/Users/chaalpritam/Blockbase/CarbideDrive` (Desktop sync application)

## Development Priorities

### ✅ Phase 1: Foundation (COMPLETED v1.0.0)
1. ✅ Core data structures and networking
2. ✅ Provider node with storage management
3. ✅ Discovery protocol implementation
4. ✅ Client SDK for mobile/desktop integration
5. ✅ Desktop GUI application with installer
6. ✅ Mac mini production deployment

### Phase 2: Advanced Features (Next)
1. Enhanced reputation algorithms with proof-of-storage
2. Mobile client applications (iOS/Android)
3. Advanced cryptographic proofs (PoRep, PoSt)
4. Dynamic pricing and market optimization
5. Cross-platform provider support (Windows, Linux)

### Phase 3: Economic Infrastructure
1. Token/payment system integration
2. Smart contract deployment for automated payments
3. Dispute resolution mechanisms
4. Global provider network expansion

### Phase 4: Scale & Production
1. Enterprise features and SLA support
2. Advanced monitoring and analytics
3. Multi-region replication strategies
4. Decentralized governance implementation

Focus on building a **sustainable ecosystem** where economic incentives create a thriving marketplace for decentralized storage.