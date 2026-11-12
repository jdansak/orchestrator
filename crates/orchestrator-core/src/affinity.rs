//! Session affinity: deriving a session key from a request.
//!
//! The key derivation and the backend mapping rules are specified in
//! `plan/07-config-schema.md` §6.

use std::net::IpAddr;

use crate::config::model::{AffinityConfig, AffinityMethod};

/// A session key derived from a request.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SessionKey {
    /// The value of a cookie.
    Cookie(String),

    /// The value of a request header.
    Header(String),

    /// The client's IP address.
    ClientIp(IpAddr),
}

/// The information about a request that affinity needs.
#[derive(Debug, Clone, Default)]
pub struct AffinityRequest {
    /// The value of the `Cookie` header, if present.
    pub cookie_header: Option<String>,

    /// The request headers, lowercased keys.
    pub headers: Vec<(String, String)>,

    /// The client's IP address.
    pub client_ip: Option<IpAddr>,
}

impl AffinityRequest {
    /// Returns the value of a single cookie by name.
    pub fn cookie(&self, name: &str) -> Option<&str> {
        let header = self.cookie_header.as_deref()?;
        for pair in header.split(';') {
            let pair = pair.trim();
            let Some((key, value)) = pair.split_once('=') else {
                continue;
            };
            if key.trim() == name {
                return Some(value.trim());
            }
        }
        None
    }

    /// Returns the value of a header by lowercase name.
    pub fn header(&self, name: &str) -> Option<&str> {
        let lowered = name.to_ascii_lowercase();
        self.headers
            .iter()
            .find(|(key, _)| key == &lowered)
            .map(|(_, value)| value.as_str())
    }
}

/// Derives the session key for a request, or `None` when the request has no session.
///
/// A `None` result means the request should be treated as a new session: the caller picks a
/// backend by weight and, for the `cookie` method, issues a `Set-Cookie`.
pub fn session_key(config: &AffinityConfig, request: &AffinityRequest) -> Option<SessionKey> {
    match config.method {
        AffinityMethod::None => None,
        AffinityMethod::Cookie => {
            let name = config.cookie_name.as_deref()?;
            request
                .cookie(name)
                .map(|value| SessionKey::Cookie(value.to_string()))
        }
        AffinityMethod::Header => {
            let name = config.header_name.as_deref()?;
            request
                .header(name)
                .map(|value| SessionKey::Header(value.to_string()))
        }
        AffinityMethod::ClientIp => request.client_ip.map(SessionKey::ClientIp),
    }
}

/// Whether the proxy should issue a `Set-Cookie` for a newly-created session.
///
/// Only the `cookie` method issues one — `header` and `client-ip` are derived from something the
/// client already sends.
pub fn issues_cookie(config: &AffinityConfig) -> bool {
    config.method == AffinityMethod::Cookie
}

/// Builds the `Set-Cookie` header value for a new session.
pub fn build_set_cookie(config: &AffinityConfig, session_id: &str, secure: bool) -> Option<String> {
    if !issues_cookie(config) {
        return None;
    }
    let name = config.cookie_name.as_deref()?;
    let mut value = format!("{name}={session_id}; Path=/; HttpOnly; SameSite=Lax");
    if secure {
        value.push_str("; Secure");
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::ConfigDuration;
    use std::time::Duration;

    fn config(method: AffinityMethod) -> AffinityConfig {
        AffinityConfig {
            method,
            cookie_name: Some("ORCHSID".to_string()),
            header_name: Some("X-Session-ID".to_string()),
            ttl: ConfigDuration(Duration::from_mins(30)),
        }
    }

    fn request() -> AffinityRequest {
        AffinityRequest {
            cookie_header: Some("other=1; ORCHSID=abc123; another=2".to_string()),
            headers: vec![("x-session-id".to_string(), "header-value".to_string())],
            client_ip: Some("10.0.0.5".parse().unwrap()),
        }
    }

    #[test]
    fn extracts_a_cookie_value() {
        let key = session_key(&config(AffinityMethod::Cookie), &request());
        assert_eq!(key, Some(SessionKey::Cookie("abc123".to_string())));
    }

    #[test]
    fn extracts_a_header_value() {
        let key = session_key(&config(AffinityMethod::Header), &request());
        assert_eq!(key, Some(SessionKey::Header("header-value".to_string())));
    }

    #[test]
    fn extracts_the_client_ip() {
        let key = session_key(&config(AffinityMethod::ClientIp), &request());
        assert_eq!(key, Some(SessionKey::ClientIp("10.0.0.5".parse().unwrap())));
    }

    #[test]
    fn no_affinity_yields_no_key() {
        assert_eq!(session_key(&config(AffinityMethod::None), &request()), None);
    }

    #[test]
    fn a_missing_cookie_yields_no_key() {
        let mut req = request();
        req.cookie_header = None;
        assert_eq!(session_key(&config(AffinityMethod::Cookie), &req), None);
    }

    #[test]
    fn only_the_cookie_method_issues_a_cookie() {
        assert!(issues_cookie(&config(AffinityMethod::Cookie)));
        assert!(!issues_cookie(&config(AffinityMethod::Header)));
        assert!(!issues_cookie(&config(AffinityMethod::ClientIp)));
        assert!(!issues_cookie(&config(AffinityMethod::None)));
    }

    #[test]
    fn builds_a_hardened_set_cookie_header() {
        let header = build_set_cookie(&config(AffinityMethod::Cookie), "sid", true).unwrap();
        assert!(header.starts_with("ORCHSID=sid;"));
        assert!(header.contains("HttpOnly"));
        assert!(header.contains("SameSite=Lax"));
        assert!(header.contains("Secure"));
    }
}
