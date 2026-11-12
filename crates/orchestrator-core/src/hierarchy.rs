//! Hierarchy traversal and weight composition.
//!
//! The routing hierarchy is four levels deep — region → datacenter → cluster → endpoint — and
//! the weights at each level compose **multiplicatively**:
//!
//! ```text
//! P(endpoint) = (w_region / 100) × (w_dc / 100) × (w_cluster / 100) × (w_endpoint / 100)
//! ```
//!
//! See `plan/07-config-schema.md` §5.

use std::collections::HashSet;

use crate::config::model::{Cluster, Datacenter, Endpoint, Hierarchy, Region};
use crate::error::ValidationErrors;

/// A flattened view of one endpoint in the hierarchy.
#[derive(Debug, Clone, PartialEq)]
pub struct FlatEndpoint {
    /// The `region/datacenter/cluster/endpoint` path.
    pub path: String,

    /// The region this endpoint is in.
    pub region: String,

    /// The datacenter this endpoint is in.
    pub datacenter: String,

    /// The cluster this endpoint is in.
    pub cluster: String,

    /// The endpoint itself.
    pub endpoint: Endpoint,

    /// The probability that a request lands on this endpoint, before health filtering.
    pub effective_weight: f64,
}

/// Flattens the hierarchy into a list of endpoints with their effective probabilities.
///
/// The returned list preserves config order, which is the order the weighted selector uses.
///
/// A level with exactly **one** child assigns that child the whole of its parent's share. The
/// child's declared weight is advisory in that case — otherwise a single datacenter at weight 70
/// would silently drop 30% of its region's traffic on the floor.
pub fn flatten(hierarchy: &Hierarchy) -> Vec<FlatEndpoint> {
    let mut out = Vec::new();
    for region in &hierarchy.regions {
        let region_share = share(region.weight, hierarchy.regions.len());
        for datacenter in &region.datacenters {
            let datacenter_share = share(datacenter.weight, region.datacenters.len());
            for cluster in &datacenter.clusters {
                let cluster_share = share(cluster.weight, datacenter.clusters.len());
                for endpoint in &cluster.endpoints {
                    let endpoint_share = share(endpoint.weight, cluster.endpoints.len());
                    out.push(FlatEndpoint {
                        path: format!(
                            "{}/{}/{}/{}",
                            region.name, datacenter.name, cluster.name, endpoint.name
                        ),
                        region: region.name.clone(),
                        datacenter: datacenter.name.clone(),
                        cluster: cluster.name.clone(),
                        endpoint: endpoint.clone(),
                        effective_weight: region_share
                            * datacenter_share
                            * cluster_share
                            * endpoint_share,
                    });
                }
            }
        }
    }
    out
}

/// The share of a parent's traffic that a child receives.
///
/// A lone child receives everything, regardless of its declared weight.
fn share(weight: u32, siblings: usize) -> f64 {
    if siblings == 1 {
        1.0
    } else {
        f64::from(weight) / 100.0
    }
}

/// Flattens the hierarchy and keeps only the endpoints in the given region.
///
/// Weights within the selected region are re-normalised so they sum to 1.0 — see
/// `plan/07-config-schema.md` §5, "Scope Restriction".
pub fn flatten_region(hierarchy: &Hierarchy, region: &str) -> Vec<FlatEndpoint> {
    let all = flatten(hierarchy);
    let scoped: Vec<FlatEndpoint> = if region == "*" {
        all
    } else {
        all.into_iter().filter(|e| e.region == region).collect()
    };

    let total: f64 = scoped.iter().map(|e| e.effective_weight).sum();
    if total > 0.0 {
        scoped
            .into_iter()
            .map(|mut e| {
                e.effective_weight /= total;
                e
            })
            .collect()
    } else {
        scoped
    }
}

/// Finds a region by name.
pub fn find_region<'a>(hierarchy: &'a Hierarchy, name: &str) -> Option<&'a Region> {
    hierarchy.regions.iter().find(|r| r.name == name)
}

/// Finds a datacenter by name within a region.
pub fn find_datacenter<'a>(region: &'a Region, name: &str) -> Option<&'a Datacenter> {
    region.datacenters.iter().find(|d| d.name == name)
}

/// Finds a cluster by name within a datacenter.
pub fn find_cluster<'a>(datacenter: &'a Datacenter, name: &str) -> Option<&'a Cluster> {
    datacenter.clusters.iter().find(|c| c.name == name)
}

/// Validates that sibling weights sum to 100 at every level of the hierarchy.
///
/// A level with exactly one child is accepted at any weight, because a single child receives all
/// of the traffic regardless.
pub fn validate_weights(hierarchy: &Hierarchy, errors: &mut ValidationErrors) {
    for (index, region) in hierarchy.regions.iter().enumerate() {
        validate_siblings(
            &format!("hierarchy.regions[{index}]"),
            region.weight,
            region
                .datacenters
                .iter()
                .map(|d| (d.name.clone(), d.weight))
                .collect(),
            errors,
        );

        for (dc_index, datacenter) in region.datacenters.iter().enumerate() {
            let dc_path = format!("hierarchy.regions[{index}].datacenters[{dc_index}]");
            validate_siblings(
                &dc_path,
                datacenter.weight,
                datacenter
                    .clusters
                    .iter()
                    .map(|c| (c.name.clone(), c.weight))
                    .collect(),
                errors,
            );

            for (cluster_index, cluster) in datacenter.clusters.iter().enumerate() {
                let cluster_path = format!("{dc_path}.clusters[{cluster_index}]");
                validate_siblings(
                    &cluster_path,
                    cluster.weight,
                    cluster
                        .endpoints
                        .iter()
                        .map(|e| (e.name.clone(), e.weight))
                        .collect(),
                    errors,
                );
            }
        }
    }
}

/// Validates one level of the hierarchy: weights in range, sum to 100, names unique.
fn validate_siblings(
    path: &str,
    _own_weight: u32,
    children: Vec<(String, u32)>,
    errors: &mut ValidationErrors,
) {
    let mut seen = HashSet::new();
    let mut sum: u64 = 0;

    for (index, (name, weight)) in children.iter().enumerate() {
        let child_path = format!("{path}[{index}]");

        if !seen.insert(name.clone()) {
            errors.push(
                format!("{child_path}.name"),
                format!("duplicate name '{name}' among siblings"),
            );
        }

        if *weight > 100 {
            errors.push(
                format!("{child_path}.weight"),
                format!("weight {weight} is outside the range 0-100"),
            );
        }

        sum += u64::from(*weight);
    }

    // A single child receives all the traffic regardless of its declared weight, so the sum
    // rule does not apply. A single child at weight 0 would otherwise make the subtree
    // unreachable.
    if children.len() > 1 && sum != 100 {
        errors.push(
            format!("{path}.weight"),
            format!("sibling weights sum to {sum}, expected 100"),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::{Datacenter, Endpoint, Region};

    fn endpoint(name: &str, weight: u32) -> Endpoint {
        Endpoint {
            name: name.to_string(),
            weight,
            address: "10.0.0.1:8080".parse().unwrap(),
        }
    }

    fn hierarchy(regions: Vec<Region>) -> Hierarchy {
        Hierarchy { regions }
    }

    fn region(name: &str, weight: u32, dcs: Vec<Datacenter>) -> Region {
        Region {
            name: name.to_string(),
            weight,
            datacenters: dcs,
        }
    }

    fn datacenter(name: &str, weight: u32, clusters: Vec<Cluster>) -> Datacenter {
        Datacenter {
            name: name.to_string(),
            weight,
            clusters,
        }
    }

    fn cluster(name: &str, weight: u32, endpoints: Vec<Endpoint>) -> Cluster {
        Cluster {
            name: name.to_string(),
            weight,
            endpoints,
        }
    }

    #[test]
    fn weights_compose_multiplicatively() {
        // `us-east` has one datacenter and one cluster, so it takes the whole of the
        // region's share. The endpoint split is what actually divides the traffic.
        let h = hierarchy(vec![
            region(
                "us-east",
                60,
                vec![datacenter(
                    "dc-1",
                    70,
                    vec![cluster(
                        "prod",
                        100,
                        vec![endpoint("ep-a", 60), endpoint("ep-b", 40)],
                    )],
                )],
            ),
            region(
                "eu-west",
                40,
                vec![datacenter(
                    "dc-2",
                    100,
                    vec![cluster("prod", 100, vec![endpoint("ep-c", 100)])],
                )],
            ),
        ]);

        let flat = flatten(&h);
        assert_eq!(flat.len(), 3);
        assert!((flat[0].effective_weight - 0.36).abs() < 1e-9);
        assert!((flat[1].effective_weight - 0.24).abs() < 1e-9);
        assert!((flat[2].effective_weight - 0.40).abs() < 1e-9);
    }

    #[test]
    fn a_lone_child_takes_the_whole_share() {
        // The region is declared at 60 but is the only region, so it takes everything.
        // If the weight were taken literally, 40% of every request would vanish.
        let h = hierarchy(vec![region(
            "us-east",
            60,
            vec![datacenter(
                "dc-1",
                70,
                vec![cluster("prod", 100, vec![endpoint("ep-a", 100)])],
            )],
        )]);

        let flat = flatten(&h);
        assert_eq!(flat.len(), 1);
        assert!((flat[0].effective_weight - 1.0).abs() < 1e-9);
    }

    #[test]
    fn effective_weights_sum_to_one() {
        let h = hierarchy(vec![
            region(
                "us-east",
                60,
                vec![datacenter(
                    "dc-1",
                    70,
                    vec![cluster(
                        "prod",
                        100,
                        vec![endpoint("ep-a", 60), endpoint("ep-b", 40)],
                    )],
                )],
            ),
            region(
                "eu-west",
                40,
                vec![datacenter(
                    "dc-2",
                    100,
                    vec![cluster("prod", 100, vec![endpoint("ep-c", 100)])],
                )],
            ),
        ]);

        let total: f64 = flatten(&h).iter().map(|e| e.effective_weight).sum();
        assert!((total - 1.0).abs() < 1e-9, "total was {total}");
    }

    #[test]
    fn region_scope_renormalises() {
        let h = hierarchy(vec![region(
            "us-east",
            60,
            vec![datacenter(
                "dc-1",
                70,
                vec![cluster(
                    "prod",
                    100,
                    vec![endpoint("ep-a", 60), endpoint("ep-b", 40)],
                )],
            )],
        )]);

        let scoped = flatten_region(&h, "us-east");
        assert_eq!(scoped.len(), 2);
        assert!((scoped[0].effective_weight - 0.6).abs() < 1e-9);
        assert!((scoped[1].effective_weight - 0.4).abs() < 1e-9);
    }

    #[test]
    fn weights_must_sum_to_one_hundred() {
        let h = hierarchy(vec![region(
            "us-east",
            60,
            vec![datacenter(
                "dc-1",
                70,
                vec![cluster(
                    "prod",
                    100,
                    vec![endpoint("ep-a", 60), endpoint("ep-b", 30)],
                )],
            )],
        )]);

        let mut errors = ValidationErrors::new();
        validate_weights(&h, &mut errors);
        assert!(!errors.is_empty(), "expected a weight-sum error");
    }

    #[test]
    fn a_single_child_is_allowed_at_any_weight() {
        let h = hierarchy(vec![region(
            "us-east",
            100,
            vec![datacenter(
                "dc-1",
                100,
                vec![cluster("prod", 100, vec![endpoint("ep-a", 100)])],
            )],
        )]);

        let mut errors = ValidationErrors::new();
        validate_weights(&h, &mut errors);
        assert!(errors.is_empty(), "unexpected errors: {errors}");
    }

    #[test]
    fn duplicate_sibling_names_are_rejected() {
        let h = hierarchy(vec![region(
            "us-east",
            100,
            vec![
                datacenter(
                    "dc-1",
                    50,
                    vec![cluster("prod", 100, vec![endpoint("ep-a", 100)])],
                ),
                datacenter(
                    "dc-1",
                    50,
                    vec![cluster("prod", 100, vec![endpoint("ep-b", 100)])],
                ),
            ],
        )]);

        let mut errors = ValidationErrors::new();
        validate_weights(&h, &mut errors);
        assert!(!errors.is_empty(), "expected a duplicate-name error");
    }

    #[test]
    fn a_weight_above_one_hundred_is_rejected() {
        let h = hierarchy(vec![region(
            "us-east",
            100,
            vec![datacenter(
                "dc-1",
                100,
                vec![cluster(
                    "prod",
                    100,
                    vec![endpoint("ep-a", 120), endpoint("ep-b", 100)],
                )],
            )],
        )]);

        let mut errors = ValidationErrors::new();
        validate_weights(&h, &mut errors);
        assert!(!errors.is_empty(), "expected an out-of-range weight error");
    }
}
