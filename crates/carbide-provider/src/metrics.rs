//! Prometheus metrics for the storage provider.
//!
//! Exposes counters, histograms and gauges that give visibility into
//! request throughput, latency, and storage utilization.

use axum::{http::StatusCode, response::IntoResponse};
use lazy_static::lazy_static;
use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounterVec, IntGauge, Opts, Registry, TextEncoder,
};

lazy_static! {
    /// Shared Prometheus registry for all provider metrics.
    pub static ref REGISTRY: Registry = Registry::new();

    /// Total HTTP requests, labeled by method, path, and status code.
    pub static ref HTTP_REQUESTS_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new("http_requests_total", "Total HTTP requests"),
        &["method", "path", "status"],
    )
    .expect("metric creation failed");

    /// HTTP request duration in seconds, labeled by method and path.
    pub static ref HTTP_REQUEST_DURATION: HistogramVec = HistogramVec::new(
        HistogramOpts::new("http_request_duration_seconds", "HTTP request duration")
            .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 5.0]),
        &["method", "path"],
    )
    .expect("metric creation failed");

    /// Current bytes of storage used.
    pub static ref STORAGE_BYTES_USED: IntGauge =
        IntGauge::new("storage_bytes_used", "Total bytes currently stored")
            .expect("metric creation failed");

    /// Current number of files stored.
    pub static ref FILES_STORED: IntGauge =
        IntGauge::new("files_stored_total", "Total number of files stored")
            .expect("metric creation failed");

    /// Current number of active contracts.
    pub static ref ACTIVE_CONTRACTS: IntGauge =
        IntGauge::new("active_contracts_total", "Number of active storage contracts")
            .expect("metric creation failed");
}

/// Register all metrics with the global registry.
/// Call once at startup.
pub fn register_metrics() {
    REGISTRY
        .register(Box::new(HTTP_REQUESTS_TOTAL.clone()))
        .ok();
    REGISTRY
        .register(Box::new(HTTP_REQUEST_DURATION.clone()))
        .ok();
    REGISTRY
        .register(Box::new(STORAGE_BYTES_USED.clone()))
        .ok();
    REGISTRY.register(Box::new(FILES_STORED.clone())).ok();
    REGISTRY
        .register(Box::new(ACTIVE_CONTRACTS.clone()))
        .ok();
}

/// Axum handler that returns metrics in Prometheus text format.
pub async fn metrics_handler() -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = Vec::new();
    match encoder.encode(&metric_families, &mut buffer) {
        Ok(()) => (
            StatusCode::OK,
            [("content-type", "text/plain; version=0.0.4")],
            buffer,
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [("content-type", "text/plain; version=0.0.4")],
            format!("Failed to encode metrics: {e}").into_bytes(),
        ),
    }
}

/// Record an HTTP request completion.
pub fn record_request(method: &str, path: &str, status: u16, duration_secs: f64) {
    HTTP_REQUESTS_TOTAL
        .with_label_values(&[method, path, &status.to_string()])
        .inc();
    HTTP_REQUEST_DURATION
        .with_label_values(&[method, path])
        .observe(duration_secs);
}

