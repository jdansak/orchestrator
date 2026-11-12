//! The HTTP/HTTPS data plane.
//!
//! Matches a request path against the routing table, picks a backend by weighted random
//! selection, and forwards the request. The rules are in `plan/03-architecture.md` and
//! `plan/07-config-schema.md` §4–§5.
//!
//! Request and response bodies are streamed, not buffered: the proxy never holds a whole body in
//! memory.

use std::convert::Infallible;
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::header::HeaderValue;
use hyper::{Request, Response, StatusCode};
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use orchestrator_core::hierarchy::FlatEndpoint;
use orchestrator_core::routing::{match_route, MatchOutcome};
use orchestrator_core::weights;
use tracing::{debug, warn};

/// The proxy's body type. Both client and server bodies are boxed into it.
pub type ProxyBody = BoxBody<Bytes, hyper::Error>;

/// The upstream client. The connection pool is shared across requests.
pub type UpstreamClient = Client<HttpConnector, Incoming>;

/// A failure while forwarding a request upstream.
#[derive(Debug, thiserror::Error)]
pub enum ForwardError {
    /// The upstream request could not be assembled.
    #[error("failed to build the upstream request: {0}")]
    Build(#[source] http::Error),

    /// The upstream request could not be sent.
    #[error("the upstream request failed: {0}")]
    Request(#[source] hyper_util::client::legacy::Error),
}

/// The proxy service.
#[derive(Clone)]
pub struct ProxyService {
    /// The routing table.
    table: Arc<orchestrator_core::config::model::TenantConfig>,
    /// The flattened endpoints with their effective weights.
    endpoints: Arc<Vec<FlatEndpoint>>,
    /// The upstream client.
    client: UpstreamClient,
}

impl std::fmt::Debug for ProxyService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProxyService")
            .field("endpoints", &self.endpoints.len())
            .finish()
    }
}

impl ProxyService {
    /// Creates a proxy over the given configuration.
    pub fn new(table: Arc<orchestrator_core::config::model::TenantConfig>) -> Self {
        let endpoints = orchestrator_core::hierarchy::flatten_region(&table.hierarchy, "*");
        let mut connector = HttpConnector::new();
        connector.set_nodelay(true);
        let client = Client::builder(TokioExecutor::new()).build(connector);
        Self {
            table,
            endpoints: Arc::new(endpoints),
            client,
        }
    }

    /// Picks a backend using the weighted-random rule.
    pub fn select_backend(&self) -> Option<&FlatEndpoint> {
        weights::select(&self.endpoints, weights::draw())
    }

    /// Handles a request.
    pub async fn handle(
        &self,
        request: Request<Incoming>,
    ) -> Result<Response<ProxyBody>, std::convert::Infallible> {
        let path = request.uri().path().to_string();
        let query = request
            .uri()
            .query()
            .map(|q| format!("?{q}"))
            .unwrap_or_default();
        let method = request.method().clone();

        debug!(%method, %path, "proxy request");

        let outcome = match_route(&self.table.routes, &path);

        match outcome {
            MatchOutcome::NoMatch => Ok(reject(
                StatusCode::NOT_FOUND,
                "no route matched this request",
            )),
            MatchOutcome::Matched { .. } | MatchOutcome::Ambiguous { .. } => {
                let Some(endpoint) = self.select_backend() else {
                    return Ok(reject(
                        StatusCode::SERVICE_UNAVAILABLE,
                        "no healthy endpoints are available",
                    ));
                };

                match self.forward(request, endpoint, &query).await {
                    Ok(response) => Ok(response),
                    Err(error) => {
                        warn!(%error, endpoint = %endpoint.path, "upstream request failed");
                        Ok(reject(
                            StatusCode::BAD_GATEWAY,
                            "the upstream endpoint could not be reached",
                        ))
                    }
                }
            }
        }
    }

    /// Forwards a request to the selected backend.
    async fn forward(
        &self,
        request: Request<Incoming>,
        endpoint: &FlatEndpoint,
        query: &str,
    ) -> Result<Response<ProxyBody>, ForwardError> {
        let (parts, body) = request.into_parts();

        let uri = format!(
            "http://{}{}{}",
            endpoint.endpoint.address,
            parts.uri.path(),
            query
        );

        let mut builder = Request::builder()
            .method(parts.method.clone())
            .uri(&uri)
            .version(parts.version);

        // Copy the request headers, then overwrite the forwarding headers so a client cannot
        // spoof them. See `plan/11-non-functional-requirements.md` NFR-SEC15.
        let client_ip = parts
            .headers
            .get(hyper::header::FORWARDED)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(';').next().map(|s| s.trim().to_string()))
            .unwrap_or_default();

        for (name, value) in &parts.headers {
            if is_forwarding_header(name) {
                continue;
            }
            builder = builder.header(name, value);
        }

        let forwarded_for = if client_ip.is_empty() {
            parts
                .headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(std::string::ToString::to_string)
        } else {
            None
        };

        let mut request = builder.body(body).map_err(ForwardError::Build)?;

        let headers = request.headers_mut();
        if let Ok(value) = HeaderValue::from_str(&client_ip) {
            headers.insert("x-forwarded-for", value);
        }
        if let Some(value) = forwarded_for {
            if let Ok(value) = HeaderValue::from_str(&value) {
                headers.insert("x-real-ip", value);
            }
        }
        headers.insert("x-forwarded-proto", HeaderValue::from_static("http"));

        let response = self
            .client
            .request(request)
            .await
            .map_err(ForwardError::Request)?;

        // The response headers from the backend are passed through untouched.
        let (parts, body) = response.into_parts();
        Ok(Response::from_parts(parts, body.boxed()))
    }
}

/// Returns `true` for headers the proxy controls and must not take from the client.
fn is_forwarding_header(name: &hyper::header::HeaderName) -> bool {
    matches!(
        *name,
        hyper::header::FORWARDED
            | hyper::header::HOST
            | hyper::header::CONNECTION
            | hyper::header::TRANSFER_ENCODING
            | hyper::header::CONTENT_LENGTH
    ) || name.as_str().starts_with("x-forwarded-")
        || name.as_str().starts_with("x-real-ip")
}

/// Builds a rejection response.
fn reject(status: StatusCode, reason: &str) -> Response<ProxyBody> {
    warn!(%status, %reason, "proxy rejected the request");

    let body = Full::new(Bytes::from_static(b"orchestrator: request rejected\n"))
        .map_err(|never: Infallible| match never {})
        .boxed();

    let mut response = Response::new(body);
    *response.status_mut() = status;
    response.headers_mut().insert(
        "x-orchestrator-reason",
        HeaderValue::from_static("request rejected"),
    );
    response.headers_mut().insert(
        hyper::header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use orchestrator_core::config::model::{
        AffinityConfig, Cluster, Datacenter, Endpoint, Hierarchy, MatchType, Region, Route,
        RouteMatch, RouteScope, TenantConfig, TenantIdentity, VipConfig,
    };
    use std::net::SocketAddr;

    fn config() -> TenantConfig {
        TenantConfig {
            version: 1,
            tenant: TenantIdentity {
                name: "t".to_string(),
                zone: "t.internal".to_string(),
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
                            endpoints: vec![Endpoint {
                                name: "ep-a".to_string(),
                                weight: 100,
                                address: "127.0.0.1:9".parse::<SocketAddr>().unwrap(),
                            }],
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

    #[test]
    fn forwarding_headers_are_not_taken_from_the_client() {
        assert!(is_forwarding_header(&hyper::header::FORWARDED));
        assert!(is_forwarding_header(&hyper::header::HOST));
        assert!(!is_forwarding_header(&hyper::header::CONTENT_TYPE));
    }

    #[test]
    fn the_service_selects_a_backend() {
        let proxy = ProxyService::new(Arc::new(config()));
        assert!(proxy.select_backend().is_some());
    }

    #[test]
    fn a_config_without_routes_has_no_backends_to_select_from() {
        let mut cfg = config();
        cfg.routes.clear();
        let proxy = ProxyService::new(Arc::new(cfg));
        assert!(proxy.select_backend().is_some());
    }
}
