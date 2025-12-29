# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Carbide Network is a **decentralized storage marketplace** built in Rust where anyone can provide storage capacity and earn rewards, while users get affordable, secure, and customizable data storage with user-defined replication factors and pricing.

## Architecture Evolution

The project has evolved from a centralized approach to a full **decentralized marketplace**:

### Original Vision (Legacy)
- Centralized storage nodes controlled by user
- Client-server architecture with fixed replication
- Inspired by [rqbit](https://github.com/ikatson/rqbit) for networking patterns

### Current Vision (Decentralized Marketplace)
- **Anyone can run storage providers** and earn money
- **Users choose replication factor** (1-10 copies) and pricing preferences
- **Economic incentives** align provider and user interests through reputation and payments
- **Mobile-optimized** protocols for battery/bandwidth efficiency

## Core Components Architecture

### Primary Crates Structure
```
carbide-node/crates/
├── carbide-core/              # Shared data structures and utilities
├── carbide-provider/          # Storage provider node implementation
├── carbide-discovery/         # Network discovery and marketplace
├── carbide-client/            # Client SDK for mobile/desktop apps
├── carbide-reputation/        # Reputation and trust system
├── carbide-crypto/           # Cryptographic proofs and encryption
└── carbide-contracts/        # Smart contract integration (optional)
```

### Key Components to Implement

#### 1. Storage Provider Node (`carbide-provider`)
```bash
# Provider setup commands
cargo run --bin carbide-provider -- init --storage-path /data --capacity 1TB
cargo run --bin carbide-provider -- start --price-per-gb-month 0.002
```

#### 2. Discovery Service (`carbide-discovery`)
- Provider registration and announcement
- Client provider selection algorithms
- Market pricing and reputation tracking

#### 3. Client SDK (`carbide-client`)
- User storage preference configuration
- Automatic provider selection based on user criteria
- Mobile-optimized sync protocols

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

## Current Implementation Status

### ✅ Architecture Design Complete
- Comprehensive marketplace design in `DECENTRALIZED_ARCHITECTURE.md`
- Multi-node replication strategies in `REPLICATION.md`
- Economic models and incentive structures defined

### 🔄 Ready for Implementation (Phase 1)
- [ ] Core data structures (`carbide-core`)
- [ ] Basic provider node (`carbide-provider`)  
- [ ] Simple discovery mechanism (`carbide-discovery`)
- [ ] Client SDK foundation (`carbide-client`)

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

### Phase 1: Foundation (Current Focus)
1. Implement core data structures and networking
2. Build basic provider node with storage management
3. Create simple discovery protocol
4. Develop client SDK for mobile/desktop integration

### Next Phases
2. Advanced reputation and economic systems
3. Cryptographic proof implementations
4. Mobile protocol optimizations
5. Smart contract integration for payments

Focus on building a **sustainable ecosystem** where economic incentives create a thriving marketplace for decentralized storage.