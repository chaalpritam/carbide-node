# Carbide Network - Decentralized Storage Marketplace

Carbide Network transforms data storage through a **decentralized marketplace** where anyone can provide storage capacity and earn rewards, while users get affordable, secure, and customizable data storage with user-defined replication factors and pricing.

## 🚀 Vision

**Democratize data storage** by creating a peer-to-peer marketplace where:
- **Anyone can earn** by contributing spare storage capacity
- **Users choose** their own replication factor (1-10 copies) and pricing preferences
- **Mobile-first design** optimized for bandwidth and battery constraints
- **Economic incentives** align provider and user interests

## 🏗️ Architecture Overview

### For Storage Providers
Run a **Carbide Provider Node** to monetize your spare storage:

```bash
# Build from source
cargo build --release

# Initialize a provider node
cargo run --bin carbide-provider -- init \
  --storage-path ./storage \
  --capacity 1TB \
  --tier home \
  --region northamerica

# Start the provider node
cargo run --bin carbide-provider -- start \
  --name "My Storage Provider" \
  --price-per-gb-month 0.002 \
  --port 8080 \
  --tier home \
  --region northamerica \
  --capacity-gb 100
```

**Provider Types:**
- **🏠 Home Users**: Spare disk space, earn ~$0.002/GB/month
- **🏢 Professionals**: Business storage, higher uptime guarantees  
- **🏭 Enterprise**: Data center grade, premium pricing
- **🌐 GlobalCDN**: High-performance edge storage

### For Data Users
**Mobile and desktop clients** with customizable storage tiers:

```toml
# Critical files: 5 copies, enterprise providers, $0.01/GB/month
# Important files: 3 copies, mixed providers, $0.005/GB/month
# Backup files: 2 copies, home providers, $0.002/GB/month
```

## 🎯 Key Features

### **User-Configurable Replication**
Unlike fixed replication in existing solutions, users choose their own safety vs. cost trade-offs:
- **Critical data**: 5+ copies across enterprise providers
- **Important data**: 3 copies across mixed provider types
- **Backup data**: 2 copies on cost-effective home providers

### **Smart Provider Selection**
Automated selection based on:
- **🏆 Reputation score** (uptime, data integrity, compliance)
- **💰 Competitive pricing** within user's budget
- **📍 Geographic distribution** for redundancy
- **⚡ Performance requirements** (latency, bandwidth)

### **Mobile Optimizations**
- **📶 WiFi**: Full replication to all selected providers
- **📱 Cellular**: Essential replication only (save bandwidth/battery)
- **✈️ Offline**: Queue uploads for optimal network conditions

### **Economic Incentives**
- **60-80% cheaper** than centralized storage providers
- **Proof-of-Storage** ensures data integrity through cryptographic verification
- **Reputation system** rewards reliable providers with more customers
- **Smart contracts** automate payments and handle disputes

## 🛠️ Technology Stack

- **Language**: Rust (performance, safety, concurrency)
- **Networking**: HTTP/2 + WebSocket (efficient mobile protocols)
- **Cryptography**: Content-addressed storage + end-to-end encryption
- **Economics**: Token-based payments + reputation scoring
- **Consensus**: Proof-of-Replication + Proof-of-Spacetime

**Inspired by**: [rqbit](https://github.com/ikatson/rqbit) for efficient P2P networking patterns

## 📁 Project Structure

```
carbide-node/
├── README.md                     # This file
├── ARCHITECTURE.md               # Centralized architecture (legacy)
├── DECENTRALIZED_ARCHITECTURE.md # Complete marketplace design  
├── REPLICATION.md                # Multi-node replication strategies
├── IMPLEMENTATION_PLAN.md        # Detailed development roadmap
├── CLAUDE.md                     # Development guidance
├── Cargo.toml                    # Workspace configuration
└── crates/                       # Rust implementation
    ├── carbide-core/             # Core data structures and types
    ├── carbide-crypto/           # Cryptographic primitives
    ├── carbide-provider/         # Storage provider node (CLI + server)
    ├── carbide-discovery/        # Network discovery service
    ├── carbide-client/           # Client SDK and CLI
    └── carbide-reputation/       # Reputation scoring system
```

## 🚦 Getting Started

### Prerequisites

- **Rust** 1.70+ ([install](https://www.rust-lang.org/tools/install))
- **Cargo** (comes with Rust)
- **Git** for cloning the repository

### Building the Project

```bash
# Clone the repository
git clone <repository-url>
cd carbide-node

# Build all crates
cargo build --release

# Run tests
cargo test --workspace

# Build specific binary
cargo build --release --bin carbide-provider
cargo build --release --bin carbide-discovery
cargo build --release --bin carbide-client
```

### Quick Start Example

**1. Start Discovery Service** (Terminal 1):
```bash
cargo run --bin carbide-discovery -- start \
  --port 9090 \
  --host 0.0.0.0
```

**2. Start Provider Node** (Terminal 2):
```bash
cargo run --bin carbide-provider -- start \
  --name "Home Provider" \
  --price-per-gb-month 0.002 \
  --port 8080 \
  --tier home \
  --capacity-gb 100
```

**3. Test with Client** (Terminal 3):
```bash
# Check provider health
cargo run --bin carbide-client -- health \
  --endpoint http://localhost:8080

# Get provider status
cargo run --bin carbide-client -- status \
  --endpoint http://localhost:8080

# Request storage quote
cargo run --bin carbide-client -- quote \
  --file-size 1048576 \
  --replication 3 \
  --duration 12 \
  --providers http://localhost:8080
```

### As a Storage Provider
1. **Build** the provider binary: `cargo build --release --bin carbide-provider`
2. **Initialize** storage: `carbide-provider init --storage-path ./storage --capacity 1TB`
3. **Start** the provider: `carbide-provider start --price-per-gb-month 0.002`
4. **Register** with discovery service to start receiving storage requests

### As a Data User  
1. **Build** the client: `cargo build --release --bin carbide-client`
2. **Configure** storage preferences (replication, budget, provider types)
3. **Upload** files with automatic provider selection

### As a Developer
1. **Read** `DECENTRALIZED_ARCHITECTURE.md` for complete system design
2. **Review** `CLAUDE.md` for development setup and patterns
3. **Check** `IMPLEMENTATION_PLAN.md` for detailed roadmap
4. **Explore** the modular crate structure for implementation details
5. **Run** examples: `cargo run --example <example-name> --package <crate-name>`

## 🗺️ Roadmap

### **Phase 1: Basic Marketplace** (Months 1-3)
- ✅ Architecture design complete
- ✅ Rust workspace and crate structure
- ✅ Core data structures and types
- ✅ Provider node implementation (CLI + HTTP server)
- ✅ Discovery service implementation
- ✅ Client SDK and CLI tools
- ✅ Cryptographic primitives (content hashing, encryption)
- ✅ Basic reputation system structure
- ⏳ File upload/download implementation
- ⏳ Provider registration and heartbeat
- ⏳ Basic replication with user choice

### **Phase 2: Advanced Features** (Months 4-6)
- ⏳ Complete reputation system implementation
- ⏳ Mobile-optimized protocols
- ⏳ Dynamic pricing algorithms
- ⏳ Provider performance monitoring
- ⏳ Storage proof generation and verification

### **Phase 3: Economic Infrastructure** (Months 7-9)
- ⏳ Token/payment system integration
- ⏳ Smart contract deployment
- ⏳ Automated dispute resolution
- ⏳ Advanced proof systems (PoRep, PoSt)

### **Phase 4: Scale & Optimize** (Months 10-12)
- ⏳ Decentralized governance
- ⏳ Cross-chain integration  
- ⏳ Enterprise features
- ⏳ Global network launch

## 🏢 Related Projects

- **Mobile Client**: `Carbide` - iOS/Android app for data storage
- **Desktop Client**: `CarbideDrive` - Desktop sync application

## 📦 Crates Overview

- **carbide-core**: Core data structures, types, and network protocols
- **carbide-crypto**: Cryptographic primitives (content hashing, encryption, proofs)
- **carbide-provider**: Storage provider node with HTTP API and CLI
- **carbide-discovery**: Provider discovery and marketplace coordination service
- **carbide-client**: Client SDK and CLI for interacting with the network
- **carbide-reputation**: Reputation scoring and tracking system

## 🤝 Contributing

This project aims to democratize data storage through economic incentives and user choice. Contributions welcome in:

- **Core Implementation**: Rust networking and storage systems
- **Mobile Optimization**: Battery and bandwidth efficient protocols  
- **Economic Modeling**: Pricing strategies and reputation algorithms
- **Security Review**: Cryptographic proofs and privacy mechanisms

## 📚 Documentation

- **[DECENTRALIZED_ARCHITECTURE.md](./DECENTRALIZED_ARCHITECTURE.md)** - Complete marketplace system design
- **[ARCHITECTURE.md](./ARCHITECTURE.md)** - Original centralized architecture
- **[REPLICATION.md](./REPLICATION.md)** - Multi-provider replication strategies
- **[IMPLEMENTATION_PLAN.md](./IMPLEMENTATION_PLAN.md)** - Detailed development roadmap and learning guide
- **[CLAUDE.md](./CLAUDE.md)** - Developer setup and guidance

## 🔧 Development

### Running Examples

Each crate includes example code demonstrating key functionality:

```bash
# Core marketplace demo
cargo run --example marketplace_demo --package carbide-core

# Networking demo
cargo run --example networking_demo --package carbide-core

# Provider demo
cargo run --example provider_demo --package carbide-provider

# Client SDK demo
cargo run --example client_sdk_demo --package carbide-client

# Crypto demo
cargo run --example crypto_demo --package carbide-crypto
```

### Testing

```bash
# Run all tests
cargo test --workspace

# Run tests for specific crate
cargo test --package carbide-core

# Run with output
cargo test --workspace -- --nocapture

# Run integration tests
cargo test --package tests
```

### Code Quality

```bash
# Format code
cargo fmt --all

# Lint code
cargo clippy --workspace -- -D warnings

# Check documentation
cargo doc --workspace --no-deps --open
```

## 🎖️ Competitive Advantages

| Feature | Carbide Network | Filecoin | Storj | Traditional Cloud |
|---------|----------------|----------|--------|-------------------|
| **User-Configurable Replication** | ✅ 1-10 copies | ❌ Fixed | ❌ Fixed | ❌ Fixed |
| **Mobile-Optimized** | ✅ Battery/bandwidth aware | ❌ Desktop focus | ❌ Desktop focus | ❌ Desktop focus |
| **Easy Provider Setup** | ✅ One-command install | ❌ Complex mining | ❌ Complex setup | ❌ N/A |
| **Flexible Pricing** | ✅ Multiple tiers | ❌ Market rate only | ❌ Fixed pricing | ❌ Fixed pricing |
| **Cost vs. Centralized** | 60-80% cheaper | ~90% cheaper | ~75% cheaper | Baseline |

---

**Built with ❤️ for a decentralized future where data storage is affordable, secure, and user-controlled.**