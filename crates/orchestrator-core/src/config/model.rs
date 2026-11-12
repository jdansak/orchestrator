//! The configuration model.
//!
//! This is the Rust translation of the normative schema in `plan/07-config-schema.md`. Every
//! field there must have a counterpart here, and vice versa. Unknown fields are rejected at
//! deserialization time so a typo fails loudly instead of silently doing nothing.

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A duration that is written as a human-readable string in configuration files.
///
/// `30s`, `5m`, `1h30m`. A bare integer is rejected — see
/// [`duration::parse_duration`](crate::config::duration::parse_duration).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConfigDuration(pub Duration);

impl ConfigDuration {
    /// The wrapped duration.
    pub const fn as_duration(&self) -> Duration {
        self.0
    }
}

impl std::ops::Deref for ConfigDuration {
    type Target = Duration;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Duration> for ConfigDuration {
    fn from(value: Duration) -> Self {
        Self(value)
    }
}

impl Default for ConfigDuration {
    fn default() -> Self {
        Self(Duration::ZERO)
    }
}

impl From<ConfigDuration> for Duration {
    fn from(value: ConfigDuration) -> Self {
        value.0
    }
}

impl Serialize for ConfigDuration {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        crate::config::duration::serde_ser::serialize(&self.0, serializer)
    }
}

impl<'de> Deserialize<'de> for ConfigDuration {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        crate::config::duration::serde_de::deserialize(deserializer).map(ConfigDuration)
    }
}

/// The root of a tenant configuration file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TenantConfig {
    /// The configuration schema version. Required.
    pub version: u32,

    /// Tenant identity and DNS zone.
    pub tenant: TenantIdentity,

    /// The virtual IP this tenant serves on.
    pub vip: VipConfig,

    /// The listeners this tenant opens.
    #[serde(default)]
    pub listeners: Listeners,

    /// Backend connection behaviour and health-check parameters.
    #[serde(default)]
    pub upstream: UpstreamConfig,

    /// The region → datacenter → cluster → endpoint routing hierarchy.
    pub hierarchy: Hierarchy,

    /// The routes exposed by this tenant.
    #[serde(default)]
    pub routes: Vec<Route>,

    /// The blue/green rollout definition, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rollout: Option<RolloutConfig>,

    /// DNS server configuration.
    #[serde(default)]
    pub dns: DnsConfig,

    /// Metrics exporter configuration.
    #[serde(default)]
    pub metrics: MetricsConfig,

    /// Logging configuration.
    #[serde(default)]
    pub logging: LoggingConfig,

    /// Worker identity and sync-channel settings.
    #[serde(default)]
    pub worker: WorkerConfig,
}

/// Tenant identity and the DNS zone it is authoritative for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TenantIdentity {
    /// The tenant's name. Must match the registry entry and the directory name.
    pub name: String,

    /// The DNS zone this tenant is authoritative for.
    pub zone: String,

    /// An optional human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// The virtual IP configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VipConfig {
    /// The VIP in CIDR notation. Must be a host address (`/32` or `/128`).
    pub address: String,

    /// The interface the VIP is added to.
    pub interface: String,

    /// An advisory ordering among multiple VIPs on the same host. This is **not** an ARP
    /// priority — see `plan/07-config-schema.md` §7.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<u8>,
}

/// The listeners a tenant opens.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Listeners {
    /// The plaintext HTTP listener.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http: Option<HttpListener>,

    /// The TLS listener.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub https: Option<HttpsListener>,

    /// Raw TCP passthrough listeners.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tcp: Vec<L4Listener>,

    /// Raw UDP passthrough listeners.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub udp: Vec<L4Listener>,
}

/// A plaintext HTTP listener.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HttpListener {
    /// The address to bind.
    pub bind: SocketAddr,
}

/// A TLS listener.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HttpsListener {
    /// The address to bind.
    pub bind: SocketAddr,

    /// The TLS material.
    pub tls: TlsConfig,
}

/// A raw L4 passthrough listener.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct L4Listener {
    /// A name for the listener, unique within the tenant.
    pub name: String,

    /// The address to bind.
    pub bind: SocketAddr,

    /// The upstream target to forward to.
    pub target: SocketAddr,
}

/// TLS material and behaviour.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TlsConfig {
    /// Path to the PEM certificate chain.
    pub cert: PathBuf,

    /// Path to the PEM private key.
    pub key: PathBuf,

    /// Path to the PEM CA bundle used to verify client certificates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ca: Option<PathBuf>,

    /// Whether client certificates are requested, optional, or required.
    #[serde(default)]
    pub client_auth: ClientAuth,

    /// The minimum TLS version. Values below 1.2 are rejected.
    #[serde(default = "default_tls_min_version")]
    pub min_version: TlsVersion,

    /// The ALPN protocols offered.
    #[serde(default = "default_alpn")]
    pub alpn: Vec<String>,
}

/// The minimum TLS version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TlsVersion {
    /// TLS 1.2.
    #[serde(rename = "1.2")]
    V1_2,
    /// TLS 1.3.
    #[serde(rename = "1.3")]
    V1_3,
}

impl TlsVersion {
    /// Returns the protocol version as a `rustls`-style numeric version.
    pub fn as_number(self) -> u16 {
        match self {
            TlsVersion::V1_2 => 0x0303,
            TlsVersion::V1_3 => 0x0304,
        }
    }
}

/// How client certificates are handled.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClientAuth {
    /// Client certificates are not requested.
    #[default]
    None,
    /// Client certificates are requested but not required.
    Optional,
    /// A valid client certificate is required.
    Required,
}

/// Backend connection behaviour and health checks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpstreamConfig {
    /// How long to wait for a backend connection.
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout: ConfigDuration,

    /// How long to wait for the first byte of a backend response.
    #[serde(default = "default_response_timeout")]
    pub response_timeout: ConfigDuration,

    /// How long a pooled connection may sit idle.
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout: ConfigDuration,

    /// The maximum number of pooled connections per endpoint.
    #[serde(default = "default_max_connections")]
    pub max_connections_per_endpoint: u32,

    /// How many times to retry a **connection** failure.
    #[serde(default = "default_retries")]
    pub retries: u32,

    /// The health-check parameters.
    #[serde(default)]
    pub health_check: HealthCheckConfig,
}

/// Health-check parameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HealthCheckConfig {
    /// Whether to probe with HTTP or just a TCP connect.
    #[serde(default = "default_health_check_type")]
    pub r#type: HealthCheckType,

    /// The HTTP path to probe.
    #[serde(default = "default_health_check_path")]
    pub path: String,

    /// How often to probe.
    #[serde(default = "default_health_check_interval")]
    pub interval: ConfigDuration,

    /// How long a single probe may take.
    #[serde(default = "default_health_check_timeout")]
    pub timeout: ConfigDuration,

    /// Consecutive successes required before an endpoint is considered healthy.
    #[serde(default = "default_healthy_threshold")]
    pub healthy_threshold: u32,

    /// Consecutive failures required before an endpoint is considered unhealthy.
    #[serde(default = "default_unhealthy_threshold")]
    pub unhealthy_threshold: u32,
}

/// The health-check probe type.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthCheckType {
    /// Probe with an HTTP GET.
    #[default]
    Http,
    /// Probe with a TCP connect.
    Tcp,
}

/// The four-level routing hierarchy.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hierarchy {
    /// The top level.
    #[serde(default)]
    pub regions: Vec<Region>,
}

/// A region.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Region {
    /// The region's name, unique among its siblings.
    pub name: String,

    /// The region's share of its parent's traffic, 0–100.
    pub weight: u32,

    /// The datacenters in this region.
    #[serde(default)]
    pub datacenters: Vec<Datacenter>,
}

/// A datacenter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Datacenter {
    /// The datacenter's name, unique among its siblings.
    pub name: String,

    /// The datacenter's share of its parent's traffic, 0–100.
    pub weight: u32,

    /// The clusters in this datacenter.
    #[serde(default)]
    pub clusters: Vec<Cluster>,
}

/// A cluster.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cluster {
    /// The cluster's name, unique among its siblings.
    pub name: String,

    /// The cluster's share of its parent's traffic, 0–100.
    pub weight: u32,

    /// The endpoints in this cluster.
    #[serde(default)]
    pub endpoints: Vec<Endpoint>,
}

/// A backend endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Endpoint {
    /// The endpoint's name, unique among its siblings.
    pub name: String,

    /// The endpoint's share of its parent's traffic, 0–100.
    pub weight: u32,

    /// The endpoint's `host:port`.
    pub address: SocketAddr,
}

/// A route.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Route {
    /// The route's name, unique within the tenant.
    pub name: String,

    /// Whether the route is served.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// What the route matches on.
    pub r#match: RouteMatch,

    /// Which part of the hierarchy the route serves.
    #[serde(default)]
    pub scope: RouteScope,

    /// The session affinity behaviour.
    #[serde(default)]
    pub affinity: AffinityConfig,
}

/// What a route matches on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteMatch {
    /// The path to match. Must start with `/`.
    pub path: String,

    /// Whether the path is matched exactly or as a prefix.
    #[serde(default = "default_match_type")]
    pub r#type: MatchType,
}

/// How a route's path is matched.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MatchType {
    /// Match a path prefix, respecting path boundaries.
    #[default]
    Prefix,
    /// Match the path exactly.
    Exact,
}

/// Which part of the hierarchy a route serves.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteScope {
    /// The region this route serves. `"*"` means any region.
    #[serde(default = "default_any_region")]
    pub region: String,
}

/// Session affinity behaviour.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AffinityConfig {
    /// The affinity method.
    #[serde(default)]
    pub method: AffinityMethod,

    /// The cookie to read, when the method is `cookie`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cookie_name: Option<String>,

    /// The header to read, when the method is `header`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header_name: Option<String>,

    /// How long a session assignment survives.
    #[serde(default = "default_affinity_ttl")]
    pub ttl: ConfigDuration,
}

/// The session affinity method.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AffinityMethod {
    /// No affinity — pure weighted random.
    #[default]
    None,
    /// Affinity keyed on a cookie value.
    Cookie,
    /// Affinity keyed on a request header value.
    Header,
    /// Affinity keyed on the client IP address.
    ClientIp,
}

/// The blue/green rollout definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RolloutConfig {
    /// The route the rollout applies to.
    pub route: String,

    /// The rollout strategy.
    pub strategy: RolloutStrategy,

    /// The blue target.
    pub blue: RolloutTarget,

    /// The green target.
    pub green: RolloutTarget,

    /// The traffic percentages for each step. Must start at 0, end at 100, and increase strictly.
    pub steps: Vec<u32>,

    /// How long each step runs before auto-advancing.
    pub step_duration: ConfigDuration,

    /// How the rollout advances.
    #[serde(default = "default_rollout_mode")]
    pub mode: RolloutMode,

    /// The auto-rollback thresholds.
    #[serde(default)]
    pub auto_rollback: AutoRollbackConfig,

    /// The notification configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notify: Option<NotifyConfig>,
}

/// The rollout strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RolloutStrategy {
    /// Blue/green: two full targets, traffic shifted between them.
    BlueGreen,
    /// Canary: a small slice of traffic on the new target.
    Canary,
}

/// How a rollout advances.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RolloutMode {
    /// Advance automatically after `step_duration`.
    #[default]
    Gradual,
    /// Jump straight to the final step.
    Instant,
    /// Advance only when an operator asks.
    Manual,
}

/// A reference to a hierarchy node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RolloutTarget {
    /// The region.
    pub region: String,

    /// The datacenter.
    pub datacenter: String,

    /// The cluster.
    pub cluster: String,
}

/// The auto-rollback thresholds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutoRollbackConfig {
    /// Whether auto-rollback is armed.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// The error rate that triggers a rollback, as a fraction.
    #[serde(default = "default_error_rate_threshold")]
    pub error_rate_threshold: f64,

    /// The p99 latency that triggers a rollback.
    #[serde(default = "default_latency_threshold")]
    pub latency_p99_threshold: ConfigDuration,

    /// The evaluation window.
    #[serde(default = "default_rollback_window")]
    pub window: ConfigDuration,

    /// The minimum number of samples required before evaluating.
    #[serde(default = "default_min_samples")]
    pub min_samples: u64,
}

/// Where rollback and completion notifications go.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NotifyConfig {
    /// The webhook URL to POST notifications to.
    pub webhook: String,

    /// The events to notify about.
    #[serde(default)]
    pub events: Vec<NotifyEvent>,
}

/// The notifiable rollout events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotifyEvent {
    /// A rollback was triggered.
    Rollback,
    /// The rollout completed successfully.
    Complete,
    /// The rollout failed.
    Failed,
}

/// The DNS server configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DnsConfig {
    /// Whether the DNS server runs.
    #[serde(default = "default_dns_enabled")]
    pub enabled: bool,

    /// The address to listen on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub listen: Option<SocketAddr>,

    /// The upstream resolvers for non-authoritative queries.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub upstream: Vec<String>,

    /// The default TTL for generated records.
    #[serde(default = "default_dns_ttl")]
    pub ttl: u32,

    /// Explicit DNS records.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub records: Vec<DnsRecord>,
}

/// A DNS record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DnsRecord {
    /// The record name, relative to the zone.
    pub name: String,

    /// The record type.
    pub r#type: DnsRecordType,

    /// The record value. Meaning depends on the type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,

    /// The record TTL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl: Option<u32>,

    /// The SRV priority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<u32>,

    /// The SRV weight.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<u32>,

    /// The SRV port.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,

    /// The SRV target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

/// A DNS record type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DnsRecordType {
    /// An IPv4 address record.
    A,
    /// An IPv6 address record.
    Aaaa,
    /// A canonical name record.
    Cname,
    /// A service locator record.
    Srv,
}

/// The metrics exporter configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricsConfig {
    /// Whether metrics are exposed.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// The address to expose metrics on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind: Option<SocketAddr>,
}

/// The logging configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoggingConfig {
    /// The log level.
    #[serde(default = "default_log_level")]
    pub level: String,

    /// The log format.
    #[serde(default = "default_log_format")]
    pub format: String,
}

/// The worker identity and sync-channel configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerConfig {
    /// The worker mode.
    #[serde(default = "default_worker_mode")]
    pub mode: WorkerMode,

    /// The stable identifier for this worker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,

    /// The sync channel settings.
    #[serde(default)]
    pub sync: SyncConfig,
}

/// The worker mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkerMode {
    /// Only the active worker serves traffic.
    #[default]
    ActivePassive,
    /// Both workers serve traffic.
    ActiveActive,
}

/// The sync channel configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyncConfig {
    /// The address the sync server binds to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub listen: Option<SocketAddr>,

    /// The peer worker's address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub peer: Option<SocketAddr>,

    /// How often heartbeats are sent.
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval: ConfigDuration,

    /// How long to wait for a peer before declaring it dead.
    #[serde(default = "default_peer_timeout")]
    pub peer_timeout: ConfigDuration,
}

/// A flat view of the hierarchy, used by the routing table and the GUI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlatEndpoint {
    /// The `region/datacenter/cluster/endpoint` path.
    pub path: String,

    /// The endpoint's address.
    pub address: SocketAddr,

    /// The endpoint's own weight.
    pub weight: u32,

    /// The probability that a request lands on this endpoint.
    pub effective_weight: f64,
}

/// A map of endpoint paths to their state, used by the API.
pub type EndpointStates = BTreeMap<String, EndpointState>;

/// The health state of one endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndpointState {
    /// Whether the endpoint is currently considered healthy.
    pub healthy: bool,
    /// How long the endpoint has been in its current state.
    pub since: ConfigDuration,
}

fn default_true() -> bool {
    true
}

// ---------------------------------------------------------------------------
// `Default` implementations
//
// These mirror the serde defaults above. They exist so the model can be built
// programmatically (in tests, and by the API) without going through YAML.
// ---------------------------------------------------------------------------

impl Default for UpstreamConfig {
    fn default() -> Self {
        Self {
            connect_timeout: default_connect_timeout(),
            response_timeout: default_response_timeout(),
            idle_timeout: default_idle_timeout(),
            max_connections_per_endpoint: default_max_connections(),
            retries: default_retries(),
            health_check: HealthCheckConfig::default(),
        }
    }
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            r#type: default_health_check_type(),
            path: default_health_check_path(),
            interval: default_health_check_interval(),
            timeout: default_health_check_timeout(),
            healthy_threshold: default_healthy_threshold(),
            unhealthy_threshold: default_unhealthy_threshold(),
        }
    }
}

impl Default for AffinityConfig {
    fn default() -> Self {
        Self {
            method: AffinityMethod::None,
            cookie_name: None,
            header_name: None,
            ttl: default_affinity_ttl(),
        }
    }
}

impl Default for DnsConfig {
    fn default() -> Self {
        Self {
            enabled: default_dns_enabled(),
            listen: None,
            upstream: Vec::new(),
            ttl: default_dns_ttl(),
            records: Vec::new(),
        }
    }
}

impl Default for RouteScope {
    fn default() -> Self {
        Self {
            region: default_any_region(),
        }
    }
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            bind: None,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
        }
    }
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            mode: default_worker_mode(),
            node_id: None,
            sync: SyncConfig::default(),
        }
    }
}

impl Default for AutoRollbackConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            error_rate_threshold: default_error_rate_threshold(),
            latency_p99_threshold: default_latency_threshold(),
            window: default_rollback_window(),
            min_samples: default_min_samples(),
        }
    }
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            listen: None,
            peer: None,
            heartbeat_interval: default_heartbeat_interval(),
            peer_timeout: default_peer_timeout(),
        }
    }
}

fn default_any_region() -> String {
    "*".to_string()
}

fn default_match_type() -> MatchType {
    MatchType::Prefix
}

fn default_affinity_ttl() -> ConfigDuration {
    ConfigDuration(Duration::from_mins(30))
}

fn default_tls_min_version() -> TlsVersion {
    TlsVersion::V1_2
}

fn default_alpn() -> Vec<String> {
    vec!["h2".to_string(), "http/1.1".to_string()]
}

fn default_connect_timeout() -> ConfigDuration {
    ConfigDuration(Duration::from_secs(2))
}

fn default_response_timeout() -> ConfigDuration {
    ConfigDuration(Duration::from_secs(30))
}

fn default_idle_timeout() -> ConfigDuration {
    ConfigDuration(Duration::from_secs(60))
}

fn default_max_connections() -> u32 {
    1024
}

fn default_retries() -> u32 {
    1
}

fn default_health_check_type() -> HealthCheckType {
    HealthCheckType::Http
}

fn default_health_check_path() -> String {
    "/healthz".to_string()
}

fn default_health_check_interval() -> ConfigDuration {
    ConfigDuration(Duration::from_secs(10))
}

fn default_health_check_timeout() -> ConfigDuration {
    ConfigDuration(Duration::from_secs(2))
}

fn default_healthy_threshold() -> u32 {
    2
}

fn default_unhealthy_threshold() -> u32 {
    3
}

fn default_rollout_mode() -> RolloutMode {
    RolloutMode::Gradual
}

fn default_error_rate_threshold() -> f64 {
    0.05
}

fn default_latency_threshold() -> ConfigDuration {
    ConfigDuration(Duration::from_secs(2))
}

fn default_rollback_window() -> ConfigDuration {
    ConfigDuration(Duration::from_secs(60))
}

fn default_min_samples() -> u64 {
    100
}

fn default_dns_ttl() -> u32 {
    60
}

/// The DNS server is opt-in. An operator enables it explicitly with `dns.enabled: true` and
/// supplies at least one upstream resolver.
fn default_dns_enabled() -> bool {
    false
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_format() -> String {
    "json".to_string()
}

fn default_worker_mode() -> WorkerMode {
    WorkerMode::ActivePassive
}

fn default_heartbeat_interval() -> ConfigDuration {
    ConfigDuration(Duration::from_millis(300))
}

fn default_peer_timeout() -> ConfigDuration {
    ConfigDuration(Duration::from_secs(1))
}
