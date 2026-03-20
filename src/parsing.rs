use std::str::FromStr;
use std::sync::LazyLock;

use ::http::header::HeaderName;
use ::http::HeaderValue;
use anyhow::{Context, Error, Result};
use regex::Regex;
use tokio::time::Duration;

/// Matches a string like '12d 24h 5m 45s' to a regex capture.
pub(crate) static DURATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        "(?P<days>[0-9]+)d|(?P<hours>[0-9]+)h|(?P<minutes>[0-9]+)m|(?P<seconds>[0-9]+)s",
    )
    .unwrap()
});

/// Parses a duration string from the CLI to a Duration.
/// '11d 3h 32m 4s' -> Duration
///
/// If no matches are found for the string or an invalid match
/// is captured an error message is returned.
pub(crate) fn parse_duration(duration: &str) -> Result<Duration> {
    let mut dur = Duration::default();

    for cap in DURATION_RE.captures_iter(duration) {
        let add_to = if let Some(days) = cap.name("days") {
            let days = days.as_str().parse::<u64>()?;
            let seconds = days
                .checked_mul(24 * 60 * 60)
                .ok_or_else(|| anyhow::anyhow!("duration overflow: {} days", days))?;
            Duration::from_secs(seconds)
        } else if let Some(hours) = cap.name("hours") {
            let hours = hours.as_str().parse::<u64>()?;
            let seconds = hours
                .checked_mul(60 * 60)
                .ok_or_else(|| anyhow::anyhow!("duration overflow: {} hours", hours))?;
            Duration::from_secs(seconds)
        } else if let Some(minutes) = cap.name("minutes") {
            let minutes = minutes.as_str().parse::<u64>()?;
            let seconds = minutes
                .checked_mul(60)
                .ok_or_else(|| anyhow::anyhow!("duration overflow: {} minutes", minutes))?;
            Duration::from_secs(seconds)
        } else if let Some(seconds) = cap.name("seconds") {
            let seconds = seconds.as_str().parse::<u64>()?;
            Duration::from_secs(seconds)
        } else {
            return Err(Error::msg(format!("invalid match: {:?}", cap)));
        };

        dur += add_to;
    }

    if dur.as_secs() == 0 {
        return Err(Error::msg(format!(
            "failed to extract any valid duration from {}",
            duration
        )));
    }

    Ok(dur)
}

/// Parses a header string of the form "Key: value" into a `(HeaderName, HeaderValue)` pair.
pub(crate) fn parse_header(value: &str) -> Result<(HeaderName, HeaderValue)> {
    let (key, value) = value
        .split_once(": ")
        .context("Header value missing colon (\": \")")?;
    let key = HeaderName::from_str(key).context("Invalid header name")?;
    let value = HeaderValue::from_str(value).context("Invalid header value")?;
    Ok((key, value))
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- parse_duration tests ---

    #[test]
    fn parse_duration_seconds_only() {
        let d = parse_duration("10s").expect("should parse");
        assert_eq!(d, Duration::from_secs(10));
    }

    #[test]
    fn parse_duration_hours_and_minutes() {
        let d = parse_duration("1h30m").expect("should parse");
        assert_eq!(d, Duration::from_secs(90 * 60));
    }

    #[test]
    fn parse_duration_one_day() {
        let d = parse_duration("1d").expect("should parse");
        assert_eq!(d, Duration::from_secs(86400));
    }

    #[test]
    fn parse_duration_combined() {
        // 1d2h3m4s = 86400 + 7200 + 180 + 4 = 93784
        let d = parse_duration("1d2h3m4s").expect("should parse");
        assert_eq!(d, Duration::from_secs(93784));
    }

    #[test]
    fn parse_duration_empty_string_errors() {
        let result = parse_duration("");
        assert!(result.is_err(), "empty string should be an error");
    }

    #[test]
    fn parse_duration_non_duration_string_errors() {
        let result = parse_duration("abc");
        assert!(result.is_err(), "non-duration string should be an error");
    }

    #[test]
    fn parse_duration_zero_seconds_errors() {
        let result = parse_duration("0s");
        assert!(result.is_err(), "zero duration should be an error");
    }

    #[test]
    fn parse_duration_overflow_errors() {
        // u64::MAX / 86400 ≈ 2.13e14; use a value well beyond that.
        let huge = format!("{}d", u64::MAX / 86400 + 1);
        let result = parse_duration(&huge);
        assert!(result.is_err(), "overflow days should be an error");
    }

    // --- parse_header tests ---

    #[test]
    fn parse_header_valid() {
        let (name, value) = parse_header("Content-Type: application/json").expect("should parse");
        assert_eq!(name.as_str(), "content-type");
        assert_eq!(value.to_str().unwrap(), "application/json");
    }

    #[test]
    fn parse_header_missing_colon_errors() {
        let result = parse_header("InvalidHeader");
        assert!(result.is_err(), "missing colon should be an error");
    }

    #[test]
    fn parse_header_empty_string_errors() {
        let result = parse_header("");
        assert!(result.is_err(), "empty string should be an error");
    }
}
