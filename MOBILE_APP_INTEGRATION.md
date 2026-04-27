# Carbide Mobile App → Decentralized Network Integration Plan

## 🎯 **Goal**
Connect the Carbide iOS/Android mobile app to the decentralized Carbide provider network, allowing users to upload/download files to/from providers running anywhere in the world.

---

## 📋 **Implementation Phases (Priority Order)**

### **PHASE 0: Infrastructure Prerequisites** 🏗️
*Must be done first - these are blocking dependencies*

#### 0.1 **Discovery Service (CRITICAL)** ⚠️
**Location**: New service - `carbide-discovery-service/`

**Purpose**: Central service that helps clients find available providers

**What to build**:
```
carbide-discovery-service/
├── src/
│   ├── main.rs              # Discovery HTTP server
│   ├── provider_registry.rs # Track active providers
│   ├── marketplace.rs       # Provider search/filtering
│   └── health_monitor.rs    # Provider health checks
└── Cargo.toml
```

**API Endpoints Needed**:
```rust
// Provider Registration (called by providers)
POST   /api/v1/providers/register
POST   /api/v1/providers/heartbeat
DELETE /api/v1/providers/unregister

// Client Discovery (called by mobile app)
GET    /api/v1/providers/search?region=asia&max_price=0.01
GET    /api/v1/providers/health/{provider_id}
GET    /api/v1/providers/quote
```

**Database**:
```sql
CREATE TABLE providers (
    id UUID PRIMARY KEY,
    name VARCHAR(255),
    endpoint VARCHAR(255),
    region VARCHAR(50),
    tier VARCHAR(50),
    price_per_gb_month DECIMAL(10,4),
    available_capacity BIGINT,
    reputation_score FLOAT,
    last_heartbeat TIMESTAMP,
    status VARCHAR(20)
);

CREATE TABLE provider_health (
    provider_id UUID,
    timestamp TIMESTAMP,
    uptime_percentage FLOAT,
    response_time_ms INTEGER,
    failures_count INTEGER
);
```

**Priority**: **🔴 CRITICAL - BLOCKS EVERYTHING**
**Estimated Work**: 3-5 days
**Dependencies**: None

---

#### 0.2 **Provider Auto-Registration** 🤖
**Location**: Update `carbide-provider/src/main.rs`

**What to add**:
```rust
// In provider startup
async fn register_with_discovery(config: &ProviderConfig) -> Result<()> {
    let client = reqwest::Client::new();

    // Register on startup
    let registration = ProviderRegistration {
        id: provider.id,
        name: config.provider.name.clone(),
        endpoint: format!("http://{}:{}", get_public_ip(), config.provider.port),
        region: config.provider.region.clone(),
        tier: config.provider.tier.clone(),
        price_per_gb_month: config.pricing.price_per_gb_month,
        available_capacity: config.provider.max_storage_gb * GB,
    };

    client.post(&format!("{}/api/v1/providers/register", discovery_endpoint))
        .json(&registration)
        .send()
        .await?;

    // Start heartbeat task
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            send_heartbeat(&provider_id, &discovery_endpoint).await;
        }
    });

    Ok(())
}
```

**Priority**: **🔴 CRITICAL - BLOCKS CLIENT DISCOVERY**
**Estimated Work**: 1 day
**Dependencies**: Discovery Service must exist

---

#### 0.3 **Gateway Service (Optional but Recommended)** 🌐
**Location**: New service - `carbide-gateway/`

**Purpose**:
- Public endpoint for mobile apps (no firewall issues)
- Load balancing
- DDoS protection
- SSL/TLS termination

**Why needed**:
- Most home providers are behind NAT/firewalls
- Mobile apps need reliable, publicly accessible endpoints
- Can proxy requests to providers

**API**:
```rust
// Gateway proxies requests to actual providers
POST /api/v1/proxy/upload/{provider_id}
GET  /api/v1/proxy/download/{provider_id}/{file_id}
GET  /api/v1/proxy/status/{provider_id}
```

**Priority**: **🟡 MEDIUM - Can start without it**
**Estimated Work**: 2-3 days
**Dependencies**: Discovery Service

---

### **PHASE 1: Backend Foundation** 🔧
*Core infrastructure for mobile app*

#### 1.1 **Swift Client SDK** 📱
**Location**: New package - `carbide-ios-sdk/`

**What to build**:
```swift
// Swift Package Manager structure
carbide-ios-sdk/
├── Sources/
│   └── CarbideSDK/
│       ├── CarbideClient.swift        # Main client class
│       ├── Models/
│       │   ├── Provider.swift
│       │   ├── FileMetadata.swift
│       │   └── StoragePreferences.swift
│       ├── Discovery/
│       │   └── DiscoveryClient.swift
│       ├── Storage/
│       │   ├── StorageManager.swift
│       │   └── UploadManager.swift
│       ├── Crypto/
│       │   └── Encryption.swift
│       └── Network/
│           └── HTTPClient.swift
└── Package.swift
```

**Core API**:
```swift
// Discovery
let client = CarbideClient(discoveryEndpoint: "https://discovery.carbide.network")

// Find providers
let providers = try await client.findProviders(
    region: .asia,
    maxPrice: 0.01,
    minCapacity: 5 * GB,
    replicationFactor: 3
)

// Upload file
let prefs = StoragePreferences(
    replicationFactor: 3,
    maxPricePerGB: 0.01,
    preferredRegions: [.asia, .northAmerica],
    encryption: .aes256
)

let result = try await client.uploadFile(
    localURL: fileURL,
    preferences: prefs,
    progress: { progress in
        print("Upload: \(progress.percentage)%")
    }
)

// Download file
let data = try await client.downloadFile(
    fileID: result.fileID,
    decryptionKey: result.encryptionKey
)
```

**Priority**: **🔴 CRITICAL**
**Estimated Work**: 5-7 days
**Dependencies**: Discovery Service

---

#### 1.2 **Android Client SDK** (Optional - iOS first) 🤖
**Location**: New package - `carbide-android-sdk/`

**Kotlin implementation** (same API as Swift)

**Priority**: **🟢 LOW - Do iOS first**
**Estimated Work**: 5-7 days
**Dependencies**: Discovery Service, Swift SDK (for reference)

---

#### 1.3 **Client-Side Encryption** 🔐
**Location**: In Swift SDK - `Sources/CarbideSDK/Crypto/`

**What to implement**:
```swift
class FileEncryption {
    // Generate encryption key
    static func generateKey() -> Data

    // Encrypt file before upload
    static func encrypt(data: Data, key: Data) throws -> Data

    // Decrypt after download
    static func decrypt(data: Data, key: Data) throws -> Data

    // Split file into chunks
    static func splitIntoChunks(data: Data, chunkSize: Int) -> [Data]
}
```

**Why critical**:
- Providers should NEVER see unencrypted data
- Zero-knowledge storage
- User controls keys

**Priority**: **🔴 CRITICAL - Security requirement**
**Estimated Work**: 2-3 days
**Dependencies**: None (can be done in parallel)

---

### **PHASE 2: Mobile App Integration** 📱
*Integrate SDK into existing mobile app*

#### 2.1 **Add SDK as Dependency**
**Location**: `/Users/chaalpritam/Blockbase/Carbide/`

**Update Package.swift or Podfile**:
```swift
// Package.swift
dependencies: [
    .package(url: "https://github.com/carbide-network/carbide-ios-sdk", from: "1.0.0")
]
```

**Priority**: **🟡 MEDIUM**
**Estimated Work**: 1 hour
**Dependencies**: Swift SDK must be published

---

#### 2.2 **Replace Mock Backend with Real API** 🔄
**Location**: Modify existing SwiftUI views

**Files to update**:
```
/Users/chaalpritam/Blockbase/Carbide/Carbide/
├── HomeView.swift           # Replace mock data
├── FilesView.swift          # Real file operations
├── FileDetailView.swift     # Real file metadata
├── SettingsView.swift       # Provider preferences
└── Models/
    ├── FileItem.swift       # Add network properties
    └── StorageManager.swift # New - SDK wrapper
```

**Example changes**:
```swift
// OLD (Mock)
@State private var files: [FileItem] = MockData.files

// NEW (Real)
@StateObject private var storageManager = StorageManager()
@State private var files: [FileItem] = []

Task {
    files = await storageManager.fetchFiles()
}
```

**Priority**: **🟡 MEDIUM**
**Estimated Work**: 3-4 days
**Dependencies**: Swift SDK, Discovery Service

---

#### 2.3 **File Upload Flow** ⬆️
**Location**: New view - `UploadView.swift`

**UI Flow**:
```
1. User selects file from Photos/Files app
2. Show provider selection screen:
   - Choose replication factor (1-10)
   - Set max price per GB
   - Select preferred regions
   - View estimated monthly cost
3. File encrypts locally
4. Upload progress bar (multi-provider)
5. Success: Show file ID + backup encryption key
```

**Code structure**:
```swift
struct UploadView: View {
    @State private var selectedFile: URL?
    @State private var replicationFactor: Int = 3
    @State private var maxPrice: Double = 0.01
    @State private var uploadProgress: Double = 0.0

    var body: some View {
        VStack {
            FilePickerButton(selection: $selectedFile)

            PreferenceSettings(
                replication: $replicationFactor,
                maxPrice: $maxPrice
            )

            if uploadProgress > 0 {
                ProgressBar(value: uploadProgress)
            }

            Button("Upload") {
                Task {
                    await uploadFile()
                }
            }
        }
    }

    func uploadFile() async {
        let result = try await storageManager.upload(
            file: selectedFile!,
            replication: replicationFactor,
            maxPrice: maxPrice
        )

        // Save encryption key securely
        KeychainManager.save(key: result.encryptionKey, for: result.fileID)
    }
}
```

**Priority**: **🟡 MEDIUM**
**Estimated Work**: 2-3 days
**Dependencies**: Swift SDK

---

#### 2.4 **File Download Flow** ⬇️
**Location**: Update `FileDetailView.swift`

**UI Flow**:
```
1. User taps file
2. Show file details + download button
3. Download from available providers (try multiple if one fails)
4. Decrypt locally
5. Open/preview file
```

**Priority**: **🟡 MEDIUM**
**Estimated Work**: 1-2 days
**Dependencies**: Swift SDK

---

#### 2.5 **Metadata Storage** 💾
**Location**: New model - `FileMetadataStore.swift`

**What to store locally** (using SwiftData):
```swift
@Model
class FileMetadata {
    var id: UUID
    var name: String
    var size: Int64
    var uploadDate: Date
    var encryptionKey: Data  // Encrypted with device key
    var providerIDs: [UUID]  // Where file is stored
    var replicationFactor: Int
    var monthlyPrice: Double
    var fileHash: String     // For integrity verification
}
```

**Why needed**:
- Track which files user has uploaded
- Store encryption keys securely
- Know which providers have each file
- Calculate storage costs

**Priority**: **🔴 CRITICAL**
**Estimated Work**: 1-2 days
**Dependencies**: Swift SDK

---

### **PHASE 3: Advanced Features** ⚡
*Not required for MVP but important for production*

#### 3.1 **Provider Selection UI** 🎯
**Location**: New view - `ProviderSelectionView.swift`

**Features**:
- Map view showing provider locations
- Filter by price, region, reputation
- Show provider stats (uptime, capacity)
- Save favorite providers

**Priority**: **🟢 LOW - Can use auto-selection initially**
**Estimated Work**: 2-3 days

---

#### 3.2 **Cost Tracking Dashboard** 💰
**Location**: Update `HomeView.swift`

**Show**:
- Total storage used
- Monthly costs (per file, per provider)
- Cost projection
- Payment history

**Priority**: **🟢 LOW**
**Estimated Work**: 2 days

---

#### 3.3 **Background Sync** 🔄
**Location**: New - `BackgroundSyncManager.swift`

**Features**:
- Upload queue (resume interrupted uploads)
- Automatic re-upload if provider goes offline
- Background fetch for file integrity checks
- Optimize for battery/network

**Priority**: **🟡 MEDIUM**
**Estimated Work**: 3-4 days

---

#### 3.4 **File Sharing** 🔗
**Location**: New view - `ShareView.swift`

**Features**:
- Generate share links
- Set expiration time
- Password protection
- Track downloads

**Priority**: **🟢 LOW**
**Estimated Work**: 2-3 days

---

#### 3.5 **Payment Integration** 💳
**Location**: New service - `PaymentManager.swift`

**Options**:
1. **Crypto payments** (preferred for decentralization)
   - Solana SPL token transfers (low fees, sub-second finality)
   - USDC on Solana
2. **Traditional payments**
   - Stripe
   - Apple Pay
   - Credit card

**Integration**:
```swift
// Charge user monthly based on storage
let monthlyBill = calculateBill(for: userId)

// Pay each provider
for provider in activeProviders {
    await paymentManager.transfer(
        to: provider.walletAddress,
        amount: provider.monthlyCharge
    )
}
```

**Priority**: **🔴 CRITICAL for production**
**Estimated Work**: 5-7 days
**Dependencies**: Legal/compliance review

---

## 📊 **Recommended Implementation Order**

### **Sprint 1: Discovery & Foundation (Week 1-2)**
```
Day 1-3:   Build Discovery Service ⚠️ CRITICAL
Day 4:     Add Provider Auto-Registration
Day 5-7:   Start Swift SDK (basic structure)
Day 8-10:  Encryption implementation
Day 11-14: Complete Swift SDK (upload/download)
```

### **Sprint 2: Mobile App Integration (Week 3-4)**
```
Day 15:    Add SDK dependency to mobile app
Day 16-18: Replace mock data with real API
Day 19-21: File upload flow
Day 22-24: File download flow
Day 25-28: Metadata storage + UI polish
```

### **Sprint 3: Testing & Production Prep (Week 5)**
```
Day 29-30: End-to-end testing
Day 31:    Security audit
Day 32-33: Performance optimization
Day 34-35: Beta testing with real users
```

### **Sprint 4: Advanced Features (Week 6+)**
```
- Background sync
- Provider selection UI
- Cost tracking
- Payment integration
- File sharing
```

---

## 🔐 **Security Checklist**

### **Client-Side (Mobile App)**
- [ ] All files encrypted before leaving device
- [ ] Encryption keys stored in Keychain (iOS) / Keystore (Android)
- [ ] HTTPS only for all network requests
- [ ] Certificate pinning for discovery service
- [ ] No sensitive data in logs
- [ ] Secure random number generation for keys

### **Network**
- [ ] TLS 1.3 minimum
- [ ] Validate SSL certificates
- [ ] Timeout protection (prevent hanging requests)
- [ ] Rate limiting on client side
- [ ] Request signing (HMAC)

### **Provider Trust**
- [ ] Never trust providers with unencrypted data
- [ ] Verify file integrity after download (hash check)
- [ ] Multiple providers for redundancy
- [ ] Regular integrity checks (proof-of-storage)

---

## 🧪 **Testing Strategy**

### **Unit Tests**
```swift
// Test encryption
func testFileEncryption() {
    let data = "Hello World".data(using: .utf8)!
    let key = FileEncryption.generateKey()
    let encrypted = try! FileEncryption.encrypt(data: data, key: key)
    let decrypted = try! FileEncryption.decrypt(data: encrypted, key: key)
    XCTAssertEqual(data, decrypted)
}

// Test discovery
func testProviderDiscovery() async {
    let client = CarbideClient(discoveryEndpoint: testEndpoint)
    let providers = try! await client.findProviders(region: .asia)
    XCTAssertGreaterThan(providers.count, 0)
}
```

### **Integration Tests**
```swift
// Test full upload flow
func testFileUpload() async {
    let testFile = createTestFile(size: 1 * MB)
    let result = try! await client.uploadFile(
        localURL: testFile,
        preferences: .default
    )
    XCTAssertNotNil(result.fileID)
    XCTAssertEqual(result.providersUsed.count, 3)
}

// Test download and verification
func testDownloadVerification() async {
    let originalHash = calculateHash(data: originalData)
    let downloaded = try! await client.downloadFile(fileID: fileID)
    let downloadedHash = calculateHash(data: downloaded)
    XCTAssertEqual(originalHash, downloadedHash)
}
```

### **E2E Tests**
```
1. User opens app
2. Selects file from Photos
3. Sets replication = 3, max price = $0.01
4. Upload succeeds
5. File appears in file list
6. Tap file to download
7. Download succeeds
8. File matches original
```

---

## 📦 **Dependencies & Tools**

### **Backend Services**
```toml
# Discovery Service
axum = "0.7"           # HTTP framework
sqlx = "0.7"           # Database
tokio = "1.0"          # Async runtime
redis = "0.24"         # Caching
```

### **Swift SDK**
```swift
// Package.swift
dependencies: [
    .package(url: "https://github.com/Alamofire/Alamofire", from: "5.8.0"),
    .package(url: "https://github.com/apple/swift-crypto", from: "3.0.0"),
]
```

### **Mobile App**
```swift
// SwiftData for local storage
// CryptoKit for encryption
// Combine for reactive programming
// SwiftUI for UI
```

---

## 🚀 **Deployment Checklist**

### **Discovery Service**
- [ ] Deploy to cloud (AWS/GCP/Azure)
- [ ] Setup load balancer
- [ ] Configure SSL certificate
- [ ] Setup monitoring (Prometheus/Grafana)
- [ ] Database backups
- [ ] Rate limiting
- [ ] DDoS protection

### **Mobile App**
- [ ] App Store submission
- [ ] Privacy policy
- [ ] Terms of service
- [ ] Beta testing (TestFlight)
- [ ] Crash reporting (Sentry/Firebase)
- [ ] Analytics

---

## 💡 **MVP Scope (Minimum Viable Product)**

**To launch mobile app MVP, you MUST have**:

1. ✅ Discovery Service (find providers)
2. ✅ Swift SDK (upload/download)
3. ✅ Client-side encryption
4. ✅ Basic UI (upload, list, download)
5. ✅ Metadata storage
6. ✅ Provider auto-registration

**Nice to have but NOT required for MVP**:
- ❌ Payment integration (free beta)
- ❌ Provider selection UI (auto-select)
- ❌ Background sync (manual for MVP)
- ❌ File sharing (add later)
- ❌ Advanced analytics

---

## 📈 **Success Metrics**

### **Phase 0-1 Success (Backend Ready)**
- Discovery service returning providers in < 100ms
- At least 10 providers registered
- Swift SDK can upload 1MB file in < 5s
- Zero unencrypted data transmitted

### **Phase 2 Success (Mobile App Working)**
- User can upload file from mobile app
- File stored on 3 different providers
- User can download and verify file
- App works offline (queues uploads)

### **Production Ready**
- 100+ active providers
- 99.9% uptime for discovery service
- < 1% failed uploads
- < 5% failed downloads (with retry)
- Average upload speed > 1 MB/s

---

## ⚠️ **Critical Blockers**

**You CANNOT build the mobile app without**:

1. **Discovery Service** - How will app find providers?
2. **Provider Registration** - How will providers announce themselves?
3. **Public Provider Endpoints** - Providers must be accessible from internet

**Current Problem**:
- Providers running on home networks are behind NAT
- Need NAT traversal or gateway service

**Solutions**:
- Option A: Gateway service (providers connect to gateway)
- Option B: UPnP/NAT-PMP (automatic port forwarding)
- Option C: Tunnel service (ngrok-like)

---

## 🎯 **Next Steps (Start Tomorrow)**

### **Day 1: Setup Discovery Service**
```bash
cd carbide-node
mkdir -p services/carbide-discovery
cd services/carbide-discovery
cargo init --name carbide-discovery

# Create basic HTTP server
# Setup PostgreSQL database
# Implement provider registration API
```

### **Day 2-3: Provider Auto-Registration**
```bash
cd crates/carbide-provider
# Modify src/main.rs to register with discovery on startup
# Add heartbeat task
# Test with local discovery service
```

### **Day 4-7: Swift SDK**
```bash
cd /Users/chaalpritam/Blockbase
mkdir carbide-ios-sdk
cd carbide-ios-sdk
swift package init --type library

# Implement discovery client
# Implement upload manager
# Implement encryption
```

### **Week 2: Mobile App Integration**
```bash
cd /Users/chaalpritam/Blockbase/Carbide
# Add SDK dependency
# Replace mock data
# Test upload flow
```

---

## 📞 **Questions to Answer Before Starting**

1. **Where will Discovery Service run?**
   - Cloud provider? (AWS, GCP, Digital Ocean)
   - Domain name? (discovery.carbide.network)
   - Budget for hosting?

2. **How will home providers be accessible?**
   - Gateway service?
   - Port forwarding?
   - VPN/tunnel?

3. **Payment model for MVP?**
   - Free beta?
   - Crypto payments?
   - Credit card?

4. **Target platforms?**
   - iOS only first?
   - Android too?
   - Web app?

5. **Storage limits for MVP?**
   - Max file size? (100MB? 1GB?)
   - Max total storage per user? (1GB? 10GB?)

---

## 🎓 **Summary**

**To make mobile app work with carbide-node network**:

1. **Build Discovery Service** (3-5 days) - CRITICAL ⚠️
2. **Update Provider to auto-register** (1 day) - CRITICAL ⚠️
3. **Build Swift SDK** (5-7 days) - CRITICAL ⚠️
4. **Integrate SDK into mobile app** (3-4 days)
5. **Test end-to-end** (2-3 days)

**Total MVP Timeline**: 3-4 weeks

**First milestone**: Discovery service running with 3+ providers registered

**Second milestone**: Can upload 1MB file from Swift SDK to providers

**Third milestone**: Mobile app can upload/download files successfully

🚀 **Start with Discovery Service - everything else depends on it!**
