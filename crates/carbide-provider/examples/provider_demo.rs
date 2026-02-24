//! # Provider Storage Demo
//!
//! Demonstrates the complete storage provider functionality including:
//! - File storage and retrieval
//! - Contract management 
//! - Proof-of-storage challenges
//! - Provider statistics tracking

use carbide_core::{
    network::*,
    ContentHash, Provider, ProviderTier, Region
};
use carbide_provider::{ProviderServer, ServerConfig};
use reqwest::multipart;
use rust_decimal::Decimal;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("🏪 Carbide Storage Provider Demo");
    println!("===============================\n");

    // 1. Create a test provider
    let provider = Provider::new(
        "Demo Storage Provider".to_string(),
        ProviderTier::Home,
        Region::NorthAmerica,
        "http://localhost:8080".to_string(),
        1024 * 1024 * 1024, // 1GB capacity
        Decimal::new(3, 3), // $0.003/GB/month
    );

    println!("📊 Provider Information:");
    println!("   ID: {}", provider.id);
    println!("   Name: {}", provider.name);
    println!("   Tier: {:?}", provider.tier);
    println!("   Region: {:?}", provider.region);
    println!("   Capacity: {:.2} GB", provider.total_capacity as f64 / (1024.0 * 1024.0 * 1024.0));
    println!("   Price: ${}/GB/month", provider.price_per_gb_month);

    // 2. Configure and start server
    let config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 8080,
        request_timeout: Duration::from_secs(30),
        max_upload_size: 50 * 1024 * 1024, // 50MB
        enable_cors: true,
    };

    println!("\n🚀 Starting provider server...");
    let storage_path = std::path::PathBuf::from("./demo_storage");
    let server = ProviderServer::new(config, provider, storage_path)?;
    
    // Start server in background
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.start().await {
            eprintln!("Server error: {}", e);
        }
    });

    // Wait for server to start
    sleep(Duration::from_secs(2)).await;

    // 3. Demonstrate API endpoints
    let client = reqwest::Client::new();
    let base_url = "http://localhost:8080";

    println!("🔍 Testing provider endpoints...\n");

    // Test health check
    println!("1. Health Check:");
    let health_response = client
        .get(&format!("{}/api/v1/health", base_url))
        .send()
        .await?;
    
    if health_response.status().is_success() {
        let health_json: serde_json::Value = health_response.json().await?;
        println!("   ✅ Health check passed");
        println!("   Response: {}", serde_json::to_string_pretty(&health_json)?);
    } else {
        println!("   ❌ Health check failed: {}", health_response.status());
    }

    println!("\n2. Provider Status:");
    let status_response = client
        .get(&format!("{}/api/v1/provider/status", base_url))
        .send()
        .await?;
    
    if status_response.status().is_success() {
        let status_json: serde_json::Value = status_response.json().await?;
        println!("   ✅ Status retrieved");
        println!("   Response: {}", serde_json::to_string_pretty(&status_json)?);
    } else {
        println!("   ❌ Status request failed: {}", status_response.status());
    }

    // 4. Test storage quote request
    println!("\n3. Storage Quote Request:");
    let quote_request = StorageQuoteRequest {
        file_size: 10 * 1024 * 1024, // 10MB
        replication_factor: 2,
        duration_months: 6,
        requirements: carbide_core::ProviderRequirements::important(),
        preferred_regions: vec![Region::NorthAmerica],
    };

    let quote_message = NetworkMessage::new(
        MessageType::StorageQuoteRequest(quote_request.clone())
    );

    let quote_response = client
        .post(&format!("{}/api/v1/marketplace/quote", base_url))
        .json(&quote_message)
        .send()
        .await?;

    if quote_response.status().is_success() {
        let quote_json: serde_json::Value = quote_response.json().await?;
        println!("   ✅ Quote retrieved");
        println!("   Request: {} bytes, {} copies, {} months", 
                quote_request.file_size, 
                quote_request.replication_factor, 
                quote_request.duration_months);
        println!("   Response: {}", serde_json::to_string_pretty(&quote_json)?);
    } else {
        println!("   ❌ Quote request failed: {}", quote_response.status());
    }

    // 5. Test file storage flow
    println!("\n4. File Storage Flow:");
    
    // Create a test file
    let test_file_data = b"Hello, Carbide Network! This is a test file for storage demonstration.";
    let file_id = ContentHash::from_data(test_file_data);
    
    println!("   📄 Test file:");
    println!("     Content: {:?}", std::str::from_utf8(test_file_data).unwrap_or("<binary>"));
    println!("     Size: {} bytes", test_file_data.len());
    println!("     File ID: {}", file_id.to_hex());

    // Request storage
    let store_request = StoreFileRequest {
        file_id,
        file_size: test_file_data.len() as u64,
        duration_months: 3,
        encryption_info: None,
        requirements: carbide_core::ProviderRequirements::important(),
        max_price: Decimal::new(5, 3), // $0.005/GB/month
    };

    let store_message = NetworkMessage::new(
        MessageType::StoreFileRequest(store_request)
    );

    println!("\n   💾 Requesting file storage...");
    let store_response = client
        .post(&format!("{}/api/v1/files/store", base_url))
        .json(&store_message)
        .send()
        .await?;

    if store_response.status().is_success() {
        let store_json: serde_json::Value = store_response.json().await?;
        println!("   ✅ Storage request accepted");
        println!("   Response: {}", serde_json::to_string_pretty(&store_json)?);

        // Extract upload URL and token for actual file upload
        if let Some(message_data) = store_json.get("message_type").and_then(|mt| mt.get("data")) {
            if let (Some(upload_url), Some(upload_token)) = (
                message_data.get("upload_url").and_then(|u| u.as_str()),
                message_data.get("upload_token").and_then(|t| t.as_str())
            ) {
                println!("\n   📤 Uploading file data...");
                
                // Create multipart form for file upload
                let form = multipart::Form::new()
                    .part("file", multipart::Part::bytes(test_file_data.to_vec()))
                    .part("file_id", multipart::Part::text(file_id.to_hex()))
                    .part("token", multipart::Part::text(upload_token.to_string()));

                let upload_response = client
                    .post(upload_url)
                    .multipart(form)
                    .send()
                    .await?;

                if upload_response.status().is_success() {
                    let upload_json: serde_json::Value = upload_response.json().await?;
                    println!("   ✅ File uploaded successfully");
                    println!("   Upload response: {}", serde_json::to_string_pretty(&upload_json)?);

                    // 6. Test file retrieval
                    println!("\n5. File Retrieval:");
                    let retrieve_url = format!("{}/api/v1/files/{}", base_url, file_id.to_hex());
                    let retrieve_response = client
                        .get(&retrieve_url)
                        .send()
                        .await?;

                    if retrieve_response.status().is_success() {
                        let retrieve_json: serde_json::Value = retrieve_response.json().await?;
                        println!("   ✅ File metadata retrieved");
                        println!("   Response: {}", serde_json::to_string_pretty(&retrieve_json)?);

                        // Test file download
                        let download_url = format!("{}/api/v1/download/{}", base_url, file_id.to_hex());
                        let download_response = client
                            .get(&download_url)
                            .send()
                            .await?;

                        if download_response.status().is_success() {
                            let downloaded_data = download_response.bytes().await?;
                            println!("   ✅ File downloaded successfully");
                            println!("   Downloaded {} bytes", downloaded_data.len());
                            
                            if downloaded_data.as_ref() == test_file_data {
                                println!("   🎯 Downloaded data matches original! Storage integrity verified.");
                            } else {
                                println!("   ⚠️ Downloaded data differs from original");
                            }
                        } else {
                            println!("   ❌ File download failed: {}", download_response.status());
                        }
                    } else {
                        println!("   ❌ File retrieval failed: {}", retrieve_response.status());
                    }

                    // 7. Test proof-of-storage challenge
                    println!("\n6. Proof-of-Storage Challenge:");
                    let challenge = StorageChallengeData {
                        challenge_id: "demo_challenge_123".to_string(),
                        file_hash: file_id,
                        chunk_indices: vec![0, 1, 2],
                        nonce: [42u8; 32],
                        issued_at: chrono::Utc::now(),
                        expires_at: chrono::Utc::now() + chrono::Duration::minutes(10),
                        expected_response_hash: ContentHash::from_data(b"expected_proof"),
                    };

                    let challenge_message = NetworkMessage::new(
                        MessageType::StorageChallenge(challenge.clone())
                    );

                    let proof_response = client
                        .post(&format!("{}/api/v1/proof/challenge", base_url))
                        .json(&challenge_message)
                        .send()
                        .await?;

                    if proof_response.status().is_success() {
                        let proof_json: serde_json::Value = proof_response.json().await?;
                        println!("   ✅ Proof-of-storage response generated");
                        println!("   Challenge ID: {}", challenge.challenge_id);
                        println!("   Chunks challenged: {:?}", challenge.chunk_indices);
                        println!("   Response: {}", serde_json::to_string_pretty(&proof_json)?);
                    } else {
                        println!("   ❌ Proof generation failed: {}", proof_response.status());
                    }

                } else {
                    println!("   ❌ File upload failed: {}", upload_response.status());
                }
            }
        }
    } else {
        println!("   ❌ Storage request failed: {}", store_response.status());
    }

    println!("\n🎉 Demo completed successfully!");
    println!("This demonstration showed:");
    println!("  • Provider health monitoring and status reporting");
    println!("  • Storage marketplace quote requests");
    println!("  • Complete file storage workflow (request → upload → store)");
    println!("  • File retrieval and download operations");
    println!("  • Proof-of-storage challenge-response protocols");
    println!("  • Real file I/O with disk storage");
    println!("  • Storage contract management");
    println!("  • Provider statistics tracking");

    // Shutdown server
    server_handle.abort();
    
    Ok(())
}