//! Integration tests with a mock discovery server.
//!
//! Spins up a lightweight Axum server on a random port and validates
//! DiscoveryClient, PaymentClient, and FileRegistry integration.

use std::net::SocketAddr;

use axum::{
    extract::Path,
    routing::{get, post},
    Json, Router,
};
use carbide_client::file_registry::{FileRecord, FileRegistry, ProviderLocation};
use serde_json::json;
use tokio::net::TcpListener;

/// Start a mock discovery server on a random port and return its address.
async fn start_mock_server() -> SocketAddr {
    let app = Router::new()
        // Mock: GET /api/v1/providers
        .route(
            "/api/v1/providers",
            get(|| async {
                Json(json!({
                    "providers": [
                        {
                            "id": "00000000-0000-0000-0000-000000000001",
                            "name": "Mock Provider",
                            "tier": "Professional",
                            "region": "NorthAmerica",
                            "endpoint": "http://localhost:8080",
                            "available_capacity": 10_000_000_000_u64,
                            "total_capacity": 25_000_000_000_u64,
                            "price_per_gb_month": "0.005",
                            "reputation": {
                                "overall": "0.8",
                                "uptime": "0.9",
                                "data_integrity": "0.9",
                                "response_time": "0.7",
                                "contract_compliance": "0.8",
                                "community_feedback": "0.7",
                                "contracts_completed": 0,
                                "last_updated": "2025-01-01T00:00:00Z"
                            },
                            "last_seen": "2025-01-01T00:00:00Z",
                            "metadata": {},
                            "wallet_address": null
                        }
                    ],
                    "total_count": 1,
                    "has_more": false
                }))
            }),
        )
        // Mock: POST /api/v1/contracts
        .route(
            "/api/v1/contracts",
            post(|Json(body): Json<serde_json::Value>| async move {
                Json(json!({
                    "id": "contract-mock-123",
                    "client_id": body["client_id"],
                    "provider_id": body["provider_id"],
                    "status": "pending_deposit",
                    "price_per_gb_month": body["price_per_gb_month"],
                    "duration_days": body["duration_days"],
                    "total_escrowed": null,
                    "total_released": null,
                    "created_at": "2025-01-01T00:00:00Z"
                }))
            }),
        )
        // Mock: POST /api/v1/contracts/:id/deposit
        .route(
            "/api/v1/contracts/:id/deposit",
            post(|Path(id): Path<String>| async move {
                Json(json!({
                    "id": id,
                    "status": "active",
                    "message": "Deposit recorded"
                }))
            }),
        )
        // Mock: GET /api/v1/contracts
        .route(
            "/api/v1/contracts",
            get(|| async {
                Json(json!([
                    {
                        "id": "contract-1",
                        "provider_id": "p1",
                        "client_id": "c1",
                        "status": "active",
                        "price_per_gb_month": "0.005",
                        "duration_days": 30,
                        "total_escrowed": "1000",
                        "total_released": "0",
                        "created_at": "2025-01-01T00:00:00Z",
                        "updated_at": "2025-01-01T00:00:00Z"
                    }
                ]))
            }),
        )
        // Mock: GET /api/v1/health
        .route(
            "/api/v1/health",
            get(|| async {
                Json(json!({
                    "status": "Healthy",
                    "timestamp": "2025-01-01T00:00:00Z",
                    "version": "1.0.0",
                    "available_storage": null,
                    "load": null,
                    "reputation": null
                }))
            }),
        );

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    addr
}

#[tokio::test]
async fn discovery_client_search_providers_against_mock() {
    let addr = start_mock_server().await;
    let endpoint = format!("http://{}", addr);

    let client = carbide_client::CarbideClient::with_defaults().unwrap();
    let discovery =
        carbide_client::DiscoveryClient::new(client, endpoint);

    let query = carbide_client::MarketplaceQuery::default();
    let result = discovery.search_providers(query).await;

    assert!(result.is_ok());
    let providers = result.unwrap();
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0].provider.name, "Mock Provider");
}

#[tokio::test]
async fn payment_client_create_contract_against_mock() {
    use carbide_client::payment::{CreateContractRequest, PaymentClient};

    let addr = start_mock_server().await;
    let endpoint = format!("http://{}", addr);

    let payment = PaymentClient::new(&endpoint).unwrap();

    let request = CreateContractRequest {
        provider_id: "p1".to_string(),
        client_id: "c1".to_string(),
        price_per_gb_month: "0.005".to_string(),
        duration_days: 30,
        total_size_bytes: Some(1024),
        file_id: Some("abc123".to_string()),
        chain_id: None,
    };

    let result = payment.create_contract(&request).await;
    assert!(result.is_ok());
    let contract = result.unwrap();
    assert_eq!(contract.id, "contract-mock-123");
    assert_eq!(contract.status, "pending_deposit");
}

#[tokio::test]
async fn file_registry_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test_files.db");
    let registry = FileRegistry::open(&db_path).unwrap();

    // Record an upload
    let record = FileRecord {
        file_id: "abc123".to_string(),
        original_name: "test.txt".to_string(),
        file_size: 2048,
        is_encrypted: true,
        replication_factor: 3,
        providers: serde_json::to_string(&vec![
            ProviderLocation {
                provider_id: "p1".to_string(),
                endpoint: "http://localhost:8080".to_string(),
                contract_id: "c1".to_string(),
            },
            ProviderLocation {
                provider_id: "p2".to_string(),
                endpoint: "http://localhost:8081".to_string(),
                contract_id: "c2".to_string(),
            },
        ])
        .unwrap(),
        status: "active".to_string(),
        stored_at: "2025-01-01T00:00:00Z".to_string(),
    };

    registry.record_upload(&record).unwrap();

    // Look up
    let file = registry.get_file("abc123").unwrap().unwrap();
    assert_eq!(file.file_id, "abc123");
    assert_eq!(file.file_size, 2048);
    assert!(file.is_encrypted);

    // List
    let active = registry.list_files(Some("active")).unwrap();
    assert_eq!(active.len(), 1);

    // Update status
    registry.update_status("abc123", "expired").unwrap();
    let updated = registry.get_file("abc123").unwrap().unwrap();
    assert_eq!(updated.status, "expired");

    // Providers
    let providers = registry.get_providers_for_file("abc123").unwrap();
    assert_eq!(providers.len(), 2);
    assert_eq!(providers[0].provider_id, "p1");
    assert_eq!(providers[1].endpoint, "http://localhost:8081");
}

#[tokio::test]
async fn discovery_client_health_check_against_mock() {
    let addr = start_mock_server().await;
    let endpoint = format!("http://{}", addr);

    let client = carbide_client::CarbideClient::with_defaults().unwrap();
    let discovery =
        carbide_client::DiscoveryClient::new(client, endpoint);

    let result = discovery.health_check().await;
    assert!(result.is_ok());
}
