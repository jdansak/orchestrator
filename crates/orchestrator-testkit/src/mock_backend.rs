//! A mock backend that records every request it receives.
//!
//! This is the single most important fixture in the project. Goals 2, 4, and 5 all require
//! observing *which* backend received *which* request, and no other fixture makes that possible.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::Bytes;
use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Router;

/// One recorded request.
#[derive(Debug, Clone)]
pub struct RecordedRequest {
    /// The request method.
    pub method: String,
    /// The request path.
    pub path: String,
    /// The request headers.
    pub headers: HeaderMap,
    /// The request body.
    pub body: Bytes,
    /// When the request arrived.
    pub received_at: std::time::Instant,
}

/// How the mock backend should respond.
#[derive(Debug, Clone)]
pub struct ResponseSpec {
    /// The status code to return.
    pub status: u16,
    /// How long to wait before responding.
    pub latency: Duration,
    /// The body to return.
    pub body: String,
    /// The fraction of requests that fail, 0.0–1.0.
    pub fail_rate: f64,
}

impl Default for ResponseSpec {
    fn default() -> Self {
        Self {
            status: 200,
            latency: Duration::ZERO,
            body: "ok".to_string(),
            fail_rate: 0.0,
        }
    }
}

/// The shared state of a mock backend.
#[derive(Debug, Default)]
struct BackendState {
    spec: Mutex<ResponseSpec>,
    requests: Mutex<Vec<RecordedRequest>>,
    fail_counter: AtomicU64,
    alive: AtomicBool,
}

/// A running mock backend.
#[derive(Debug, Clone)]
pub struct MockBackend {
    address: SocketAddr,
    state: Arc<BackendState>,
}

impl MockBackend {
    /// Starts a mock backend on an ephemeral port.
    pub async fn start() -> Self {
        Self::start_with(ResponseSpec::default()).await
    }

    /// Starts a mock backend with an initial response specification.
    pub async fn start_with(spec: ResponseSpec) -> Self {
        let state = Arc::new(BackendState {
            spec: Mutex::new(spec),
            requests: Mutex::new(Vec::new()),
            fail_counter: AtomicU64::new(0),
            alive: AtomicBool::new(true),
        });

        let app = Router::new()
            .fallback(axum::routing::any(handle))
            .with_state(state.clone());

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("failed to bind the mock backend");
        let address = listener
            .local_addr()
            .expect("failed to read the mock backend address");

        tokio::spawn(async move {
            let _ = axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await;
        });

        Self { address, state }
    }

    /// The backend's address.
    pub fn address(&self) -> SocketAddr {
        self.address
    }

    /// The backend's `host:port` string.
    pub fn authority(&self) -> String {
        self.address.to_string()
    }

    /// Replaces the response specification.
    pub fn set_response(&self, spec: ResponseSpec) {
        if let Ok(mut current) = self.state.spec.lock() {
            *current = spec;
        }
    }

    /// Sets the fraction of requests that fail.
    pub fn set_fail_rate(&self, fail_rate: f64) {
        if let Ok(mut spec) = self.state.spec.lock() {
            spec.fail_rate = fail_rate;
        }
    }

    /// Sets the response latency.
    pub fn set_latency(&self, latency: Duration) {
        if let Ok(mut spec) = self.state.spec.lock() {
            spec.latency = latency;
        }
    }

    /// Stops responding, simulating a backend that has died.
    pub fn kill(&self) {
        self.state.alive.store(false, Ordering::SeqCst);
    }

    /// Brings the backend back.
    pub fn revive(&self) {
        self.state.alive.store(true, Ordering::SeqCst);
    }

    /// Every request the backend has received.
    pub fn requests(&self) -> Vec<RecordedRequest> {
        self.state
            .requests
            .lock()
            .map(|requests| requests.clone())
            .unwrap_or_default()
    }

    /// How many requests the backend has received.
    pub fn hit_count(&self) -> usize {
        self.state.requests.lock().map_or(0, |r| r.len())
    }

    /// Forgets every recorded request.
    pub fn reset(&self) {
        if let Ok(mut requests) = self.state.requests.lock() {
            requests.clear();
        }
    }
}

/// The request handler.
async fn handle(
    State(state): State<Arc<BackendState>>,
    ConnectInfo(_peer): ConnectInfo<SocketAddr>,
    method: axum::http::Method,
    uri: axum::http::Uri,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let spec = state
        .spec
        .lock()
        .map(|spec| spec.clone())
        .unwrap_or_default();

    if let Ok(mut requests) = state.requests.lock() {
        requests.push(RecordedRequest {
            method: method.to_string(),
            path: uri.path().to_string(),
            headers,
            body,
            received_at: std::time::Instant::now(),
        });
    }

    if !state.alive.load(Ordering::SeqCst) {
        return (StatusCode::SERVICE_UNAVAILABLE, "backend is down").into_response();
    }

    if spec.fail_rate > 0.0 {
        let counter = state.fail_counter.fetch_add(1, Ordering::Relaxed);
        let draw: f64 = (counter % 1000) as f64 / 1000.0;
        if draw < spec.fail_rate {
            return (StatusCode::INTERNAL_SERVER_ERROR, "injected failure").into_response();
        }
    }

    if !spec.latency.is_zero() {
        tokio::time::sleep(spec.latency).await;
    }

    (
        StatusCode::from_u16(spec.status).unwrap_or(StatusCode::OK),
        spec.body,
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn records_requests() {
        let backend = MockBackend::start().await;
        let response = reqwest_get(&backend.authority()).await;
        assert_eq!(response, 200);
        assert_eq!(backend.hit_count(), 1);
    }

    /// A minimal HTTP GET — avoids pulling in a full HTTP client for a smoke test.
    async fn reqwest_get(authority: &str) -> u16 {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let mut stream = tokio::net::TcpStream::connect(authority)
            .await
            .expect("failed to connect");
        let request = format!("GET / HTTP/1.1\r\nHost: {authority}\r\nConnection: close\r\n\r\n");
        stream
            .write_all(request.as_bytes())
            .await
            .expect("failed to write the request");
        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .await
            .expect("failed to read the response");
        response
            .split_whitespace()
            .nth(1)
            .and_then(|code| code.parse().ok())
            .unwrap_or(0)
    }
}
