//! The configuration model, its parser, and its validator.

pub mod duration;
pub mod model;
pub mod parse;
pub mod validate;

pub use model::{
    AffinityConfig, AffinityMethod, AutoRollbackConfig, ClientAuth, Cluster, ConfigDuration,
    DnsConfig, DnsRecord, DnsRecordType, Endpoint, FlatEndpoint, HealthCheckConfig,
    HealthCheckType, Hierarchy, L4Listener, Listeners, LoggingConfig, MatchType, MetricsConfig,
    NotifyConfig, NotifyEvent, Region, RolloutConfig, RolloutMode, RolloutStrategy, RolloutTarget,
    Route, RouteMatch, RouteScope, SyncConfig, TenantConfig, TenantIdentity, TlsConfig, TlsVersion,
    UpstreamConfig, VipConfig, WorkerConfig, WorkerMode,
};
pub use parse::{load_tenant_config, parse_tenant_config, parse_yaml};
pub use validate::validate;
