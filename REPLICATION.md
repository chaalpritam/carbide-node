# Carbide Node Replication & Multi-Node Architecture

## Storage Model Comparison

### BitTorrent vs Carbide Node

| Aspect | BitTorrent | Carbide Node |
|--------|------------|--------------|
| **Architecture** | Decentralized P2P | Centralized with replication |
| **Data Distribution** | Pieces across many peers | Complete files on your nodes |
| **Availability** | Depends on peer presence | Always available (your infrastructure) |
| **Access Control** | Public swarms | Private, authenticated access |
| **Data Integrity** | Peer-dependent | Guaranteed by your nodes |
| **Performance** | Variable (peer-dependent) | Consistent (your hardware) |

### What Carbide Node Learns from BitTorrent

While not P2P, Carbide Node adopts these BitTorrent concepts:
- **Content addressing**: Files identified by cryptographic hashes
- **Chunking**: Large files split into manageable pieces for transfer
- **Deduplication**: Same content stored once, referenced multiple times
- **Efficient synchronization**: Only changed chunks transferred

## Multi-Node Replication Architecture

### Node Hierarchy

```
┌─────────────────────────────────────────────────────────────┐
│                     Client Layer                           │
├─────────────────────┬───────────────────────────────────────┤
│   Mobile Client     │         Desktop Client                │
│   (Primary Source)  │         (Sync Target)                 │
└─────────────────────┴───────────────────────────────────────┘
                              │
            ┌─────────────────┼─────────────────┐
            │                 │                 │
    ┌───────▼──────┐  ┌──────▼──────┐  ┌──────▼──────┐
    │ Primary Node │  │Secondary Node│  │Tertiary Node│
    │ (Home/Local) │  │  (Cloud/VPS) │  │  (Backup)   │
    └──────────────┘  └─────────────┘  └─────────────┘
            │                 │                 │
    ┌───────▼──────┐  ┌──────▼──────┐  ┌──────▼──────┐
    │   Storage    │  │   Storage   │  │   Storage   │
    │   Complete   │  │  Complete   │  │  Complete   │
    │   Copy A     │  │   Copy B    │  │   Copy C    │
    └──────────────┘  └─────────────┘  └─────────────┘
```

### Node Types and Roles

#### 1. Primary Node
- **Role**: Main data repository and coordination point
- **Location**: Typically local (home server, NAS)
- **Responsibilities**:
  - Accept all client uploads
  - Coordinate replication to secondary nodes
  - Handle conflict resolution
  - Serve as authoritative source

#### 2. Secondary Nodes
- **Role**: Hot standby and load distribution
- **Location**: Cloud providers (AWS, DigitalOcean, etc.)
- **Responsibilities**:
  - Maintain synchronized copy of all data
  - Serve read requests when primary unavailable
  - Provide geographic distribution

#### 3. Tertiary Nodes
- **Role**: Cold backup and disaster recovery
- **Location**: Different geographic region/provider
- **Responsibilities**:
  - Maintain complete backup
  - Activate only during primary/secondary failures
  - Long-term data preservation

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

This multi-node replication system gives you:
- **Data Safety**: Multiple copies across different locations
- **High Availability**: Automatic failover if nodes go down
- **Performance**: Load distribution and geographic optimization
- **Mobile-Friendly**: Smart bandwidth and battery management
- **Consistency**: Automatic verification and repair of data integrity

Your mobile client can sync to multiple nodes simultaneously, ensuring your data is always safe and accessible from anywhere!