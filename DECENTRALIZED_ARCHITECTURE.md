# Carbide Network: Decentralized Storage Marketplace Architecture

**v1.0.0 Implementation Status**: Core architecture implemented and working

## Vision Statement

Carbide Network is a **working decentralized storage marketplace** where anyone can contribute storage capacity and earn rewards, while users get affordable, secure, and customizable data storage with user-defined replication factors and pricing tiers.

**Current Status**: The architecture described in this document has been successfully implemented in v1.0.0, with a fully functional Mac provider node, desktop GUI application, and complete backend infrastructure.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        Client Layer                            │
├─────────────────────────┬───────────────────────────────────────┤
│     Mobile Client       │         Desktop Client                │
│   (Data Consumer)       │       (Data Consumer)                 │
└─────────────────────────┴───────────────────────────────────────┘
                                    │
                      ┌─────────────┼─────────────┐
                      │             │             │
              ┌───────▼──┐   ┌─────▼──┐   ┌──────▼───┐
              │Discovery │   │Gateway │   │Reputation│
              │  Nodes   │   │ Nodes  │   │  System  │
              └──────────┘   └────────┘   └──────────┘
                      │             │             │
    ┌─────────────────┴─────────────┼─────────────┴─────────────────┐
    │                   Carbide Network Layer                       │
    ├───────────────────────────────────────────────────────────────┤
    │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────┐  │
    │  │Provider  │ │Provider  │ │Provider  │ │Provider  │ │... │  │
    │  │  Node    │ │  Node    │ │  Node    │ │  Node    │ │    │  │
    │  │ (Home)   │ │ (Office) │ │  (VPS)   │ │(Datactr) │ │    │  │
    │  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └────┘  │
    └───────────────────────────────────────────────────────────────┘
                                    │
    ┌─────────────────────────────────────────────────────────────────┐
    │             Blockchain Layer (Optional)                        │
    ├─────────────────────────────────────────────────────────────────┤
    │    Smart Contracts • Payments • Reputation • Governance        │
    └─────────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. Storage Provider Nodes (`carbide-provider`) ✅ **IMPLEMENTED**

**Anyone can run a Carbide Provider Node** to earn money by contributing storage space.

**Implementation Status**: Fully working provider node with HTTP API, storage management, configuration system, and beautiful desktop GUI application.

#### Provider Types:
```rust
enum ProviderType {
    Home {          // Home users with spare disk space
        capacity: u64,
        uptime_guarantee: f32,    // 90-95%
        bandwidth: Bandwidth,
    },
    Professional {  // Small businesses, enthusiasts
        capacity: u64,
        uptime_guarantee: f32,    // 95-99%
        bandwidth: Bandwidth,
        backup_power: bool,
    },
    Enterprise {    // Data centers, cloud providers
        capacity: u64,
        uptime_guarantee: f32,    // 99%+
        bandwidth: Bandwidth,
        sla_guarantees: SlaLevel,
        geographic_location: Region,
    },
}
```

#### Provider Setup Process (v1.0.0 - Working):
```bash
# Option 1: Use GUI Installer (Recommended) ✅
# Download CarbideProvider-Installer-1.0.0.dmg
# Install and run the setup wizard

# Option 2: Command-line Installation ✅
./install.sh  # Automated Mac mini setup

# Option 3: Manual Setup ✅
cargo build --release
cargo run --bin carbide-provider -- init \
  --storage-path ./storage \
  --capacity 25GB \
  --tier home \
  --region northamerica

cargo run --bin carbide-provider -- start \
  --name "My Provider" \
  --price-per-gb-month 0.005 \
  --port 8080
```

### 2. Client Storage Configuration ✅ **BASIC IMPLEMENTATION**

**Users configure their storage preferences** during signup or per-file.

**Implementation Status**: Basic client SDK and CLI tools implemented. Mobile-specific optimizations planned for Phase 2.

#### Storage Tiers:
```rust
#[derive(Clone, Debug)]
struct StorageTier {
    name: String,
    replication_factor: u8,      // 1-10 copies
    provider_requirements: ProviderRequirements,
    max_price_per_gb_month: f64, // USD
    performance_tier: PerformanceTier,
}

enum PerformanceTier {
    Economy,    // Slower access, cheapest
    Standard,   // Balanced
    Premium,    // Fast access, most expensive
}

struct ProviderRequirements {
    min_uptime: f32,           // 90%, 95%, 99%
    preferred_regions: Vec<Region>,
    exclude_home_providers: bool,
    require_backup_power: bool,
    max_latency_ms: u32,
}
```

#### User Configuration Example:
```toml
# User's ~/.carbide/storage-config.toml

[[storage_tiers]]
name = "Critical"
replication_factor = 5
max_price_per_gb_month = 0.01
min_uptime = 0.99
require_backup_power = true
exclude_home_providers = true

[[storage_tiers]]
name = "Important" 
replication_factor = 3
max_price_per_gb_month = 0.005
min_uptime = 0.95
require_backup_power = false

[[storage_tiers]]
name = "Backup"
replication_factor = 2
max_price_per_gb_month = 0.002
min_uptime = 0.90
exclude_home_providers = false
```

### 3. Network Discovery & Marketplace ✅ **IMPLEMENTED**

**Implementation Status**: Full discovery service with provider registry, health checking, and marketplace coordination.

#### Discovery Nodes (`carbide-discovery`)
```rust
struct DiscoveryNode {
    known_providers: HashMap<ProviderId, ProviderInfo>,
    reputation_cache: ReputationCache,
    pricing_index: PricingIndex,
    geographic_index: GeographicIndex,
}

impl DiscoveryNode {
    async fn find_providers(&self, requirements: StorageRequirements) -> Vec<ProviderMatch> {
        let candidates = self.filter_by_requirements(requirements).await;
        let scored = self.score_providers(candidates, requirements).await;
        self.rank_by_price_performance(scored)
    }
    
    async fn get_market_rates(&self, region: Region, tier: PerformanceTier) -> MarketRates {
        MarketRates {
            median_price: self.pricing_index.median_price(region, tier),
            provider_count: self.pricing_index.provider_count(region, tier),
            availability_score: self.calculate_availability_score(region, tier),
        }
    }
}
```

#### Provider Discovery Protocol:
```rust
// Providers announce themselves to discovery nodes
struct ProviderAnnouncement {
    provider_id: ProviderId,
    capacity_available: u64,
    pricing: PricingModel,
    location: GeographicLocation,
    uptime_history: UptimeStats,
    proof_of_capacity: CapacityProof,
    reputation_score: f64,
}

struct PricingModel {
    storage_price_per_gb_month: f64,
    bandwidth_price_per_gb: f64,
    retrieval_price_per_request: f64,
    minimum_contract_duration: Duration,
}
```

### 4. Smart Contract Layer ⏳ **PLANNED FOR PHASE 3**

**Implementation Status**: Not yet implemented. Planned for Phase 3 (Economic Infrastructure).

#### Storage Contracts (Future):
```solidity
contract CarbideStorageContract {
    struct StorageDeal {
        bytes32 fileHash;
        address client;
        address[] providers;
        uint256 replicationFactor;
        uint256 duration;
        uint256 pricePerProvider;
        uint256 collateralPerProvider;
        DealStatus status;
    }
    
    mapping(bytes32 => StorageDeal) public deals;
    
    function createStorageDeal(
        bytes32 fileHash,
        address[] memory providers,
        uint256 duration,
        uint256 replicationFactor
    ) external payable {
        require(providers.length >= replicationFactor, "Insufficient providers");
        // ... implementation
    }
    
    function submitProofOfStorage(
        bytes32 fileHash,
        bytes memory proof
    ) external {
        // Verify proof and release payments
    }
}
```

### 5. Reputation System ✅ **IMPLEMENTED**

**Implementation Status**: Basic reputation tracking implemented with uptime monitoring and scoring. Advanced proof-of-storage verification planned for Phase 2.

#### Multi-Dimensional Reputation:
```rust
struct ReputationScore {
    overall: f64,           // 0.0 - 1.0
    components: ReputationComponents,
    history: Vec<ReputationEvent>,
}

struct ReputationComponents {
    uptime: f64,           // Historical uptime percentage
    data_integrity: f64,   // Successful proof-of-storage submissions
    response_time: f64,    // Average response time for requests
    contract_compliance: f64, // Contract fulfillment rate
    community_feedback: f64,  // Peer and client feedback
}

impl ReputationSystem {
    async fn calculate_reputation(&self, provider_id: ProviderId) -> ReputationScore {
        let events = self.get_reputation_events(provider_id).await;
        let uptime = self.calculate_uptime_score(&events);
        let integrity = self.calculate_integrity_score(&events);
        let response = self.calculate_response_score(&events);
        let compliance = self.calculate_compliance_score(&events);
        let feedback = self.calculate_feedback_score(&events);
        
        let overall = (uptime * 0.25) + (integrity * 0.25) + 
                     (response * 0.2) + (compliance * 0.2) + 
                     (feedback * 0.1);
        
        ReputationScore {
            overall,
            components: ReputationComponents {
                uptime, integrity, response, compliance, feedback
            },
            history: events,
        }
    }
}
```

## Storage Request Flow

### 1. Client Upload with Custom Replication:

```rust
async fn upload_with_preferences(
    client: &CarbideClient,
    file: File,
    storage_tier: StorageTier,
) -> Result<FileStorageResult> {
    
    // 1. Client specifies storage preferences
    let requirements = StorageRequirements {
        replication_factor: storage_tier.replication_factor,
        max_price: storage_tier.max_price_per_gb_month,
        performance_tier: storage_tier.performance_tier,
        provider_requirements: storage_tier.provider_requirements,
    };
    
    // 2. Discovery service finds suitable providers
    let providers = client.discovery.find_providers(requirements).await?;
    
    // 3. Client creates storage contracts
    let contracts = create_storage_contracts(&file, &providers).await?;
    
    // 4. Encrypt and chunk the file
    let encrypted_file = client.encrypt_file(file).await?;
    let chunks = chunk_file(encrypted_file, providers.len()).await?;
    
    // 5. Distribute chunks to selected providers
    let upload_results = futures::join_all(
        providers.iter().zip(chunks.iter()).map(|(provider, chunk)| {
            provider.store_chunk(chunk.clone())
        })
    ).await;
    
    // 6. Verify all uploads succeeded
    let successful_uploads = upload_results.iter()
        .filter(|result| result.is_ok())
        .count();
    
    if successful_uploads >= requirements.replication_factor as usize {
        Ok(FileStorageResult {
            file_id: file.hash(),
            providers: providers.clone(),
            replication_achieved: successful_uploads,
            total_cost: calculate_total_cost(&providers, file.size()),
        })
    } else {
        // Handle partial failure - find replacement providers
        handle_upload_failure(file, requirements, upload_results).await
    }
}
```

### 2. Provider Selection Algorithm:

```rust
struct ProviderSelector {
    reputation_weight: f64,     // 0.4
    price_weight: f64,          // 0.3
    performance_weight: f64,    // 0.2
    geographic_weight: f64,     // 0.1
}

impl ProviderSelector {
    async fn select_optimal_providers(
        &self,
        candidates: Vec<Provider>,
        requirements: StorageRequirements,
    ) -> Vec<Provider> {
        
        let mut scored_providers: Vec<_> = candidates.into_iter()
            .filter(|p| self.meets_requirements(p, &requirements))
            .map(|p| {
                let reputation_score = p.reputation.overall * self.reputation_weight;
                let price_score = self.calculate_price_score(p.pricing, requirements.max_price) * self.price_weight;
                let performance_score = self.calculate_performance_score(&p) * self.performance_weight;
                let geographic_score = self.calculate_geographic_score(&p, &requirements) * self.geographic_weight;
                
                let total_score = reputation_score + price_score + performance_score + geographic_score;
                
                (p, total_score)
            })
            .collect();
        
        // Sort by score (descending) and take top N
        scored_providers.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scored_providers.into_iter()
            .take(requirements.replication_factor as usize)
            .map(|(provider, _)| provider)
            .collect()
    }
}
```

## Proof of Storage System

### Proof-of-Replication (PoRep):
```rust
struct ProofOfReplication {
    file_hash: Hash,
    sector_id: SectorId,
    proof: Vec<u8>,
    timestamp: DateTime<Utc>,
}

impl StorageProvider {
    async fn generate_proof_of_replication(&self, file_hash: Hash) -> Result<ProofOfReplication> {
        // 1. Retrieve stored file
        let stored_data = self.storage.get_file(file_hash).await?;
        
        // 2. Generate cryptographic proof
        let proof = self.crypto.generate_replication_proof(
            &stored_data,
            &self.provider_id,
            Utc::now().timestamp()
        )?;
        
        Ok(ProofOfReplication {
            file_hash,
            sector_id: self.calculate_sector_id(file_hash),
            proof,
            timestamp: Utc::now(),
        })
    }
}
```

### Proof-of-Spacetime (PoSt):
```rust
struct ProofOfSpacetime {
    provider_id: ProviderId,
    challenge: Challenge,
    proof: Vec<u8>,
    files_proven: Vec<Hash>,
}

impl NetworkChallenger {
    async fn issue_spacetime_challenge(&self, provider: ProviderId) -> Challenge {
        let stored_files = self.get_provider_files(provider).await;
        let random_files = self.select_random_files(stored_files, 10);
        
        Challenge {
            id: Uuid::new_v4(),
            provider_id: provider,
            files_to_prove: random_files,
            challenge_time: Utc::now(),
            deadline: Utc::now() + Duration::hours(1),
        }
    }
}
```

## Economic Model

### Payment Structure:
```rust
struct PaymentModel {
    storage_fee: StorageFee,
    retrieval_fee: RetrievalFee,
    network_fee: NetworkFee,
}

struct StorageFee {
    base_rate_per_gb_month: f64,    // e.g., $0.002
    replication_multiplier: f64,    // 1.0x for single copy, 0.8x per additional
    duration_discount: f64,         // Discount for longer commitments
}

struct RetrievalFee {
    per_gb_retrieved: f64,          // e.g., $0.001
    per_request: f64,              // e.g., $0.0001
}

struct NetworkFee {
    discovery_fee: f64,            // Small fee for discovery service
    reputation_fee: f64,           // Fee for reputation system maintenance
}

impl PaymentCalculator {
    fn calculate_monthly_cost(
        &self,
        file_size_gb: f64,
        replication_factor: u8,
        provider_rates: &[f64],
    ) -> f64 {
        let base_cost: f64 = provider_rates.iter()
            .take(replication_factor as usize)
            .sum();
        
        let network_overhead = base_cost * 0.05; // 5% network fee
        
        base_cost + network_overhead
    }
}
```

### Provider Economics:
```rust
struct ProviderEconomics {
    hardware_costs: HardwareCosts,
    operational_costs: OperationalCosts,
    target_profit_margin: f64,
}

struct HardwareCosts {
    storage_cost_per_gb: f64,      // Amortized disk cost
    bandwidth_cost_per_gb: f64,    // Internet costs
    power_cost_per_watt_hour: f64, // Electricity
}

impl ProviderEconomics {
    fn calculate_minimum_viable_price(&self) -> f64 {
        let total_monthly_costs = 
            self.hardware_costs.storage_cost_per_gb +
            self.operational_costs.maintenance_per_gb +
            self.operational_costs.insurance_per_gb;
        
        total_monthly_costs * (1.0 + self.target_profit_margin)
    }
}
```

## Mobile Client Optimizations

### Adaptive Replication Strategy:
```rust
struct MobileStorageManager {
    network_monitor: NetworkMonitor,
    battery_monitor: BatteryMonitor,
    cost_optimizer: CostOptimizer,
}

impl MobileStorageManager {
    async fn upload_with_smart_replication(
        &self,
        file: File,
        importance: FileImportance,
    ) -> Result<StorageResult> {
        
        let network_type = self.network_monitor.current_network_type();
        let battery_level = self.battery_monitor.battery_percentage();
        let user_budget = self.cost_optimizer.get_monthly_budget();
        
        let replication_strategy = match (network_type, importance, battery_level) {
            (NetworkType::WiFi, FileImportance::Critical, _) => {
                ReplicationStrategy::Maximum { factor: 5 }
            },
            (NetworkType::WiFi, FileImportance::Important, _) => {
                ReplicationStrategy::Standard { factor: 3 }
            },
            (NetworkType::Cellular, FileImportance::Critical, battery) if battery > 20 => {
                ReplicationStrategy::Essential { factor: 2 }
            },
            (NetworkType::Cellular, _, _) => {
                ReplicationStrategy::Deferred
            },
            (NetworkType::Offline, _, _) => {
                ReplicationStrategy::Queue
            },
        };
        
        self.execute_replication_strategy(file, replication_strategy).await
    }
}
```

## Network Governance

### Decentralized Governance Model:
```rust
struct GovernanceSystem {
    voting_power: HashMap<ParticipantId, VotingPower>,
    active_proposals: Vec<Proposal>,
    voting_mechanism: VotingMechanism,
}

enum VotingMechanism {
    TokenBased,     // Voting power based on network token holdings
    StakeBased,     // Voting power based on storage provided/used
    Hybrid,         // Combination of tokens and stake
}

struct Proposal {
    id: ProposalId,
    title: String,
    description: String,
    proposal_type: ProposalType,
    votes_for: u64,
    votes_against: u64,
    deadline: DateTime<Utc>,
}

enum ProposalType {
    NetworkParameter {     // Change network parameters
        parameter: String,
        current_value: String,
        proposed_value: String,
    },
    ProtocolUpgrade {      // Upgrade network protocol
        version: String,
        features: Vec<String>,
    },
    EconomicModel {        // Change fee structure
        current_model: PaymentModel,
        proposed_model: PaymentModel,
    },
}
```

## Security Considerations

### 1. Data Privacy:
- **Client-side encryption**: Files encrypted before leaving user's device
- **Zero-knowledge providers**: Providers cannot read stored data
- **Key management**: Users control encryption keys

### 2. Provider Security:
- **Collateral requirements**: Providers must stake tokens/funds
- **Reputation slashing**: Malicious behavior results in reputation loss
- **Geographic distribution**: Prevents single-jurisdiction attacks

### 3. Network Security:
- **Proof verification**: Regular challenges ensure data integrity
- **Byzantine fault tolerance**: Network operates with up to 33% malicious nodes
- **Economic incentives**: Aligned incentives prevent attacks

## Implementation Roadmap

### Phase 1: Basic Marketplace (Months 1-3)
- [ ] Core provider node implementation
- [ ] Simple discovery mechanism
- [ ] Basic replication (fixed factor)
- [ ] Local payment tracking

### Phase 2: Advanced Features (Months 4-6)
- [ ] Dynamic replication factor selection
- [ ] Reputation system implementation
- [ ] Mobile-optimized protocols
- [ ] Price optimization algorithms

### Phase 3: Economic Infrastructure (Months 7-9)
- [ ] Token/payment system integration
- [ ] Smart contract deployment
- [ ] Automated provider selection
- [ ] Advanced proof systems

### Phase 4: Scale & Optimize (Months 10-12)
- [ ] Network governance system
- [ ] Advanced economic models
- [ ] Cross-chain integration
- [ ] Enterprise features

## Best Approach Summary

### **Recommended Architecture: Hybrid Approach**

1. **Start Simple**: Begin with centralized discovery and reputation, move to decentralized over time
2. **Economic Incentives First**: Focus on making it profitable for providers and affordable for users
3. **Mobile-Centric Design**: Optimize for mobile constraints from day one
4. **Gradual Decentralization**: Add blockchain components incrementally as network grows
5. **Interoperability**: Learn from and potentially integrate with existing protocols (IPFS/Filecoin)

### **Key Differentiators from Existing Solutions:**
- **User-Configurable Replication**: Unlike fixed replication in Storj/Filecoin
- **Mobile-First Design**: Optimized for mobile constraints and usage patterns
- **Flexible Economic Model**: Multiple pricing tiers and provider types
- **Easy Provider Onboarding**: Simple setup for home users to contribute storage

This architecture creates a **sustainable ecosystem** where:
- **Users** get affordable, customizable storage
- **Providers** earn meaningful revenue from spare capacity
- **The network** scales organically through economic incentives