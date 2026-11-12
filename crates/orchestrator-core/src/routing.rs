//! Route matching.
//!
//! The rules are specified in `plan/07-config-schema.md` §4. In short:
//!
//! - Every route whose match succeeds is a candidate
//! - Among candidates, the **longest prefix wins**
//! - A prefix match respects path boundaries — `/api` matches `/api` and `/api/v2`, but not
//!   `/apiv2`
//! - Query strings are not part of the match
//! - Matching is case-sensitive

use crate::config::model::{MatchType, Route};

/// The outcome of matching a request path against the route table.
#[derive(Debug, Clone, PartialEq)]
pub enum MatchOutcome {
    /// No route matched. The proxy returns `404`.
    NoMatch,

    /// Exactly one route matched.
    Matched {
        /// The index of the matching route.
        index: usize,
        /// The route itself.
        route: Route,
    },

    /// Several routes matched. The longest prefix wins; `others` lists the routes that lost.
    Ambiguous {
        /// The index of the winning route.
        index: usize,
        /// The winning route.
        route: Route,
        /// The indices of the routes that matched but were shorter.
        others: Vec<usize>,
    },
}

/// Normalises a configured path: strips a trailing `/*` wildcard and any trailing slash.
///
/// `/api/v2/*` and `/api/v2` are equivalent — see `plan/07-config-schema.md` §4.2.
pub fn normalize_configured_path(path: &str) -> &str {
    let trimmed = path.strip_suffix("/*").unwrap_or(path);
    if trimmed.len() > 1 {
        trimmed.strip_suffix('/').unwrap_or(trimmed)
    } else {
        trimmed
    }
}

/// Strips the query string and fragment from a request path.
pub fn strip_query(path: &str) -> &str {
    let without_fragment = path.split('#').next().unwrap_or(path);
    without_fragment
        .split('?')
        .next()
        .unwrap_or(without_fragment)
}

/// Returns `true` when `request_path` matches `configured_path` under `match_type`.
pub fn path_matches(configured_path: &str, request_path: &str, match_type: MatchType) -> bool {
    let request = strip_query(request_path);
    let configured = normalize_configured_path(configured_path);

    match match_type {
        MatchType::Exact => request == configured,
        MatchType::Prefix => {
            if request == configured {
                return true;
            }
            if !request.starts_with(configured) {
                return false;
            }
            // Respect path boundaries: `/api` matches `/api/v2` but not `/apiv2`.
            let remainder = &request[configured.len()..];
            remainder.starts_with('/') || configured.ends_with('/')
        }
    }
}

/// Matches a request path against the route table.
///
/// Returns the winning route and the indices of any other routes that matched but lost to a
/// longer prefix.
pub fn match_route(routes: &[Route], request_path: &str) -> MatchOutcome {
    let mut best: Option<(usize, &Route, usize)> = None;
    let mut others = Vec::new();

    for (index, route) in routes.iter().enumerate() {
        if !route.enabled {
            continue;
        }
        if !path_matches(&route.r#match.path, request_path, route.r#match.r#type) {
            continue;
        }

        let length = normalize_configured_path(&route.r#match.path).len();

        match best {
            None => best = Some((index, route, length)),
            Some((_, _, best_length)) if length > best_length => {
                others.push(best.expect("best is set").0);
                best = Some((index, route, length));
            }
            Some(_) => others.push(index),
        }
    }

    match best {
        None => MatchOutcome::NoMatch,
        Some((index, route, _)) if others.is_empty() => MatchOutcome::Matched {
            index,
            route: route.clone(),
        },
        Some((index, route, _)) => MatchOutcome::Ambiguous {
            index,
            route: route.clone(),
            others,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::{AffinityConfig, RouteMatch, RouteScope};

    fn route(name: &str, path: &str, kind: MatchType) -> Route {
        Route {
            name: name.to_string(),
            enabled: true,
            r#match: RouteMatch {
                path: path.to_string(),
                r#type: kind,
            },
            scope: RouteScope {
                region: "*".to_string(),
            },
            affinity: AffinityConfig::default(),
        }
    }

    #[test]
    fn prefix_respects_path_boundaries() {
        assert!(path_matches("/api", "/api", MatchType::Prefix));
        assert!(path_matches("/api", "/api/v2/users", MatchType::Prefix));
        assert!(!path_matches("/api", "/apiv2", MatchType::Prefix));
        assert!(!path_matches("/api", "/apix", MatchType::Prefix));
    }

    #[test]
    fn exact_requires_an_identical_path() {
        assert!(path_matches("/health", "/health", MatchType::Exact));
        assert!(!path_matches("/health", "/health/live", MatchType::Exact));
        assert!(!path_matches("/health", "/healthz", MatchType::Exact));
    }

    #[test]
    fn query_strings_are_ignored() {
        assert!(path_matches("/api", "/api?x=1&y=2", MatchType::Prefix));
        assert!(path_matches("/health", "/health?x=1", MatchType::Exact));
    }

    #[test]
    fn trailing_wildcard_is_normalised() {
        assert_eq!(normalize_configured_path("/api/v2/*"), "/api/v2");
        assert_eq!(normalize_configured_path("/api/v2"), "/api/v2");
        assert_eq!(normalize_configured_path("/"), "/");
    }

    #[test]
    fn longest_prefix_wins() {
        let routes = vec![
            route("root", "/", MatchType::Prefix),
            route("api", "/api", MatchType::Prefix),
            route("api-v2", "/api/v2", MatchType::Prefix),
        ];

        match match_route(&routes, "/api/v2/users") {
            MatchOutcome::Ambiguous { route, others, .. } => {
                assert_eq!(route.name, "api-v2");
                assert_eq!(others.len(), 2);
            }
            other => panic!("expected an ambiguous match, got {other:?}"),
        }
    }

    #[test]
    fn a_single_match_is_unambiguous() {
        let routes = vec![route("health", "/health", MatchType::Exact)];
        match match_route(&routes, "/health") {
            MatchOutcome::Matched { route, .. } => assert_eq!(route.name, "health"),
            other => panic!("expected a match, got {other:?}"),
        }
    }

    #[test]
    fn no_match_is_reported() {
        let routes = vec![route("health", "/health", MatchType::Exact)];
        assert_eq!(match_route(&routes, "/nope"), MatchOutcome::NoMatch);
    }

    #[test]
    fn disabled_routes_are_skipped() {
        let mut routes = vec![route("health", "/health", MatchType::Exact)];
        routes[0].enabled = false;
        assert_eq!(match_route(&routes, "/health"), MatchOutcome::NoMatch);
    }

    #[test]
    fn matching_is_case_sensitive() {
        assert!(!path_matches("/Health", "/health", MatchType::Exact));
    }
}
