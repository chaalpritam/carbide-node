# Carbide Network Replication & Multi-Provider Architecture

**v1.0.0 Implementation Status**: Core multi-provider architecture implemented and tested

> **Note**: This document describes the replication architecture, with basic multi-provider support implemented in v1.0.0. Advanced features like automated failover and dynamic replication are planned for Phase 2.

## Storage Model Evolution

### BitTorrent vs Original Carbide vs Carbide Network (Current)

| Aspect | BitTorrent | Original Carbide | Carbide Network (Current) |
|--------|------------|------------------|---------------------------|
| **Architecture** | Decentralized P2P | Centralized with replication | **Decentralized Marketplace** |
| **Data Distribution** | Pieces across many peers | Complete files on your nodes | **Complete files across chosen providers** |
| **Availability** | Depends on peer presence | Always available (your infrastructure) | **High availability (multiple paid providers)** |
| **Access Control** | Public swarms | Private, authenticated access | **Private, economic incentives** |
| **Data Integrity** | Peer-dependent | Guaranteed by your nodes | **Cryptographically verified by providers** |
| **Performance** | Variable (peer-dependent) | Consistent (your hardware) | **Reputation-based provider selection** |
| **Economics** | Free but unreliable | Your infrastructure cost | **Pay for service, earn from providing** |

### What Carbide Network Learns from BitTorrent and rqbit

The marketplace adopts proven decentralized storage concepts:
- **Content addressing**: Files identified by cryptographic hashes (like IPFS)
- **Chunking**: Large files split into manageable pieces for efficient transfer
- **Deduplication**: Same content stored once, referenced multiple times
- **Proof systems**: Cryptographic verification of storage (like Filecoin)
- **Economic incentives**: Market-driven provider participation

## Decentralized Multi-Provider Architecture

### Current Marketplace Model

```
┌─────────────────────────────────────────────────────────────────┐
│                        Client Layer                            │
├─────────────────────────┬───────────────────────────────────────┤
│     Mobile Client       │         Desktop Client                │
│   (Chooses: 3 copies    │    (Chooses: 5 copies               │
│    $0.005/GB/month)     │     $0.01/GB/month)                  │
└─────────────────────────┴───────────────────────────────────────┘
                                    │
                          ┌─────────┼─────────┐
                          │         │         │
              ┌───────────▼─┐   ┌───▼────┐   ┌▼─────────────┐
              │Discovery &  │   │Gateway │   │Reputation &  │
              │Marketplace  │   │Nodes   │   │Payment System│
              └─────────────┘   └────────┘   └──────────────┘
                          │         │         │
    ┌─────────────────────┴─────────┼─────────┴─────────────────────┐
    │              Provider Network (Anyone Can Join)               │
    ├───────────────────────────────────────────────────────────────┤
    │ 🏠 Home       🏢 Professional   🏭 Enterprise   🌐 Global     │
    │ Providers     Providers         Providers       CDN Providers │
    │ ($0.002/GB)   ($0.004/GB)       ($0.008/GB)    ($0.012/GB)   │
    │ 95% uptime    99% uptime        99.9% uptime    99.99% uptime │
    └───────────────────────────────────────────────────────────────┘
```

### Provider Ecosystem ✅ **WORKING IN v1.0.0**

Anyone can become a storage provider by running Carbide Provider software:

```bash
# Option 1: Use GUI Installer (Mac mini) ✅
# Download and install CarbideProvider-Installer-1.0.0.dmg

# Option 2: Automated installation ✅
./install.sh  # Mac mini with 25GB allocation

# Option 3: Manual setup ✅
cargo build --release
cargo run --bin carbide-provider -- init \
  --storage-path ./storage \
  --capacity 25GB \
  --tier home \
  --region northamerica

cargo run --bin carbide-provider -- start \
  --price-per-gb-month 0.005 \
  --port 8080
```

### Provider Types and Economics

#### 1. 🏠 Home Providers
- **Target**: Individuals with spare storage capacity
- **Pricing**: $0.002/GB/month (competitive with centralized storage)
- **Requirements**: 500GB+ available space, residential internet
- **Uptime**: 95% average (suitable for backup storage)
- **Earnings**: ~$24/year per 1TB (passive income from spare capacity)

#### 2. 🏢 Professional Providers  
- **Target**: Small businesses, tech enthusiasts, prosumers
- **Pricing**: $0.004/GB/month (premium for reliability)
- **Requirements**: Dedicated hardware, UPS backup, business internet
- **Uptime**: 99% guaranteed (suitable for important data)
- **Earnings**: ~$48/year per 1TB

#### 3. 🏭 Enterprise Providers
- **Target**: Data centers, cloud providers, hosting companies  
- **Pricing**: $0.008/GB/month (premium for high availability)
- **Requirements**: Enterprise hardware, redundant power, SLA guarantees
- **Uptime**: 99.9% guaranteed (suitable for critical data)
- **Earnings**: ~$96/year per 1TB

#### 4. 🌐 Global CDN Providers
- **Target**: Major cloud providers with global presence
- **Pricing**: $0.012/GB/month (premium for performance)
- **Requirements**: Global distribution, edge caching, 24/7 support
- **Uptime**: 99.99% guaranteed (mission-critical data)
- **Earnings**: ~$144/year per 1TB

## Replication Strategies

### 1. Synchronous Replication
```rust
// Client uploads to primary, primary replicates before confirming
async fn upload_file_sync(file: File) -> Result<FileId> {
    let file_id = primary_node.store(file).await?;
    
    // Wait for replication to complete
    let replication_results = futures::join!(
        secondary_node.replicate(file_id),
        tertiary_node.replicate(file_id)
    );
    
    // Confirm only after successful replication
    if replication_results.all_success() {
        Ok(file_id)
    } else {
        Err("Replication failed")
    }
}
```

**Pros**: Data safety guaranteed before confirmation
**Cons**: Higher latency, potential for upload failures

### 2. Asynchronous Replication (Recommended)
```rust
// Client gets immediate confirmation, replication happens in background
async fn upload_file_async(file: File) -> Result<FileId> {
    let file_id = primary_node.store(file).await?;
    
    // Immediate response to client
    let response = Ok(file_id);
    
    // Background replication
    tokio::spawn(async move {
        replicate_to_secondaries(file_id).await;
    });
    
    response
}
```

**Pros**: Low latency, better user experience
**Cons**: Temporary window where data exists only on primary

### 3. Hybrid Replication
```rust
// Critical files use sync, regular files use async
async fn upload_file_hybrid(file: File, priority: Priority) -> Result<FileId> {
    match priority {
        Priority::Critical => upload_file_sync(file).await,
        Priority::Normal => upload_file_async(file).await,
    }
}
```

## Node Discovery and Selection

### Configuration-Based Discovery
```toml
# ~/.carbide/config.toml
[nodes]
primary = "https://home.example.com:8443"
secondary = [
    "https://eu-west.carbide.example.com",
    "https://us-east.carbide.example.com"
]
tertiary = [
    "https://backup.example.com:8443"
]

[replication]
strategy = "async"  # sync, async, hybrid
min_replicas = 2
max_replicas = 3
```

### Dynamic Discovery (Advanced)
```rust
// Service discovery for cloud deployments
struct NodeDiscovery {
    consul_client: ConsulClient,
    etcd_client: EtcdClient,
}

impl NodeDiscovery {
    async fn discover_nodes(&self) -> Vec<NodeInfo> {
        // Query service registry for available Carbide nodes
        let nodes = self.consul_client
            .health_service("carbide-node", true)
            .await?;
        
        nodes.into_iter()
            .map(|service| NodeInfo {
                id: service.service_id,
                address: service.address,
                load: service.meta["load"].parse().unwrap_or(0),
                region: service.meta["region"].clone(),
            })
            .collect()
    }
}
```

## Client-Side Multi-Node Logic

### Smart Routing
```rust
struct MultiNodeClient {
    primary: NodeClient,
    secondaries: Vec<NodeClient>,
    health_monitor: HealthMonitor,
}

impl MultiNodeClient {
    async fn upload(&self, file: File) -> Result<FileId> {
        // Always try primary first
        match self.primary.upload(file.clone()).await {
            Ok(file_id) => {
                // Schedule background replication
                self.replicate_async(file_id, file).await;
                Ok(file_id)
            },
            Err(_) => {
                // Fallback to best available secondary
                let secondary = self.select_best_secondary().await?;
                secondary.upload(file).await
            }
        }
    }
    
    async fn download(&self, file_id: FileId) -> Result<File> {
        // Try nodes in order of preference
        for node in self.ordered_nodes().await {
            if let Ok(file) = node.download(file_id).await {
                return Ok(file);
            }
        }
        Err("No nodes available")
    }
    
    async fn select_best_secondary(&self) -> Result<&NodeClient> {
        let mut best_node = None;
        let mut best_latency = Duration::MAX;
        
        for node in &self.secondaries {
            if let Ok(latency) = self.health_monitor.ping(node).await {
                if latency < best_latency {
                    best_latency = latency;
                    best_node = Some(node);
                }
            }
        }
        
        best_node.ok_or("No healthy secondaries")
    }
}
```

## Data Safety Mechanisms

### 1. Consistency Verification
```rust
struct ConsistencyChecker {
    nodes: Vec<NodeClient>,
}

impl ConsistencyChecker {
    async fn verify_file_consistency(&self, file_id: FileId) -> Result<bool> {
        let mut checksums = Vec::new();
        
        for node in &self.nodes {
            if let Ok(checksum) = node.get_file_checksum(file_id).await {
                checksums.push(checksum);
            }
        }
        
        // All checksums should match
        Ok(checksums.iter().all(|c| c == &checksums[0]))
    }
    
    async fn repair_inconsistency(&self, file_id: FileId) -> Result<()> {
        // Find the majority consensus checksum
        let correct_checksum = self.find_consensus_checksum(file_id).await?;
        
        // Repair nodes with incorrect checksums
        for node in &self.nodes {
            if node.get_file_checksum(file_id).await? != correct_checksum {
                self.repair_node_file(node, file_id, correct_checksum).await?;
            }
        }
        
        Ok(())
    }
}
```

### 2. Automatic Failover
```rust
struct FailoverManager {
    nodes: Vec<NodeClient>,
    health_states: HashMap<NodeId, HealthState>,
}

#[derive(Clone, Copy)]
enum HealthState {
    Healthy,
    Degraded,
    Failed,
    Recovering,
}

impl FailoverManager {
    async fn handle_node_failure(&mut self, failed_node: NodeId) {
        self.health_states.insert(failed_node, HealthState::Failed);
        
        // Redistribute load to healthy nodes
        let healthy_nodes: Vec<_> = self.nodes.iter()
            .filter(|n| self.is_healthy(n.id()))
            .collect();
        
        if healthy_nodes.len() < 2 {
            log::warn!("Only {} healthy nodes remaining", healthy_nodes.len());
            // Trigger alerts
        }
        
        // Start recovery process
        tokio::spawn(async move {
            self.attempt_node_recovery(failed_node).await;
        });
    }
}
```

### 3. Backup Verification
```rust
struct BackupManager {
    nodes: Vec<NodeClient>,
}

impl BackupManager {
    async fn verify_all_backups(&self) -> BackupReport {
        let mut report = BackupReport::new();
        
        for node in &self.nodes {
            let node_report = self.verify_node_backup(node).await;
            report.add_node_report(node.id(), node_report);
        }
        
        report
    }
    
    async fn verify_node_backup(&self, node: &NodeClient) -> NodeBackupReport {
        let file_list = node.list_all_files().await?;
        let mut missing_files = Vec::new();
        let mut corrupted_files = Vec::new();
        
        for file_id in file_list {
            match node.verify_file_integrity(file_id).await {
                Ok(true) => {}, // File is intact
                Ok(false) => corrupted_files.push(file_id),
                Err(_) => missing_files.push(file_id),
            }
        }
        
        NodeBackupReport {
            total_files: file_list.len(),
            missing_files,
            corrupted_files,
            last_verified: Utc::now(),
        }
    }
}
```

## Mobile Client Specific Optimizations

### Bandwidth-Aware Replication
```rust
struct MobileReplicationManager {
    network_monitor: NetworkMonitor,
    battery_monitor: BatteryMonitor,
    user_preferences: UserPreferences,
}

impl MobileReplicationManager {
    async fn should_replicate(&self, file_size: u64) -> ReplicationDecision {
        let network_type = self.network_monitor.current_network_type();
        let battery_level = self.battery_monitor.battery_percentage();
        
        match network_type {
            NetworkType::WiFi => {
                // Always replicate on WiFi
                ReplicationDecision::ReplicateAll
            },
            NetworkType::Cellular => {
                if battery_level > 20 && file_size < self.user_preferences.cellular_limit {
                    ReplicationDecision::ReplicateImportant
                } else {
                    ReplicationDecision::ReplicateEssential
                }
            },
            NetworkType::Offline => ReplicationDecision::QueueForLater,
        }
    }
}

enum ReplicationDecision {
    ReplicateAll,      // Replicate to all configured nodes
    ReplicateImportant, // Replicate only to primary + one secondary
    ReplicateEssential, // Replicate only to primary
    QueueForLater,     // Queue for when network improves
}
```

### Sync Priority Queue
```rust
struct SyncQueue {
    high_priority: VecDeque<SyncTask>,    // Critical user data
    medium_priority: VecDeque<SyncTask>,  // Important documents
    low_priority: VecDeque<SyncTask>,     // Media files, cache
}

impl SyncQueue {
    async fn process_queue(&mut self, available_bandwidth: u64) {
        while available_bandwidth > 0 {
            let task = self.next_task().await;
            
            if task.estimated_size <= available_bandwidth {
                self.execute_sync_task(task).await;
                available_bandwidth -= task.estimated_size;
            } else {
                break; // Not enough bandwidth for next task
            }
        }
    }
}
```

## Monitoring and Health Checks

### Replication Health Dashboard
```rust
struct ReplicationMonitor {
    metrics: PrometheusMetrics,
}

impl ReplicationMonitor {
    async fn collect_metrics(&self) -> ReplicationMetrics {
        ReplicationMetrics {
            replication_lag: self.measure_replication_lag().await,
            node_health: self.check_all_node_health().await,
            consistency_score: self.measure_consistency().await,
            bandwidth_utilization: self.measure_bandwidth_usage().await,
            failed_replications: self.count_failed_replications().await,
        }
    }
    
    async fn generate_health_report(&self) -> HealthReport {
        let metrics = self.collect_metrics().await;
        
        HealthReport {
            overall_status: self.calculate_overall_status(&metrics),
            recommendations: self.generate_recommendations(&metrics),
            alerts: self.check_for_alerts(&metrics),
            timestamp: Utc::now(),
        }
    }
}
```

---

## Migration to Decentralized Architecture

The above sections describe the **legacy centralized replication** approach. The current **Carbide Network** implements a decentralized marketplace model where:

### Key Differences:
- **Provider Choice**: Users select from marketplace providers vs. managing own nodes
- **Economic Model**: Pay providers vs. manage infrastructure costs  
- **Replication Control**: User-configurable (1-10 copies) vs. fixed architecture
- **Mobile Optimization**: Smart provider selection based on network conditions

### For Complete Current Architecture:
See **[DECENTRALIZED_ARCHITECTURE.md](./DECENTRALIZED_ARCHITECTURE.md)** for the full marketplace design including:
- Provider discovery and selection algorithms
- Reputation-based trust system  
- Economic incentives and payment models
- User-configurable storage tiers
- Mobile-optimized protocols

The decentralized marketplace provides:
- **60-80% cost savings** vs. centralized storage
- **User choice** in replication factor and provider types
- **Economic incentives** for a sustainable provider ecosystem
- **Global availability** through diverse provider network