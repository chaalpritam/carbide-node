//! # Carbide Network Communication Demo
//!
//! Demonstrates the HTTP server and client communication for the
//! Carbide Network, including provider APIs and marketplace interactions.

use std::time::Duration;

use carbide_core::{network::*, ContentHash, Provider, ProviderTier, Region};
use rust_decimal::Decimal;
use serde_json;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌐 Carbide Network Communication Demo");
    println!("===================================\n");

    // 1. Demonstrate message creation and serialization
    demo_message_types().await?;

    // 2. Show network configuration
    demo_network_config();

    // 3. Demonstrate API endpoint structure
    demo_api_endpoints();

    println!("\n✅ Networking demo completed! This shows:");
    println!("   • JSON message serialization for network communication");
    println!("   • RESTful API endpoints for all operations");
    println!("   • Network configuration and timeouts");
    println!("   • Provider-client communication protocols");
    println!("   • Marketplace quote and storage request flows");

    Ok(())
}

async fn demo_message_types() -> Result<(), Box<dyn std::error::Error>> {
    println!("📨 Network Message Types and Serialization");
    println!("-----------------------------------------");

    // 1. Health check messages
    println!("🏥 Health Check Messages:");
    let health_response = HealthCheckResponse {
        status: ServiceStatus::Healthy,
        timestamp: chrono::Utc::now(),
        version: "1.0.0".to_string(),
        available_storage: Some(50 * 1024 * 1024 * 1024), // 50GB
        load: Some(0.3),                                  // 30% load
        reputation: Some(Decimal::new(92, 2)),            // 0.92 reputation
    };

    let health_message = NetworkMessage::new(MessageType::HealthCheckResponse(health_response));

    let json = serde_json::to_string_pretty(&health_message)?;
    println!("   Health Response JSON ({} bytes):", json.len());
    println!(
        "   {}",
        json.lines().take(10).collect::<Vec<_>>().join("\n")
    );
    println!("   ... (truncated)");

    // 2. Storage quote messages
    println!("\n💰 Storage Quote Messages:");
    let quote_request = StorageQuoteRequest {
        file_size: 100 * 1024 * 1024, // 100MB
        replication_factor: 3,
        duration_months: 12,
        requirements: carbide_core::ProviderRequirements::important(),
        preferred_regions: vec![Region::NorthAmerica, Region::Europe],
    };

    let quote_request_message =
        NetworkMessage::new(MessageType::StorageQuoteRequest(quote_request.clone()));

    println!("   Quote Request:");
    println!(
        "     File Size: {} bytes ({:.1} MB)",
        quote_request.file_size,
        quote_request.file_size as f64 / (1024.0 * 1024.0)
    );
    println!(
        "     Replication: {} copies",
        quote_request.replication_factor
    );
    println!("     Duration: {} months", quote_request.duration_months);
    println!(
        "     Preferred Regions: {:?}",
        quote_request.preferred_regions
    );

    let quote_response = StorageQuoteResponse {
        provider_id: uuid::Uuid::new_v4(),
        price_per_gb_month: Decimal::new(4, 3),  // $0.004
        total_monthly_cost: Decimal::new(12, 3), // $0.012
        can_fulfill: true,
        available_capacity: 500 * 1024 * 1024 * 1024, // 500GB
        estimated_start_time: 2,                      // 2 hours
        valid_until: chrono::Utc::now() + chrono::Duration::hours(24),
    };

    println!("   Quote Response:");
    println!(
        "     Price: ${}/GB/month",
        quote_response.price_per_gb_month
    );
    println!(
        "     Total Cost: ${}/month",
        quote_response.total_monthly_cost
    );
    println!("     Can Fulfill: {}", quote_response.can_fulfill);
    println!(
        "     Available: {:.1} GB",
        quote_response.available_capacity as f64 / (1024.0 * 1024.0 * 1024.0)
    );

    // 3. File storage messages
    println!("\n📁 File Storage Messages:");
    let file_id = ContentHash::from_data(b"demo_file_content_123");
    let store_request = StoreFileRequest {
        file_id,
        file_size: 50 * 1024 * 1024, // 50MB
        duration_months: 6,
        encryption_info: Some(EncryptionInfo {
            algorithm: "AES-256-GCM".to_string(),
            key_derivation: Some(KeyDerivationInfo {
                method: "PBKDF2".to_string(),
                salt: "abcdef123456".to_string(),
                iterations: 100000,
            }),
            is_encrypted: true,
        }),
        requirements: carbide_core::ProviderRequirements::critical(),
        max_price: Decimal::new(8, 3), // $0.008/GB/month
    };

    println!("   Store Request:");
    println!("     File ID: {}...", &file_id.to_hex()[..16]);
    println!(
        "     File Size: {} bytes ({:.1} MB)",
        store_request.file_size,
        store_request.file_size as f64 / (1024.0 * 1024.0)
    );
    println!("     Duration: {} months", store_request.duration_months);
    println!(
        "     Encrypted: {}",
        store_request
            .encryption_info
            .as_ref()
            .map(|e| e.is_encrypted)
            .unwrap_or(false)
    );
    println!("     Max Price: ${}/GB/month", store_request.max_price);

    // 4. Proof of storage messages
    println!("\n🛡️ Proof of Storage Messages:");
    let challenge = StorageChallengeData {
        challenge_id: "challenge_abc123".to_string(),
        file_hash: file_id,
        chunk_indices: vec![0, 2, 5, 8],
        nonce: [42u8; 32],
        issued_at: chrono::Utc::now(),
        expires_at: chrono::Utc::now() + chrono::Duration::minutes(10),
        expected_response_hash: ContentHash::from_data(b"expected_response"),
    };

    println!("   Challenge:");
    println!("     ID: {}", challenge.challenge_id);
    println!("     File: {}...", &challenge.file_hash.to_hex()[..16]);
    println!("     Chunks to prove: {:?}", challenge.chunk_indices);
    println!(
        "     Expires: {} minutes",
        (challenge.expires_at - challenge.issued_at).num_minutes()
    );

    let proof = StorageProofData {
        challenge_id: challenge.challenge_id.clone(),
        merkle_proofs: vec![ChunkProofData {
            chunk_index: 0,
            chunk_hash: ContentHash::from_data(b"chunk_0_data"),
            merkle_path: vec![
                ContentHash::from_data(b"sibling_1"),
                ContentHash::from_data(b"sibling_2"),
            ],
            chunk_data: None,
        }],
        response_hash: ContentHash::from_data(b"proof_response"),
        signature: vec![0u8; 64],
        generated_at: chrono::Utc::now(),
    };

    println!("   Proof Response:");
    println!("     Challenge ID: {}", proof.challenge_id);
    println!("     Merkle Proofs: {} chunks", proof.merkle_proofs.len());
    println!("     Signature: {} bytes", proof.signature.len());

    // 5. Error messages
    println!("\n❌ Error Messages:");
    let error_msg = ErrorMessage {
        code: ErrorCodes::INSUFFICIENT_STORAGE.to_string(),
        message: "Not enough storage space available".to_string(),
        details: Some(
            [
                ("requested".to_string(), "100GB".to_string()),
                ("available".to_string(), "50GB".to_string()),
            ]
            .iter()
            .cloned()
            .collect(),
        ),
    };

    println!("   Error:");
    println!("     Code: {}", error_msg.code);
    println!("     Message: {}", error_msg.message);
    if let Some(details) = &error_msg.details {
        for (key, value) in details {
            println!("     {}: {}", key, value);
        }
    }

    Ok(())
}

fn demo_network_config() {
    println!("\n⚙️ Network Configuration");
    println!("------------------------");

    let config = NetworkConfig::default();

    println!("📡 Default Configuration:");
    println!(
        "   Max Message Size: {:.1} MB",
        config.max_message_size as f64 / (1024.0 * 1024.0)
    );
    println!("   Request Timeout: {} seconds", config.request_timeout);
    println!("   Keep-Alive: {} seconds", config.keep_alive_timeout);
    println!("   Max Connections: {}", config.max_connections);
    println!("   Compression: {}", config.compression);

    if let Some(rate_limit) = config.rate_limit {
        println!("   Rate Limit: {} req/min", rate_limit);
    }

    // Custom configuration for different scenarios
    println!("\n🏠 Home Provider Config:");
    let home_config = NetworkConfig {
        max_message_size: 10 * 1024 * 1024, // 10MB
        request_timeout: 60,                // 60 seconds
        max_connections: 100,
        rate_limit: Some(30), // 30 req/min
        ..config
    };

    println!(
        "   Max Message Size: {:.1} MB",
        home_config.max_message_size as f64 / (1024.0 * 1024.0)
    );
    println!(
        "   Request Timeout: {} seconds",
        home_config.request_timeout
    );
    println!("   Max Connections: {}", home_config.max_connections);

    println!("\n🏢 Enterprise Provider Config:");
    let enterprise_config = NetworkConfig {
        max_message_size: 1024 * 1024 * 1024, // 1GB
        request_timeout: 300,                 // 5 minutes
        max_connections: 10000,
        rate_limit: None, // No rate limiting
        ..config
    };

    println!(
        "   Max Message Size: {:.1} GB",
        enterprise_config.max_message_size as f64 / (1024.0 * 1024.0 * 1024.0)
    );
    println!(
        "   Request Timeout: {} seconds",
        enterprise_config.request_timeout
    );
    println!("   Max Connections: {}", enterprise_config.max_connections);
    println!("   Rate Limit: Unlimited");
}

fn demo_api_endpoints() {
    println!("\n🌍 API Endpoints and REST Interface");
    println!("---------------------------------");

    println!("📍 Core Endpoints:");
    println!("   Health Check:     GET  {}", ApiEndpoints::HEALTH_CHECK);
    println!(
        "   Provider Status:  GET  {}",
        ApiEndpoints::PROVIDER_STATUS
    );
    println!("   Provider List:    GET  {}", ApiEndpoints::PROVIDER_LIST);

    println!("\n📁 File Operations:");
    println!("   Store File:       POST {}", ApiEndpoints::FILE_STORE);
    println!(
        "   Retrieve File:    GET  {}/{{file_id}}",
        ApiEndpoints::FILE_RETRIEVE
    );
    println!(
        "   Delete File:      DEL  {}/{{file_id}}",
        ApiEndpoints::FILE_DELETE
    );
    println!("   Upload Data:      POST {}", ApiEndpoints::FILE_UPLOAD);
    println!(
        "   Download Data:    GET  {}/{{file_id}}",
        ApiEndpoints::FILE_DOWNLOAD
    );

    println!("\n💰 Marketplace:");
    println!("   Storage Quote:    POST {}", ApiEndpoints::STORAGE_QUOTE);
    println!(
        "   Storage Contract: POST {}",
        ApiEndpoints::STORAGE_CONTRACT
    );

    println!("\n🛡️ Proof of Storage:");
    println!(
        "   Send Challenge:   POST {}",
        ApiEndpoints::PROOF_CHALLENGE
    );
    println!("   Submit Proof:     POST {}", ApiEndpoints::PROOF_RESPONSE);

    println!("\n📊 Monitoring:");
    println!("   Metrics:          GET  {}", ApiEndpoints::METRICS);

    println!("\n🔗 Example API Flows:");
    println!("   1. Provider Discovery:");
    println!("      GET /api/v1/providers?region=northamerica&tier=professional");

    println!("   2. Storage Quote:");
    println!("      POST /api/v1/marketplace/quote");
    println!("      {{\"file_size\": 104857600, \"replication_factor\": 3}}");

    println!("   3. File Storage:");
    println!("      POST /api/v1/files/store");
    println!("      POST /api/v1/upload (with multipart data)");

    println!("   4. Health Monitoring:");
    println!("      GET /api/v1/health");
    println!("      GET /api/v1/provider/status");

    println!("\n📈 HTTP Status Codes:");
    println!("   200 OK           - Successful request");
    println!("   201 Created      - Resource created (contract, etc.)");
    println!("   400 Bad Request  - Invalid request format");
    println!("   401 Unauthorized - Authentication required");
    println!("   404 Not Found    - File or provider not found");
    println!("   409 Conflict     - Storage conflict or capacity issue");
    println!("   413 Too Large    - File exceeds size limits");
    println!("   500 Server Error - Internal provider error");
    println!("   503 Unavailable  - Provider temporarily unavailable");
}
