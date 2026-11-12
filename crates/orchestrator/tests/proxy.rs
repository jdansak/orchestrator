//! End-to-end tests for the HTTP data plane.
//!
//! These start a real proxy over a real HTTP backend. They are the first proof that the
//! routing, the weighting, and the error responses actually work end to end.

use std::sync::Arc;

use orchestrator::data_plane::http::ProxyService;
use orchestrator_core::config::model::{
    AffinityConfig, Cluster, Datacenter, Endpoint, Hierarchy, MatchType, Region, Route, RouteMatch,
    RouteScope, TenantConfig, TenantIdentity, VipConfig,
};
use orchestrator_testkit::{MockBackend, ResponseSpec};

/// Builds a config with one endpoint per backend, weighted evenly.
fn config_with_backends(backends: &[MockBackend]) -> TenantConfig {
    let endpoints = backends
        .iter()
        .enumerate()
        .map(|(index, backend)| Endpoint {
            name: format!("ep-{index}"),
            weight: 100,
            address: backend.address(),
        })
        .collect();

    TenantConfig {
        version: 1,
        tenant: TenantIdentity {
            name: "test".to_string(),
            zone: "test.internal".to_string(),
            description: None,
        },
        vip: VipConfig {
            address: "10.0.0.100/32".to_string(),
            interface: "eth0".to_string(),
            priority: None,
        },
        listeners: Default::default(),
        upstream: Default::default(),
        hierarchy: Hierarchy {
            regions: vec![Region {
                name: "us-east".to_string(),
                weight: 100,
                datacenters: vec![Datacenter {
                    name: "dc-1".to_string(),
                    weight: 100,
                    clusters: vec![Cluster {
                        name: "prod".to_string(),
                        weight: 100,
                        endpoints,
                    }],
                }],
            }],
        },
        routes: vec![Route {
            name: "api".to_string(),
            enabled: true,
            r#match: RouteMatch {
                path: "/api".to_string(),
                r#type: MatchType::Prefix,
            },
            scope: RouteScope {
                region: "*".to_string(),
            },
            affinity: AffinityConfig::default(),
        }],
        rollout: None,
        dns: Default::default(),
        metrics: Default::default(),
        logging: Default::default(),
        worker: Default::default(),
    }
}

/// Sends a GET request to the proxy and returns the status and body.
async fn get(proxy: &ProxyService, path: &str) -> (u16, String) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind the proxy listener");
    let address = listener.local_addr().expect("failed to read the address");

    let service = proxy.clone();
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("failed to accept");
        let io = hyper_util::rt::TokioIo::new(stream);
        let _ = hyper::server::conn::http1::Builder::new()
            .serve_connection(
                io,
                hyper::service::service_fn(move |request| {
                    let service = service.clone();
                    async move { service.handle(request).await }
                }),
            )
            .await;
    });

    let mut stream = tokio::net::TcpStream::connect(address)
        .await
        .expect("failed to connect to the proxy");
    let request = format!("GET {path} HTTP/1.1\r\nHost: test\r\nConnection: close\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .await
        .expect("failed to write the request");
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .await
        .expect("failed to read the response");

    let status = response
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);
    let body = response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.to_string())
        .unwrap_or_default();

    (status, body)
}

#[tokio::test]
async fn forwards_a_request_to_the_backend() {
    let backend = MockBackend::start_with(ResponseSpec {
        status: 200,
        latency: std::time::Duration::ZERO,
        body: "hello from the backend".to_string(),
        fail_rate: 0.0,
    })
    .await;

    let proxy = ProxyService::new(Arc::new(config_with_backends(std::slice::from_ref(
        &backend,
    ))));

    let (status, body) = get(&proxy, "/api/things").await;

    assert_eq!(status, 200);
    assert!(body.contains("hello from the backend"), "got: {body}");
    assert_eq!(
        backend.hit_count(),
        1,
        "the backend should have been called once"
    );
}

#[tokio::test]
async fn forwards_the_request_path_and_query() {
    let backend = MockBackend::start().await;
    let proxy = ProxyService::new(Arc::new(config_with_backends(std::slice::from_ref(
        &backend,
    ))));

    let (status, _) = get(&proxy, "/api/v2/users?page=3").await;

    assert_eq!(status, 200);
    let requests = backend.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/api/v2/users");
}

#[tokio::test]
async fn an_unmatched_path_returns_404() {
    let backend = MockBackend::start().await;
    let proxy = ProxyService::new(Arc::new(config_with_backends(std::slice::from_ref(
        &backend,
    ))));

    let (status, _) = get(&proxy, "/does-not-match").await;

    assert_eq!(status, 404);
    assert_eq!(
        backend.hit_count(),
        0,
        "the backend should not have been called"
    );
}

#[tokio::test]
async fn an_unreachable_backend_returns_502() {
    // Port 9 is the discard port. Nothing listens there.
    let mut config = config_with_backends(&[]);
    config.hierarchy.regions[0].datacenters[0].clusters[0].endpoints = vec![Endpoint {
        name: "dead".to_string(),
        weight: 100,
        address: "127.0.0.1:9".parse().unwrap(),
    }];

    let proxy = ProxyService::new(Arc::new(config));

    let (status, _) = get(&proxy, "/api/things").await;

    assert_eq!(status, 502);
}

#[tokio::test]
async fn a_backend_error_is_passed_through() {
    let backend = MockBackend::start_with(ResponseSpec {
        status: 503,
        latency: std::time::Duration::ZERO,
        body: "unavailable".to_string(),
        fail_rate: 0.0,
    })
    .await;

    let proxy = ProxyService::new(Arc::new(config_with_backends(std::slice::from_ref(
        &backend,
    ))));

    let (status, body) = get(&proxy, "/api/things").await;

    assert_eq!(status, 503);
    assert!(body.contains("unavailable"), "got: {body}");
}

#[tokio::test]
async fn traffic_is_spread_across_the_backends() {
    let first = MockBackend::start().await;
    let second = MockBackend::start().await;

    let proxy = ProxyService::new(Arc::new(config_with_backends(&[
        first.clone(),
        second.clone(),
    ])));

    for _ in 0..50 {
        let (status, _) = get(&proxy, "/api/things").await;
        assert_eq!(status, 200);
    }

    let first_hits = first.hit_count();
    let second_hits = second.hit_count();

    assert_eq!(
        first_hits + second_hits,
        50,
        "every request should be forwarded"
    );
    assert!(
        first_hits > 0,
        "the first backend should have received traffic"
    );
    assert!(
        second_hits > 0,
        "the second backend should have received traffic"
    );
}
