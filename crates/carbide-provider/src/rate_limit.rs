//! Per-IP token-bucket rate limiter for Axum.
//!
//! Uses DashMap for lock-free concurrent access. Each IP gets a bucket
//! that refills at `requests_per_second` tokens per second. When the
//! bucket is empty the middleware returns 429 Too Many Requests.

use std::{
    net::IpAddr,
    sync::Arc,
    time::Instant,
};

use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use dashmap::DashMap;

/// Per-client token bucket state.
struct Bucket {
    tokens: f64,
    last_refill: Instant,
}

/// Shared rate-limit state.
pub struct RateLimitState {
    max_tokens: f64,
    refill_rate: f64, // tokens per second
    clients: DashMap<IpAddr, Bucket>,
}

impl RateLimitState {
    /// Create a new rate limiter that allows `requests_per_second` per IP.
    pub fn new(requests_per_second: u32) -> Arc<Self> {
        Arc::new(Self {
            max_tokens: requests_per_second as f64,
            refill_rate: requests_per_second as f64,
            clients: DashMap::new(),
        })
    }

    /// Try to consume one token for the given IP.
    /// Returns true if the request is allowed, false if rate-limited.
    fn allow(&self, ip: IpAddr) -> bool {
        let now = Instant::now();

        let mut entry = self.clients.entry(ip).or_insert_with(|| Bucket {
            tokens: self.max_tokens,
            last_refill: now,
        });

        let bucket = entry.value_mut();
        let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        bucket.last_refill = now;

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Axum middleware that enforces the per-IP rate limit.
///
/// If the client IP cannot be determined (e.g. behind a proxy without
/// `ConnectInfo`) the request is allowed through — we prefer leniency
/// over false-positive rejections.
pub async fn rate_limit_middleware(
    state: axum::extract::State<Arc<RateLimitState>>,
    req: Request<Body>,
    next: Next,
) -> std::result::Result<Response, StatusCode> {
    // Try to extract client IP from ConnectInfo (set by Axum's into_make_service_with_connect_info)
    // If not available, fall through and allow the request.
    let ip = req
        .extensions()
        .get::<ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0.ip());

    if let Some(ip) = ip {
        if !state.allow(ip) {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
    }

    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_allows_within_limit() {
        let state = RateLimitState::new(5);
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        // First 5 requests should be allowed
        for _ in 0..5 {
            assert!(state.allow(ip));
        }
    }

    #[test]
    fn test_blocks_over_limit() {
        let state = RateLimitState::new(2);
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));

        assert!(state.allow(ip));
        assert!(state.allow(ip));
        assert!(!state.allow(ip)); // Third request within same instant → blocked
    }

    #[test]
    fn test_different_ips_independent() {
        let state = RateLimitState::new(1);
        let ip1 = IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(2, 2, 2, 2));

        assert!(state.allow(ip1));
        assert!(state.allow(ip2)); // Different IP, should be allowed
        assert!(!state.allow(ip1)); // Same IP, over limit
    }
}
