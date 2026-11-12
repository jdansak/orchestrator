//! Parsing for the human-readable duration and size strings used in configuration files.
//!
//! Durations look like `30s`, `5m`, `1h30m`, `500ms`. Sizes look like `512M`, `2G`, `1K`.
//! Bare integers are **rejected** — a config that says `timeout: 30` is ambiguous (seconds?)
//! and silently guessing is how a traffic manager ends up with a 30-second timeout that someone
//! meant to be 30 milliseconds.

use std::fmt;
use std::time::Duration;

/// The error returned when a duration string cannot be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseDurationError {
    /// The input that failed to parse.
    pub input: String,
    /// Why it failed.
    pub reason: String,
}

impl fmt::Display for ParseDurationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid duration '{}': {}", self.input, self.reason)
    }
}

impl std::error::Error for ParseDurationError {}

/// The error returned when a size string cannot be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseSizeError {
    /// The input that failed to parse.
    pub input: String,
    /// Why it failed.
    pub reason: String,
}

impl fmt::Display for ParseSizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid size '{}': {}", self.input, self.reason)
    }
}

impl std::error::Error for ParseSizeError {}

/// Parses a duration string such as `30s`, `5m`, `1h30m`, or `500ms`.
///
/// The result is a [`Duration`]. Components may appear in any order, but each unit may appear at
/// most once.
pub fn parse_duration(input: &str) -> Result<Duration, ParseDurationError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ParseDurationError {
            input: input.to_string(),
            reason: "value is empty".to_string(),
        });
    }

    let bytes = trimmed.as_bytes();
    let mut index = 0usize;
    let mut total = Duration::ZERO;
    let mut matched_any = false;

    while index < bytes.len() {
        let start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if index == start {
            return Err(ParseDurationError {
                input: input.to_string(),
                reason: format!("expected a number at position {start}"),
            });
        }
        let number: u64 = trimmed[start..index]
            .parse()
            .map_err(|_| ParseDurationError {
                input: input.to_string(),
                reason: format!("number at position {start} is out of range"),
            })?;

        let unit_start = index;
        while index < bytes.len() && !bytes[index].is_ascii_digit() {
            index += 1;
        }
        let unit = &trimmed[unit_start..index];

        let duration = match unit {
            "ms" => Duration::from_millis(number),
            "s" => Duration::from_secs(number),
            "m" => Duration::from_secs(number.saturating_mul(60)),
            "h" => Duration::from_secs(number.saturating_mul(3_600)),
            "d" => Duration::from_secs(number.saturating_mul(86_400)),
            "" => {
                return Err(ParseDurationError {
                    input: input.to_string(),
                    reason: format!("missing unit after the number at position {start}"),
                })
            }
            other => {
                return Err(ParseDurationError {
                    input: input.to_string(),
                    reason: format!("unknown unit '{other}'"),
                })
            }
        };

        total = total
            .checked_add(duration)
            .ok_or_else(|| ParseDurationError {
                input: input.to_string(),
                reason: "duration overflows".to_string(),
            })?;
        matched_any = true;
    }

    if !matched_any {
        return Err(ParseDurationError {
            input: input.to_string(),
            reason: "no duration components found".to_string(),
        });
    }

    Ok(total)
}

/// Parses a size string such as `512M`, `2G`, or `1K`.
///
/// A bare integer is interpreted as a byte count.
pub fn parse_size(input: &str) -> Result<u64, ParseSizeError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ParseSizeError {
            input: input.to_string(),
            reason: "value is empty".to_string(),
        });
    }

    let split = trimmed
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(trimmed.len());
    let (digits, suffix) = trimmed.split_at(split);

    let number: u64 = digits.parse().map_err(|_| ParseSizeError {
        input: input.to_string(),
        reason: "value is not a valid size".to_string(),
    })?;

    let multiplier: u64 = match suffix.to_ascii_uppercase().as_str() {
        "" | "B" => 1,
        "K" | "KB" => 1_024,
        "M" | "MB" => 1_024 * 1_024,
        "G" | "GB" => 1_024 * 1_024 * 1_024,
        other => {
            return Err(ParseSizeError {
                input: input.to_string(),
                reason: format!("unknown unit '{other}'"),
            })
        }
    };

    number
        .checked_mul(multiplier)
        .ok_or_else(|| ParseSizeError {
            input: input.to_string(),
            reason: "size overflows".to_string(),
        })
}

/// Serde helpers for deserializing a [`Duration`] from a human-readable string.
pub mod serde_de {
    use super::parse_duration;
    use serde::{Deserialize, Deserializer};
    use std::time::Duration;

    /// Deserializes a duration from a string such as `"30s"`.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        parse_duration(&raw).map_err(serde::de::Error::custom)
    }
}

/// Serde helpers for serializing a [`Duration`] as a human-readable string.
pub mod serde_ser {
    use serde::Serializer;
    use std::fmt;
    use std::time::Duration;

    /// Serializes a duration as a string such as `"30s"`.
    pub fn serialize<S>(value: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(&DurationDisplay(*value))
    }

    struct DurationDisplay(Duration);

    impl fmt::Display for DurationDisplay {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let total = self.0.as_secs();
            if total == 0 {
                return write!(f, "{}ms", self.0.as_millis());
            }
            if total.is_multiple_of(86_400) {
                return write!(f, "{}d", total / 86_400);
            }
            if total.is_multiple_of(3_600) {
                return write!(f, "{}h", total / 3_600);
            }
            if total.is_multiple_of(60) {
                return write!(f, "{}m", total / 60);
            }
            write!(f, "{total}s")
        }
    }
}

/// Combined serde helpers, for use with `#[serde(with = ...)]` on a newtype.
pub mod serde_str {
    pub use super::serde_de::deserialize;
    pub use super::serde_ser::serialize;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_durations() {
        assert_eq!(parse_duration("30s").unwrap(), Duration::from_secs(30));
        assert_eq!(parse_duration("5m").unwrap(), Duration::from_secs(300));
        assert_eq!(parse_duration("2h").unwrap(), Duration::from_secs(7_200));
        assert_eq!(parse_duration("1d").unwrap(), Duration::from_hours(24));
        assert_eq!(parse_duration("500ms").unwrap(), Duration::from_millis(500));
    }

    #[test]
    fn parses_compound_durations() {
        assert_eq!(parse_duration("1h30m").unwrap(), Duration::from_mins(90));
        assert_eq!(parse_duration("1m30s").unwrap(), Duration::from_secs(90));
    }

    #[test]
    fn trims_whitespace() {
        assert_eq!(parse_duration("  30s  ").unwrap(), Duration::from_secs(30));
    }

    #[test]
    fn rejects_bare_numbers() {
        let error = parse_duration("30").unwrap_err();
        assert!(error.reason.contains("missing unit"), "got: {error}");
    }

    #[test]
    fn rejects_unknown_units() {
        let error = parse_duration("30x").unwrap_err();
        assert!(error.reason.contains("unknown unit"), "got: {error}");
    }

    #[test]
    fn rejects_empty_input() {
        assert!(parse_duration("").is_err());
        assert!(parse_duration("   ").is_err());
    }

    #[test]
    fn parses_sizes() {
        assert_eq!(parse_size("512").unwrap(), 512);
        assert_eq!(parse_size("512B").unwrap(), 512);
        assert_eq!(parse_size("1K").unwrap(), 1_024);
        assert_eq!(parse_size("512M").unwrap(), 512 * 1_024 * 1_024);
        assert_eq!(parse_size("2G").unwrap(), 2 * 1_024 * 1_024 * 1_024);
    }

    #[test]
    fn rejects_unknown_size_units() {
        assert!(parse_size("512Z").is_err());
    }
}
