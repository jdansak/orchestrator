//! Configuration validation.
//!
//! Implements the rules in `plan/07-config-schema.md` §9. The validator is **total**: it
//! collects every problem rather than stopping at the first, so an operator fixing a config sees
//! the full list in one pass.

use std::net::IpAddr;
use std::str::FromStr;

use crate::config::model::{AffinityMethod, DnsRecordType, TenantConfig, TlsVersion};
use crate::error::ValidationErrors;
use crate::hierarchy::validate_weights;
use crate::version::MAX_SCHEMA_VERSION;

/// The documented limits from `plan/11-non-functional-requirements.md` §3.
pub const MAX_ROUTES: usize = 1_000;
/// The maximum number of endpoints in a hierarchy.
pub const MAX_ENDPOINTS: usize = 10_000;
/// The maximum number of DNS records in a zone.
pub const MAX_DNS_RECORDS: usize = 10_000;

/// Validates a configuration, returning every problem found.
///
/// An empty result means the configuration is valid.
pub fn validate(config: &TenantConfig) -> ValidationErrors {
    let mut errors = ValidationErrors::new();

    validate_version(config, &mut errors);
    validate_tenant(config, &mut errors);
    validate_vip(config, &mut errors);
    validate_listeners(config, &mut errors);
    validate_hierarchy(config, &mut errors);
    validate_routes(config, &mut errors);
    validate_rollout(config, &mut errors);
    validate_dns(config, &mut errors);
    validate_limits(config, &mut errors);

    errors
}

/// V1 — the schema version must be supported.
fn validate_version(config: &TenantConfig, errors: &mut ValidationErrors) {
    if config.version > MAX_SCHEMA_VERSION {
        errors.push(
            "version",
            format!(
                "schema version {} is newer than the supported version {MAX_SCHEMA_VERSION}",
                config.version
            ),
        );
    }
}

/// V16 — route names must be unique.
fn validate_tenant(config: &TenantConfig, errors: &mut ValidationErrors) {
    if config.tenant.name.trim().is_empty() {
        errors.push("tenant.name", "must not be empty");
    }
    if config.tenant.zone.trim().is_empty() {
        errors.push("tenant.zone", "must not be empty");
    } else if !config.tenant.zone.contains('.') {
        errors.push(
            "tenant.zone",
            format!(
                "'{}' is not a valid DNS zone name — it must contain at least one dot",
                config.tenant.zone
            ),
        );
    }
}

/// V13 — the VIP must be a host address on a known interface name.
fn validate_vip(config: &TenantConfig, errors: &mut ValidationErrors) {
    if config.vip.interface.trim().is_empty() {
        errors.push("vip.interface", "must not be empty");
    }

    let Some((address, prefix)) = config.vip.address.split_once('/') else {
        errors.push(
            "vip.address",
            format!(
                "'{}' must be in CIDR notation, e.g. 10.0.0.100/32",
                config.vip.address
            ),
        );
        return;
    };

    let Ok(ip) = IpAddr::from_str(address.trim()) else {
        errors.push(
            "vip.address",
            format!("'{address}' is not a valid IP address"),
        );
        return;
    };

    let Ok(prefix) = prefix.parse::<u8>() else {
        errors.push(
            "vip.address",
            format!("'{prefix}' is not a valid prefix length"),
        );
        return;
    };

    let host_bits = if ip.is_ipv4() { 32 } else { 128 };
    if prefix != host_bits {
        errors.push(
            "vip.address",
            format!(
                "the VIP must be a host address — expected a /{host_bits} prefix, got /{prefix}"
            ),
        );
    }
}

/// V11, V12 — TLS material and version.
fn validate_listeners(config: &TenantConfig, errors: &mut ValidationErrors) {
    let Some(https) = &config.listeners.https else {
        return;
    };

    let tls = &https.tls;

    if tls.min_version != TlsVersion::V1_2 && tls.min_version != TlsVersion::V1_3 {
        errors.push(
            "listeners.https.tls.min_version",
            "the minimum TLS version must be 1.2 or 1.3 — TLS 1.0 and 1.1 are not negotiable",
        );
    }

    if tls.cert.as_os_str().is_empty() {
        errors.push("listeners.https.tls.cert", "must not be empty");
    }
    if tls.key.as_os_str().is_empty() {
        errors.push("listeners.https.tls.key", "must not be empty");
    }

    if tls.client_auth != crate::config::model::ClientAuth::None && tls.ca.is_none() {
        errors.push(
            "listeners.https.tls.ca",
            "a CA bundle is required when client_auth is 'optional' or 'required'",
        );
    }

    if tls.alpn.is_empty() {
        errors.push(
            "listeners.https.tls.alpn",
            "at least one ALPN protocol must be offered",
        );
    }
}

/// V3, V4, V5, V6 — hierarchy weights, uniqueness, and non-empty levels.
fn validate_hierarchy(config: &TenantConfig, errors: &mut ValidationErrors) {
    validate_weights(&config.hierarchy, errors);

    if config.hierarchy.regions.is_empty() {
        errors.push("hierarchy.regions", "at least one region is required");
    }

    for (index, region) in config.hierarchy.regions.iter().enumerate() {
        if region.datacenters.is_empty() {
            errors.push(
                format!("hierarchy.regions[{index}].datacenters"),
                "at least one datacenter is required",
            );
        }
        for (dc_index, datacenter) in region.datacenters.iter().enumerate() {
            if datacenter.clusters.is_empty() {
                errors.push(
                    format!("hierarchy.regions[{index}].datacenters[{dc_index}].clusters"),
                    "at least one cluster is required",
                );
            }
            for (cluster_index, cluster) in datacenter.clusters.iter().enumerate() {
                if cluster.endpoints.is_empty() {
                    errors.push(
                        format!(
                            "hierarchy.regions[{index}].datacenters[{dc_index}].clusters[{cluster_index}].endpoints"
                        ),
                        "at least one endpoint is required",
                    );
                }
            }
        }
    }
}

/// V7, V16, V17 — route references, uniqueness, and path syntax.
fn validate_routes(config: &TenantConfig, errors: &mut ValidationErrors) {
    let mut seen = Vec::new();

    for (index, route) in config.routes.iter().enumerate() {
        let path = format!("routes[{index}]");

        if seen.contains(&route.name) {
            errors.push(
                format!("{path}.name"),
                format!("duplicate route name '{}'", route.name),
            );
        }
        seen.push(route.name.clone());

        if !route.r#match.path.starts_with('/') {
            errors.push(
                format!("{path}.match.path"),
                format!("path '{}' must start with '/'", route.r#match.path),
            );
        }

        // V7 — the scope must reference a region that exists.
        if route.scope.region != "*"
            && !config
                .hierarchy
                .regions
                .iter()
                .any(|r| r.name == route.scope.region)
        {
            errors.push(
                format!("{path}.scope.region"),
                format!("unknown region '{}'", route.scope.region),
            );
        }

        // V10 — the affinity method must have the field it needs.
        match route.affinity.method {
            AffinityMethod::Cookie => {
                if route
                    .affinity
                    .cookie_name
                    .as_deref()
                    .is_none_or(str::is_empty)
                {
                    errors.push(
                        format!("{path}.affinity.cookie_name"),
                        "is required when the affinity method is 'cookie'",
                    );
                }
            }
            AffinityMethod::Header => {
                if route
                    .affinity
                    .header_name
                    .as_deref()
                    .is_none_or(str::is_empty)
                {
                    errors.push(
                        format!("{path}.affinity.header_name"),
                        "is required when the affinity method is 'header'",
                    );
                }
            }
            AffinityMethod::None | AffinityMethod::ClientIp => {}
        }

        if route.affinity.ttl.as_duration().is_zero() {
            errors.push(format!("{path}.affinity.ttl"), "must be greater than zero");
        }
    }
}

/// V8, V9 — rollout targets and steps.
fn validate_rollout(config: &TenantConfig, errors: &mut ValidationErrors) {
    let Some(rollout) = &config.rollout else {
        return;
    };

    // V8 — the route must exist.
    if !config.routes.iter().any(|r| r.name == rollout.route) {
        errors.push(
            "rollout.route",
            format!("unknown route '{}'", rollout.route),
        );
    }

    // V8 — the blue and green targets must exist.
    for (label, target) in [("blue", &rollout.blue), ("green", &rollout.green)] {
        let path = format!("rollout.{label}");
        let Some(region) = config
            .hierarchy
            .regions
            .iter()
            .find(|r| r.name == target.region)
        else {
            errors.push(
                format!("{path}.region"),
                format!("unknown region '{}'", target.region),
            );
            continue;
        };

        let Some(datacenter) = region
            .datacenters
            .iter()
            .find(|d| d.name == target.datacenter)
        else {
            errors.push(
                format!("{path}.datacenter"),
                format!(
                    "unknown datacenter '{}' in region '{}'",
                    target.datacenter, target.region
                ),
            );
            continue;
        };

        if !datacenter.clusters.iter().any(|c| c.name == target.cluster) {
            errors.push(
                format!("{path}.cluster"),
                format!(
                    "unknown cluster '{}' in datacenter '{}'",
                    target.cluster, target.datacenter
                ),
            );
        }
    }

    // V9 — the steps must start at 0, end at 100, and increase strictly.
    if rollout.steps.is_empty() {
        errors.push("rollout.steps", "at least one step is required");
    } else {
        if rollout.steps[0] != 0 {
            errors.push(
                "rollout.steps",
                format!("must start at 0, got {}", rollout.steps[0]),
            );
        }
        if let Some(last) = rollout.steps.last() {
            if *last != 100 {
                errors.push("rollout.steps", format!("must end at 100, got {last}"));
            }
        }
        for pair in rollout.steps.windows(2) {
            if pair[1] <= pair[0] {
                errors.push(
                    "rollout.steps",
                    format!(
                        "must increase strictly — {} is not greater than {}",
                        pair[1], pair[0]
                    ),
                );
                break;
            }
        }
    }

    if rollout.auto_rollback.error_rate_threshold < 0.0
        || rollout.auto_rollback.error_rate_threshold > 1.0
    {
        errors.push(
            "rollout.auto_rollback.error_rate_threshold",
            "must be a fraction between 0.0 and 1.0",
        );
    }
}

/// V15, and record-specific checks.
fn validate_dns(config: &TenantConfig, errors: &mut ValidationErrors) {
    if !config.dns.enabled {
        return;
    }

    if config.dns.upstream.is_empty() {
        errors.push(
            "dns.upstream",
            "at least one upstream resolver is required when DNS is enabled",
        );
    }

    for (index, record) in config.dns.records.iter().enumerate() {
        let path = format!("dns.records[{index}]");

        match record.r#type {
            DnsRecordType::A => {
                let Some(value) = &record.value else {
                    errors.push(format!("{path}.value"), "is required for an A record");
                    continue;
                };
                if IpAddr::from_str(value).is_err() {
                    errors.push(
                        format!("{path}.value"),
                        format!("'{value}' is not a valid IP address"),
                    );
                }
            }
            DnsRecordType::Aaaa => {
                let Some(value) = &record.value else {
                    errors.push(format!("{path}.value"), "is required for an AAAA record");
                    continue;
                };
                match IpAddr::from_str(value) {
                    Ok(IpAddr::V6(_)) => {}
                    Ok(IpAddr::V4(_)) => errors.push(
                        format!("{path}.value"),
                        format!("'{value}' is not an IPv6 address"),
                    ),
                    Err(_) => errors.push(
                        format!("{path}.value"),
                        format!("'{value}' is not a valid IP address"),
                    ),
                }
            }
            DnsRecordType::Cname => {
                let Some(value) = &record.value else {
                    errors.push(format!("{path}.value"), "is required for a CNAME record");
                    continue;
                };
                if value.is_empty() {
                    errors.push(format!("{path}.value"), "must not be empty");
                }
            }
            DnsRecordType::Srv => {
                if record.target.is_none() {
                    errors.push(format!("{path}.target"), "is required for an SRV record");
                }
                if record.port.is_none() {
                    errors.push(format!("{path}.port"), "is required for an SRV record");
                }
            }
        }
    }
}

/// The documented scalability limits.
fn validate_limits(config: &TenantConfig, errors: &mut ValidationErrors) {
    if config.routes.len() > MAX_ROUTES {
        errors.push(
            "routes",
            format!(
                "{} routes exceeds the limit of {MAX_ROUTES}",
                config.routes.len()
            ),
        );
    }

    let endpoints = crate::hierarchy::flatten(&config.hierarchy).len();
    if endpoints > MAX_ENDPOINTS {
        errors.push(
            "hierarchy",
            format!("{endpoints} endpoints exceeds the limit of {MAX_ENDPOINTS}"),
        );
    }

    if config.dns.records.len() > MAX_DNS_RECORDS {
        errors.push(
            "dns.records",
            format!(
                "{} records exceeds the limit of {MAX_DNS_RECORDS}",
                config.dns.records.len()
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::{
        AffinityConfig, Cluster, Datacenter, Endpoint, HealthCheckConfig, Region, Route,
        RouteMatch, RouteScope, UpstreamConfig, VipConfig,
    };
    use std::net::SocketAddr;

    fn endpoint(name: &str, weight: u32) -> Endpoint {
        Endpoint {
            name: name.to_string(),
            weight,
            address: "10.1.1.5:8080".parse::<SocketAddr>().unwrap(),
        }
    }

    fn config() -> TenantConfig {
        TenantConfig {
            version: 1,
            tenant: crate::config::model::TenantIdentity {
                name: "app-frontend".to_string(),
                zone: "app-frontend.internal".to_string(),
                description: None,
            },
            vip: VipConfig {
                address: "10.0.0.100/32".to_string(),
                interface: "eth0".to_string(),
                priority: None,
            },
            listeners: Default::default(),
            upstream: UpstreamConfig {
                health_check: HealthCheckConfig::default(),
                ..Default::default()
            },
            hierarchy: crate::config::model::Hierarchy {
                regions: vec![Region {
                    name: "us-east".to_string(),
                    weight: 100,
                    datacenters: vec![Datacenter {
                        name: "dc-1".to_string(),
                        weight: 100,
                        clusters: vec![Cluster {
                            name: "prod".to_string(),
                            weight: 100,
                            endpoints: vec![endpoint("ep-a", 100)],
                        }],
                    }],
                }],
            },
            routes: vec![Route {
                name: "api".to_string(),
                enabled: true,
                r#match: RouteMatch {
                    path: "/api".to_string(),
                    r#type: crate::config::model::MatchType::Prefix,
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
    fn a_valid_config_has_no_errors() {
        let errors = validate(&config());
        assert!(errors.is_empty(), "unexpected errors: {errors}");
    }

    #[test]
    fn weights_must_sum_to_one_hundred() {
        let mut cfg = config();
        cfg.hierarchy.regions[0].datacenters[0].clusters[0].endpoints =
            vec![endpoint("ep-a", 60), endpoint("ep-b", 30)];
        let errors = validate(&cfg);
        assert!(!errors.is_empty(), "expected a weight-sum error");
        assert!(errors.to_string().contains("sum to 90"));
    }

    #[test]
    fn the_vip_must_be_a_host_address() {
        let mut cfg = config();
        cfg.vip.address = "10.0.0.100/24".to_string();
        let errors = validate(&cfg);
        assert!(errors.to_string().contains("host address"));
    }

    #[test]
    fn the_vip_must_be_a_valid_cidr() {
        let mut cfg = config();
        cfg.vip.address = "not-an-ip".to_string();
        let errors = validate(&cfg);
        assert!(errors.to_string().contains("CIDR"));
    }

    #[test]
    fn a_route_path_must_start_with_a_slash() {
        let mut cfg = config();
        cfg.routes[0].r#match.path = "api".to_string();
        let errors = validate(&cfg);
        assert!(errors.to_string().contains("must start with '/'"));
    }

    #[test]
    fn duplicate_route_names_are_rejected() {
        let mut cfg = config();
        cfg.routes.push(cfg.routes[0].clone());
        let errors = validate(&cfg);
        assert!(errors.to_string().contains("duplicate route name"));
    }

    #[test]
    fn an_unknown_region_scope_is_rejected() {
        let mut cfg = config();
        cfg.routes[0].scope.region = "mars".to_string();
        let errors = validate(&cfg);
        assert!(errors.to_string().contains("unknown region 'mars'"));
    }

    #[test]
    fn a_cookie_affinity_requires_a_cookie_name() {
        let mut cfg = config();
        cfg.routes[0].affinity.method = AffinityMethod::Cookie;
        let errors = validate(&cfg);
        assert!(errors.to_string().contains("cookie_name"));
    }

    #[test]
    fn dns_requires_an_upstream() {
        let mut cfg = config();
        cfg.dns.enabled = true;
        cfg.dns.upstream.clear();
        let errors = validate(&cfg);
        assert!(errors.to_string().contains("upstream resolver"));
    }

    #[test]
    fn every_problem_is_reported_at_once() {
        let mut cfg = config();
        cfg.vip.address = "bogus".to_string();
        cfg.routes[0].r#match.path = "nope".to_string();
        cfg.routes[0].scope.region = "mars".to_string();

        let errors = validate(&cfg);
        assert!(
            errors.len() >= 3,
            "expected at least 3 errors, got {errors}"
        );
    }
}
