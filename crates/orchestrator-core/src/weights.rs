//! Weighted random selection over the flattened endpoint list.
//!
//! Selection is **weighted random** using a cumulative-distribution lookup: draw a uniform
//! random number in `[0, 1)`, walk the leaves in config order accumulating their effective
//! weight, and return the first leaf whose cumulative total exceeds the draw.
//!
//! This is deliberately *not* smooth weighted round-robin. The requirement asks for random
//! selection, and a deterministic round-robin would make the distribution tests pass trivially
//! while hiding real randomness bugs.

use crate::hierarchy::FlatEndpoint;

/// Picks an endpoint using the weighted-random rule.
///
/// `draw` must be in `[0, 1)`. Callers normally pass a value from a uniform random source; the
/// parameter exists so the selection can be tested deterministically.
pub fn select(endpoints: &[FlatEndpoint], draw: f64) -> Option<&FlatEndpoint> {
    if endpoints.is_empty() {
        return None;
    }

    let mut cumulative = 0.0f64;
    for endpoint in endpoints {
        cumulative += endpoint.effective_weight;
        if draw < cumulative {
            return Some(endpoint);
        }
    }

    // Floating-point accumulation can leave the total a hair below 1.0, which would leave a
    // draw in the final sliver unmatched. Fall back to the last endpoint.
    endpoints.last()
}

/// Draws a uniformly distributed value in `[0, 1)`.
///
/// Uses a simple xorshift64* generator seeded from the system clock. This is used for traffic
/// distribution, not for anything security-sensitive — session affinity and token generation
/// use a CSPRNG instead.
pub fn draw() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0x9E37_79B9_7F4A_7C15, |d| d.as_nanos() as u64);

    let mut state = nanos ^ 0x9E37_79B9_7F4A_7C15;
    // Warm up the generator.
    for _ in 0..4 {
        state = xorshift64(state);
    }

    // Take the top 53 bits, the full mantissa of an f64.
    ((state >> 11) as f64) / ((1u64 << 53) as f64)
}

/// The xorshift64* generator.
fn xorshift64(mut state: u64) -> u64 {
    state ^= state >> 12;
    state ^= state << 25;
    state ^= state >> 27;
    state.wrapping_mul(0x2545_F491_4F6C_DD1D)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::Endpoint;
    use std::net::SocketAddr;

    fn endpoint(name: &str, weight: f64) -> FlatEndpoint {
        FlatEndpoint {
            path: format!("us-east/dc-1/prod/{name}"),
            region: "us-east".to_string(),
            datacenter: "dc-1".to_string(),
            cluster: "prod".to_string(),
            endpoint: Endpoint {
                name: name.to_string(),
                weight: 100,
                address: "10.0.0.1:8080".parse::<SocketAddr>().unwrap(),
            },
            effective_weight: weight,
        }
    }

    #[test]
    fn selects_by_cumulative_distribution() {
        let endpoints = vec![endpoint("a", 0.6), endpoint("b", 0.4)];

        assert_eq!(select(&endpoints, 0.0).unwrap().endpoint.name, "a");
        assert_eq!(select(&endpoints, 0.59).unwrap().endpoint.name, "a");
        assert_eq!(select(&endpoints, 0.6).unwrap().endpoint.name, "b");
        assert_eq!(select(&endpoints, 0.99).unwrap().endpoint.name, "b");
    }

    #[test]
    fn empty_selection_returns_none() {
        assert!(select(&[], 0.5).is_none());
    }

    #[test]
    fn distribution_is_roughly_proportional() {
        let endpoints = vec![endpoint("a", 0.6), endpoint("b", 0.4)];
        let mut a_count = 0u32;
        let mut b_count = 0u32;

        for i in 0..100_000 {
            let draw = f64::from(i) / 100_000.0;
            match select(&endpoints, draw).unwrap().endpoint.name.as_str() {
                "a" => a_count += 1,
                _ => b_count += 1,
            }
        }

        let ratio = f64::from(a_count) / f64::from(a_count + b_count);
        assert!((ratio - 0.6).abs() < 0.01, "ratio was {ratio}");
    }

    #[test]
    fn draws_are_in_range() {
        for _ in 0..1000 {
            let value = draw();
            assert!((0.0..1.0).contains(&value), "draw was {value}");
        }
    }
}
