// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Background version check that notifies users of available CLI updates.
//!
//! This module checks for new CLI versions at most once per day by caching the result
//! in `~/.aptos/version_check.json`. The check is performed in a background task so it
//! never blocks CLI execution.
//!
//! The flow is:
//! 1. On startup, read the local cache file (fast local I/O).
//! 2. If cached data indicates a newer version, print a notice to stderr.
//! 3. If the cache is stale (>24 hours) or missing, spawn a non-blocking background
//!    task that queries the GitHub API and writes the result to the cache for next time.
//!
//! Users can disable this by setting `APTOS_DISABLE_UPDATE_CHECK=1`.

use colored::Colorize;
use self_update::cargo_crate_version;
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

/// How often to check for updates (24 hours).
const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// Environment variable to disable the update check.
const DISABLE_ENV_VAR: &str = "APTOS_DISABLE_UPDATE_CHECK";

/// The cache file name stored in `~/.aptos/`.
const VERSION_CHECK_FILE: &str = "version_check.json";

/// GitHub API URL to fetch the latest CLI release tag.
/// We use the releases endpoint filtered to find the `aptos-cli-v*` tag.
const GITHUB_RELEASES_URL: &str =
    "https://api.github.com/repos/aptos-labs/aptos-core/releases?per_page=30";

/// Cached version check information written to disk.
#[derive(Debug, Serialize, Deserialize)]
struct VersionCheckCache {
    /// Unix timestamp (seconds) of the last successful check.
    last_check_epoch_secs: u64,
    /// The latest CLI version found (e.g. "3.5.0"), without any prefix.
    latest_version: String,
}

/// A minimal representation of a GitHub release for deserialization.
#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

/// Returns the path to the version check cache file.
fn cache_file_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".aptos").join(VERSION_CHECK_FILE))
}

/// Read the cached version check info from disk. Returns `None` if the file
/// doesn't exist or can't be parsed (we silently ignore errors).
fn read_cache() -> Option<VersionCheckCache> {
    let path = cache_file_path()?;
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

/// Write version check info to the cache file. Errors are silently ignored.
fn write_cache(cache: &VersionCheckCache) {
    if let Some(path) = cache_file_path() {
        // Ensure the parent directory exists.
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(data) = serde_json::to_string_pretty(cache) {
            let _ = std::fs::write(path, data);
        }
    }
}

/// Returns the current time as seconds since UNIX epoch.
fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Returns `true` if the version check is disabled via environment variable.
fn is_check_disabled() -> bool {
    matches!(
        std::env::var(DISABLE_ENV_VAR).as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    )
}

/// Check the local cache and print an update notice to stderr if a newer
/// version is available. This is a fast, synchronous, local-only operation.
///
/// Returns a `JoinHandle` for the background refresh task if one was spawned,
/// so the caller can optionally wait for it (though it's fine to drop it).
pub fn check_for_update_and_notify() -> Option<tokio::task::JoinHandle<()>> {
    if is_check_disabled() {
        return None;
    }

    let current_version = cargo_crate_version!();

    // Read cached data and decide whether we need to refresh.
    let cache = read_cache();
    let needs_refresh = match &cache {
        Some(c) => {
            let age = now_epoch_secs().saturating_sub(c.last_check_epoch_secs);
            age >= CHECK_INTERVAL.as_secs()
        },
        None => true,
    };

    // If we have cached data, check if there's a newer version and notify.
    if let Some(ref cache) = cache {
        if version_is_newer(current_version, &cache.latest_version) {
            print_update_notice(current_version, &cache.latest_version);
        }
    }

    // If the cache is stale or missing, spawn a background task to refresh it.
    if needs_refresh {
        Some(tokio::spawn(async move {
            let _ = refresh_cache_background().await;
        }))
    } else {
        None
    }
}

/// Compare two semver-like version strings and return `true` if `latest` is
/// newer than `current`. We do a simple numeric comparison of major.minor.patch.
fn version_is_newer(current: &str, latest: &str) -> bool {
    // Strip any leading "v" prefix just in case.
    let current = current.strip_prefix('v').unwrap_or(current);
    let latest = latest.strip_prefix('v').unwrap_or(latest);

    let parse = |v: &str| -> Option<(u64, u64, u64)> {
        // Take only the first three numeric components (ignore suffixes like ".beta").
        let parts: Vec<&str> = v.split('.').collect();
        if parts.len() < 3 {
            return None;
        }
        Some((
            parts[0].parse().ok()?,
            parts[1].parse().ok()?,
            parts[2].parse().ok()?,
        ))
    };

    match (parse(current), parse(latest)) {
        (Some(c), Some(l)) => l > c,
        _ => false,
    }
}

/// Print a user-friendly update notice to stderr (so it doesn't interfere with
/// JSON output on stdout).
fn print_update_notice(current: &str, latest: &str) {
    let notice = format!(
        "Update available: {} -> {} (run `aptos update aptos` or see https://aptos.dev/tools/aptos-cli/install-cli/)",
        current, latest
    );
    eprintln!("{}\n", notice.yellow());
}

/// Fetch the latest CLI version from GitHub and update the local cache.
/// This runs in a background task and silently ignores all errors.
async fn refresh_cache_background() -> Option<()> {
    let latest_version = fetch_latest_cli_version().await?;
    let cache = VersionCheckCache {
        last_check_epoch_secs: now_epoch_secs(),
        latest_version,
    };
    write_cache(&cache);
    Some(())
}

/// Query the GitHub releases API to find the latest `aptos-cli-v*` release tag.
/// Returns `None` on any error (network failure, parse error, etc.).
async fn fetch_latest_cli_version() -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .user_agent("aptos-cli")
        .build()
        .ok()?;

    let releases: Vec<GitHubRelease> = client
        .get(GITHUB_RELEASES_URL)
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;

    // Find the first release whose tag starts with "aptos-cli-v".
    for release in releases {
        if let Some(version) = release.tag_name.strip_prefix("aptos-cli-v") {
            return Some(version.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_is_newer() {
        assert!(version_is_newer("1.0.0", "2.0.0"));
        assert!(version_is_newer("1.0.0", "1.1.0"));
        assert!(version_is_newer("1.0.0", "1.0.1"));
        assert!(!version_is_newer("2.0.0", "1.0.0"));
        assert!(!version_is_newer("1.0.0", "1.0.0"));
        assert!(version_is_newer("7.14.2", "7.15.0"));
        assert!(!version_is_newer("7.14.2", "7.14.2"));
        assert!(!version_is_newer("7.14.2", "7.14.1"));
    }

    #[test]
    fn test_version_is_newer_with_prefix() {
        assert!(version_is_newer("v1.0.0", "v2.0.0"));
        assert!(version_is_newer("1.0.0", "v2.0.0"));
    }

    #[test]
    fn test_version_is_newer_with_suffix() {
        // "1.0.0.beta" — the fourth component is not numeric so we only parse 3
        assert!(version_is_newer("1.0.0", "2.0.0.beta"));
        assert!(!version_is_newer("2.0.0", "1.0.0.beta"));
    }

    #[test]
    fn test_version_is_newer_malformed() {
        assert!(!version_is_newer("abc", "2.0.0"));
        assert!(!version_is_newer("1.0.0", "xyz"));
        assert!(!version_is_newer("", ""));
    }

    #[test]
    fn test_is_check_disabled() {
        // By default the env var is not set, so the check is enabled.
        // Note: we don't remove the env var here since remove_var is unsafe
        // in recent Rust versions. Instead we test the logic with a known state.
        if std::env::var(DISABLE_ENV_VAR).is_err() {
            assert!(!is_check_disabled());
        }
    }

    #[test]
    fn test_cache_serialization() {
        let cache = VersionCheckCache {
            last_check_epoch_secs: 1700000000,
            latest_version: "3.5.0".to_string(),
        };
        let json = serde_json::to_string(&cache).unwrap();
        let parsed: VersionCheckCache = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.last_check_epoch_secs, 1700000000);
        assert_eq!(parsed.latest_version, "3.5.0");
    }
}
