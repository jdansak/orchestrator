//! A builder for test configurations.
//!
//! Tests should never hand-write YAML. A builder produces a valid config by construction, so a
//! test that is checking the routing logic is not also debugging indentation.

use std::net::SocketAddr;

use orchestrator_core::config::model::{
    AffinityConfig, Cluster, Datacenter, Endpoint, Hierarchy, MatchType, Region, Route, RouteMatch,
    RouteScope, TenantConfig, TenantIdentity, VipConfig,
};

/// Builds a [`TenantConfig`].
#[derive(Debug, Clone)]
pub struct TestConfigBuilder {
    name: String,
    zone: String,
    vip: String,
    interface: String,
    regions: Vec<Region>,
    routes: Vec<Route>,
}

impl TestConfigBuilder {
    /// Starts a builder for a tenant with the given name.
    pub fn builder(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            zone: format!("{name}.internal"),
            name,
            vip: "10.0.0.100/32".to_string(),
            interface: "eth0".to_string(),
            regions: Vec::new(),
            routes: Vec::new(),
        }
    }

    /// Sets the tenant's DNS zone.
    pub fn zone(mut self, zone: impl Into<String>) -> Self {
        self.zone = zone.into();
        self
    }

    /// Sets the tenant's VIP.
    pub fn vip(mut self, vip: impl Into<String>) -> Self {
        self.vip = vip.into();
        self
    }

    /// Sets the network interface.
    pub fn interface(mut self, interface: impl Into<String>) -> Self {
        self.interface = interface.into();
        self
    }

    /// Adds a region with a single datacenter, cluster, and endpoint.
    ///
    /// The weights are divided evenly unless a weight is given.
    pub fn region(self, name: &str, weight: u32, endpoints: &[&str]) -> Self {
        let per_endpoint = if endpoints.is_empty() {
            100
        } else {
            100 / endpoints.len() as u32
        };

        let endpoints = endpoints
            .iter()
            .map(|endpoint| Endpoint {
                name: (*endpoint).to_string(),
                weight: per_endpoint,
                address: "127.0.0.1:9".to_string().parse::<SocketAddr>().unwrap(),
            })
            .collect();

        self.region_with(
            name,
            weight,
            vec![Datacenter {
                name: "dc-1".to_string(),
                weight: 100,
                clusters: vec![Cluster {
                    name: "prod".to_string(),
                    weight: 100,
                    endpoints,
                }],
            }],
        )
    }

    /// Adds a region with an explicit datacenter list.
    pub fn region_with(self, name: &str, weight: u32, datacenters: Vec<Datacenter>) -> Self {
        let mut regions = self.regions;
        regions.push(Region {
            name: name.to_string(),
            weight,
            datacenters,
        });
        Self { regions, ..self }
    }

    /// Adds a route.
    pub fn route(
        mut self,
        name: &str,
        path: &str,
        match_type: MatchType,
        affinity: AffinityConfig,
    ) -> Self {
        self.routes.push(Route {
            name: name.to_string(),
            enabled: true,
            r#match: RouteMatch {
                path: path.to_string(),
                r#type: match_type,
            },
            scope: RouteScope {
                region: "*".to_string(),
            },
            affinity,
        });
        self
    }

    /// Builds the configuration.
    pub fn build(self) -> TenantConfig {
        TenantConfig {
            version: 1,
            tenant: TenantIdentity {
                name: self.name,
                zone: self.zone,
                description: None,
            },
            vip: VipConfig {
                address: self.vip,
                interface: self.interface,
                priority: None,
            },
            listeners: Default::default(),
            upstream: Default::default(),
            hierarchy: Hierarchy {
                regions: self.regions,
            },
            routes: self.routes,
            rollout: None,
            dns: Default::default(),
            metrics: Default::default(),
            logging: Default::default(),
            worker: Default::default(),
        }
    }

    /// Builds the configuration and serialises it to YAML.
    pub fn to_yaml(&self) -> String {
        serde_yaml_ng::to_string(&self.clone().build()).expect("the config is serialisable")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orchestrator_core::config::validate::validate;

    #[test]
    fn builds_a_valid_config() {
        let config = TestConfigBuilder::builder("app")
            .region("us-east", 100, &["ep-a"])
            .route("api", "/api", MatchType::Prefix, AffinityConfig::default())
            .build();

        assert!(validate(&config).is_empty());
    }

    #[test]
    fn splits_weights_evenly_between_endpoints() {
        let config = TestConfigBuilder::builder("app")
            .region("us-east", 100, &["ep-a", "ep-b"])
            .build();

        let endpoints = orchestrator_core::hierarchy::flatten(&config.hierarchy);
        assert_eq!(endpoints.len(), 2);
        assert!((endpoints[0].effective_weight - 0.5).abs() < 1e-9);
    }

    #[test]
    fn serialises_to_yaml() {
        let yaml = TestConfigBuilder::builder("app")
            .region("us-east", 100, &["ep-a"])
            .to_yaml();
        assert!(yaml.contains("us-east"));
        assert!(yaml.contains("ep-a"));
    }
}
