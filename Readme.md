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
# Install and start earning
curl -sSL https://get.carbide.network/provider | bash
carbide-provider init --storage-path /mnt/spare --capacity 1TB
carbide-provider start --price-per-gb-month 0.002
```

**Provider Types:**
- **🏠 Home Users**: Spare disk space, earn ~$0.002/GB/month
- **🏢 Professionals**: Business storage, higher uptime guarantees  
- **🏭 Enterprise**: Data center grade, premium pricing

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
├── CLAUDE.md                     # Development guidance
└── crates/                       # Rust implementation (coming soon)
    ├── carbide-core/             # Core data structures
    ├── carbide-provider/         # Storage provider node
    ├── carbide-discovery/        # Network discovery service
    ├── carbide-client/           # Client SDK
    └── carbide-reputation/       # Reputation system
```

## 🚦 Getting Started

### As a Storage Provider
1. **Download** the provider software
2. **Allocate** storage space and set pricing
3. **Start earning** from your spare capacity

### As a Data User  
1. **Install** Carbide mobile/desktop client
2. **Configure** storage preferences (replication, budget, provider types)
3. **Upload** with automatic provider selection and payment

### As a Developer
1. **Read** `DECENTRALIZED_ARCHITECTURE.md` for complete system design
2. **Review** `CLAUDE.md` for development setup and patterns
3. **Explore** the modular crate structure for implementation details

## 🗺️ Roadmap

### **Phase 1: Basic Marketplace** (Months 1-3)
- ✅ Architecture design complete
- ⏳ Core provider node implementation  
- ⏳ Simple discovery mechanism
- ⏳ Basic replication with user choice

### **Phase 2: Advanced Features** (Months 4-6)
- ⏳ Reputation system implementation
- ⏳ Mobile-optimized protocols
- ⏳ Dynamic pricing algorithms
- ⏳ Provider performance monitoring

### **Phase 3: Economic Infrastructure** (Months 7-9)
- ⏳ Token/payment system integration
- ⏳ Smart contract deployment
- ⏳ Automated dispute resolution
- ⏳ Advanced proof systems

### **Phase 4: Scale & Optimize** (Months 10-12)
- ⏳ Decentralized governance
- ⏳ Cross-chain integration  
- ⏳ Enterprise features
- ⏳ Global network launch

## 🏢 Related Projects

- **Mobile Client**: `/Users/chaalpritam/Blockbase/Carbide` - iOS/Android app for data storage
- **Desktop Client**: `/Users/chaalpritam/Blockbase/CarbideDrive` - Desktop sync application

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
- **[CLAUDE.md](./CLAUDE.md)** - Developer setup and guidance

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