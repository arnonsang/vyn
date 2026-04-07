use std::time::{SystemTime, UNIX_EPOCH};

use semver::Version;
use vyn_core::models::{load_global_config, save_global_config};

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const RELEASES_API: &str = "https://api.github.com/repos/arnonsang/vyn/releases/latest";
/// Staleness threshold in seconds (24 hours).
const CHECK_INTERVAL_SECS: u64 = 86_400;

/// Result of a version check.
pub enum VersionStatus {
    /// A newer version is available.
    UpdateAvailable(String),
    /// Current version is the latest.
    UpToDate,
    /// Could not reach GitHub (network error or parse failure).
    CheckFailed,
}

/// Checks whether a newer version of vyn is available.
///
/// When `force` is false returns a cached result if the last network check is
/// younger than 24 hours. When `force` is true it always queries the API.
///
/// All I/O and network errors are swallowed -- this must never panic or block
/// the caller for a noticeable duration.
pub fn check_for_update(force: bool) -> VersionStatus {
    let mut cfg = load_global_config();

    if !force && let Some(ts_secs) = cfg.last_version_check_unix {
        let now = unix_now();
        if now.saturating_sub(ts_secs) < CHECK_INTERVAL_SECS {
            return evaluate_cached(&cfg.latest_known_version);
        }
    }

    let latest = match fetch_latest_version() {
        Some(v) => v,
        None => return VersionStatus::CheckFailed,
    };

    cfg.last_version_check_unix = Some(unix_now());
    cfg.latest_known_version = Some(latest.clone());
    // Best-effort save -- ignore errors so a read-only filesystem doesn't break anything.
    let _ = save_global_config(&cfg);

    evaluate_new(CURRENT_VERSION, &latest)
}

/// Returns `Some(latest)` if newer than current, otherwise `None`.
/// Convenience wrapper around `check_for_update` for the update hint.
pub fn newer_version(force: bool) -> Option<String> {
    match check_for_update(force) {
        VersionStatus::UpdateAvailable(v) => Some(v),
        _ => None,
    }
}

/// Spawns a background thread that updates the version cache without blocking
/// the main command. The result is discarded -- the hint will show on the next run.
pub fn spawn_background_check() {
    std::thread::spawn(|| {
        check_for_update(false);
    });
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn evaluate_cached(latest_known: &Option<String>) -> VersionStatus {
    match latest_known {
        Some(latest) => evaluate_new(CURRENT_VERSION, latest),
        None => VersionStatus::CheckFailed,
    }
}

fn evaluate_new(current: &str, latest: &str) -> VersionStatus {
    let cur = match Version::parse(current) {
        Ok(v) => v,
        Err(_) => return VersionStatus::CheckFailed,
    };
    let lat = match Version::parse(latest) {
        Ok(v) => v,
        Err(_) => return VersionStatus::CheckFailed,
    };
    if lat > cur {
        VersionStatus::UpdateAvailable(latest.to_string())
    } else {
        VersionStatus::UpToDate
    }
}

fn fetch_latest_version() -> Option<String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(format!("vyn/{CURRENT_VERSION}"))
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .ok()?;

    let resp: serde_json::Value = client.get(RELEASES_API).send().ok()?.json().ok()?;

    let tag = resp.get("tag_name")?.as_str()?;
    // Strip leading 'v' if present.
    Some(tag.trim_start_matches('v').to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_version_detected() {
        assert!(matches!(
            evaluate_new("0.1.2", "0.1.3"),
            VersionStatus::UpdateAvailable(v) if v == "0.1.3"
        ));
    }

    #[test]
    fn same_version_up_to_date() {
        assert!(matches!(
            evaluate_new("0.1.3", "0.1.3"),
            VersionStatus::UpToDate
        ));
    }

    #[test]
    fn older_remote_up_to_date() {
        assert!(matches!(
            evaluate_new("0.1.4", "0.1.3"),
            VersionStatus::UpToDate
        ));
    }

    #[test]
    fn invalid_version_string_check_failed() {
        assert!(matches!(
            evaluate_new("not-a-version", "0.1.3"),
            VersionStatus::CheckFailed
        ));
    }
}
