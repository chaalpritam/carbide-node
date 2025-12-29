# Carbide Node Architecture

## Overview

Carbide Node is a high-performance, Rust-based backend service designed to serve as the central data storage and synchronization hub for the Carbide ecosystem. It connects desktop (`CarbideDrive`) and mobile (`Carbide`) clients, providing efficient large data storage, retrieval, and real-time synchronization capabilities.

## Design Philosophy

Inspired by [rqbit](https://github.com/ikatson/rqbit), Carbide Node adopts a modular, performance-first architecture with:
- **Modular Crate Structure**: Separation of concerns through focused Rust crates
- **Efficient Networking**: Optimized client-server communication protocols
- **Scalable Storage**: Intelligent data management and synchronization
- **Security-First**: Built-in authentication and encryption

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Client Layer                             │
├─────────────────────┬───────────────────────────────────────┤
│   CarbideDrive      │           Carbide Mobile              │
│   (Desktop Client)  │           (Mobile Client)             │
└─────────────────────┴───────────────────────────────────────┘
                              │
                    ┌─────────┴─────────┐
                    │   Network Layer   │
                    │ (HTTP/WebSocket)  │
                    └─────────┬─────────┘
                              │
┌─────────────────────────────┴─────────────────────────────────┐
│                    Carbide Node                               │
├───────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │
│  │   API       │  │  Sync       │  │    Authentication       │ │
│  │  Gateway    │  │  Engine     │  │     & Security          │ │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘ │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │
│  │   Data      │  │  Storage    │  │      Monitoring         │ │
│  │  Manager    │  │  Engine     │  │    & Telemetry          │ │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘ │
└───────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────┴─────────────────────────────────┐
│                 Persistent Storage                            │
├───────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │
│  │  Metadata   │  │    Blob     │  │       Indexes          │ │
│  │   Store     │  │   Storage   │  │    & Cache             │ │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘ │
└───────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. API Gateway (`carbide-api`)

**Purpose**: HTTP/WebSocket API layer for client communication

**Responsibilities**:
- RESTful API endpoints for CRUD operations
- WebSocket connections for real-time synchronization
- Request routing and middleware management
- Rate limiting and request validation

**Key Endpoints**:
```
POST   /api/v1/auth/login
GET    /api/v1/files/{id}
PUT    /api/v1/files/{id}
DELETE /api/v1/files/{id}
POST   /api/v1/files/upload
GET    /api/v1/sync/status
WS     /api/v1/sync/realtime
```

### 2. Authentication & Security (`carbide-auth`)

**Purpose**: Secure client authentication and authorization

**Responsibilities**:
- JWT token generation and validation
- Client device registration and management
- Role-based access control (RBAC)
- Encryption key management

**Features**:
- Multi-device support per user
- Device-specific encryption keys
- Session management and revocation
- End-to-end encryption for sensitive data

### 3. Data Manager (`carbide-data`)

**Purpose**: High-level data operations and business logic

**Responsibilities**:
- File metadata management
- Data deduplication and compression
- Version control and conflict resolution
- Data integrity verification

**Features**:
- Content-addressed storage (CAS) for deduplication
- Incremental updates and delta synchronization
- Automatic data verification using checksums
- Smart caching strategies

### 4. Sync Engine (`carbide-sync`)

**Purpose**: Real-time data synchronization between clients

**Responsibilities**:
- Change detection and propagation
- Conflict resolution strategies
- Network-aware synchronization
- Offline support and queuing

**Sync Strategies**:
- **Last-Write-Wins**: Simple conflict resolution
- **Operational Transform**: For collaborative editing
- **Vector Clocks**: Distributed conflict detection
- **Priority-Based**: User-defined conflict resolution

### 5. Storage Engine (`carbide-storage`)

**Purpose**: Low-level data persistence and retrieval

**Responsibilities**:
- Blob storage with content addressing
- Metadata persistence
- Index management
- Storage optimization

**Storage Architecture**:
```
/data/
├── blobs/           # Content-addressed blob storage
│   ├── 00/
│   ├── 01/
│   └── ...
├── metadata/        # File metadata and relationships
│   ├── users/
│   ├── devices/
│   └── files/
└── indexes/         # Search and query indexes
    ├── content/
    ├── tags/
    └── temporal/
```

### 6. Monitoring & Telemetry (`carbide-metrics`)

**Purpose**: System observability and performance monitoring

**Responsibilities**:
- Prometheus metrics collection
- Distributed tracing
- Health checks and alerting
- Performance profiling

**Metrics**:
- Storage utilization and growth
- API request latencies and throughput
- Sync performance and conflicts
- Client connection statistics

## Data Flow

### File Upload Flow
```
1. Client → API Gateway: POST /api/v1/files/upload
2. API Gateway → Auth: Validate JWT token
3. API Gateway → Data Manager: Process file metadata
4. Data Manager → Storage Engine: Store blob + metadata
5. Data Manager → Sync Engine: Notify other clients
6. Sync Engine → WebSocket: Broadcast changes
7. API Gateway → Client: Return file ID + metadata
```

### Real-time Sync Flow
```
1. Client A: File modification detected
2. Client A → Sync Engine: Send change delta
3. Sync Engine: Process and validate change
4. Sync Engine → Storage Engine: Persist change
5. Sync Engine → All Clients: Broadcast change via WebSocket
6. Clients B,C: Apply change locally
```

## Networking Architecture

### Protocol Stack
- **Transport**: HTTP/2 with TLS 1.3
- **Real-time**: WebSocket with automatic reconnection
- **Compression**: Brotli for HTTP, custom binary protocol for WebSocket
- **Security**: End-to-end encryption with device-specific keys

### Connection Management
- Connection pooling with automatic scaling
- Health checks and circuit breakers
- Graceful degradation for network issues
- Bandwidth-adaptive synchronization

## Storage Strategy

### Content-Addressed Storage
```rust
struct ContentAddress {
    algorithm: HashAlgorithm,  // SHA-256, BLAKE3
    hash: [u8; 32],           // Content hash
}

struct BlobMetadata {
    address: ContentAddress,
    size: u64,
    compression: CompressionType,
    encryption: EncryptionMeta,
    created_at: DateTime<Utc>,
}
```

### Deduplication
- Block-level deduplication using rolling hashes
- Cross-user deduplication (encrypted)
- Intelligent chunking for optimal storage

### Data Lifecycle
1. **Hot**: Recently accessed, in-memory cache
2. **Warm**: Frequently accessed, SSD storage
3. **Cold**: Archived data, compressed and potentially moved to slower storage

## Security Model

### Encryption Layers
1. **Transport**: TLS 1.3 for all network communication
2. **Application**: AES-256-GCM for data at rest
3. **End-to-End**: Client-controlled encryption for sensitive files

### Authentication Flow
```
1. Client registration with public key
2. Server generates device-specific keypair
3. JWT tokens with short expiry (15 min)
4. Refresh tokens for session persistence
5. Device revocation support
```

### Privacy Considerations
- Zero-knowledge architecture for encrypted files
- Metadata minimization
- Optional anonymous usage telemetry

## Performance Characteristics

### Target Metrics
- **API Latency**: p99 < 100ms for metadata operations
- **Upload Throughput**: > 100MB/s per client
- **Sync Latency**: < 500ms for small changes
- **Storage Efficiency**: 80%+ deduplication ratio
- **Availability**: 99.9% uptime

### Scalability
- Horizontal scaling through microservice architecture
- Database sharding by user ID
- CDN integration for blob distribution
- Auto-scaling based on load metrics

## Development Roadmap

### Phase 1: Core Infrastructure
- [ ] Basic API Gateway with authentication
- [ ] Simple file upload/download
- [ ] SQLite-based metadata storage
- [ ] Local blob storage

### Phase 2: Synchronization
- [ ] WebSocket-based real-time sync
- [ ] Conflict detection and resolution
- [ ] Multi-device support
- [ ] Basic encryption

### Phase 3: Optimization
- [ ] Content deduplication
- [ ] Advanced caching strategies
- [ ] Performance monitoring
- [ ] Horizontal scaling support

### Phase 4: Advanced Features
- [ ] Collaborative editing support
- [ ] Advanced security features
- [ ] Mobile-specific optimizations
- [ ] Analytics and insights

## Technology Stack

### Core Dependencies
- **Web Framework**: [axum](https://github.com/tokio-rs/axum) - Modern async web framework
- **Database**: [sqlx](https://github.com/launchbadge/sqlx) - Async SQL toolkit
- **Serialization**: [serde](https://serde.rs/) - Serialization framework
- **Async Runtime**: [tokio](https://tokio.rs/) - Async runtime
- **Cryptography**: [ring](https://github.com/briansmith/ring) - Crypto library
- **Metrics**: [prometheus](https://github.com/prometheus/client_rust) - Metrics collection

### Storage
- **Metadata**: PostgreSQL or SQLite for development
- **Blobs**: Local filesystem or S3-compatible storage
- **Cache**: Redis for session management and hot data
- **Search**: [tantivy](https://github.com/quickwit-oss/tantivy) - Full-text search

### Development Tools
- **Testing**: [cargo nextest](https://nexte.st/) - Faster test runner
- **Benchmarking**: [criterion](https://github.com/bheisler/criterion.rs) - Statistical benchmarking
- **Fuzzing**: [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) - Fuzz testing
- **Profiling**: [flamegraph](https://github.com/flamegraph-rs/flamegraph) - Performance profiling

## Configuration Management

### Environment-based Configuration
```toml
[server]
host = "0.0.0.0"
port = 8080
workers = 4

[database]
url = "postgresql://user:pass@localhost/carbide"
max_connections = 10

[storage]
backend = "local"  # local, s3
path = "/data/blobs"

[auth]
jwt_secret = "${JWT_SECRET}"
token_expiry = "15m"

[sync]
max_clients_per_user = 10
heartbeat_interval = "30s"
```

## Monitoring and Observability

### Logging Strategy
- Structured logging with [tracing](https://github.com/tokio-rs/tracing)
- Log levels: ERROR, WARN, INFO, DEBUG, TRACE
- Request ID correlation across services

### Metrics Collection
- Application metrics (request counts, latencies)
- System metrics (CPU, memory, disk usage)
- Business metrics (user engagement, storage growth)

### Health Checks
```
GET /health          # Basic health check
GET /health/ready    # Readiness probe
GET /health/live     # Liveness probe
```

This architecture provides a solid foundation for building a scalable, secure, and high-performance data storage and synchronization service that can efficiently serve both desktop and mobile clients in the Carbide ecosystem.