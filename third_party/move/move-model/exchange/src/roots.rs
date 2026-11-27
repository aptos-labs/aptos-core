// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Resolves root checkout locations from environment variables.
//!
//! No defaults, no upward directory walks, and no derivation from one tree's
//! position to another.

use anyhow::{bail, Context, Result};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

/// Environment variable naming the root of a Verex checkout.
pub const VEREX_ROOT_ENV: &str = "VEREX_ROOT";

/// Standard error message when `VEREX_ROOT` is unset or empty.
pub const VEREX_ROOT_UNSET_MESSAGE: &str =
    "VEREX_ROOT is not set: set it to the root of a verex checkout (the directory holding verex-ir, verex-move, verex-rust and verex-e2e-tests).";

/// Marker files checked in order under `VEREX_ROOT`.
const VEREX_ROOT_MARKERS: &[&str] = &["verex-ir/lakefile.toml", "verex-move/lakefile.toml"];

/// Whether `VEREX_ROOT` is set to a non-empty value.
///
/// Lean-route test harnesses use this to ignore tests when `VEREX_ROOT` is
/// unset, while still running (and failing with [`verex_root`]'s standard
/// message) whenever it is set to an invalid path.
pub fn verex_root_is_set() -> bool {
    std::env::var_os(VEREX_ROOT_ENV).is_some_and(|value| !value.is_empty())
}

/// Resolves and validates `VEREX_ROOT` from the process environment.
///
/// Check order:
/// 1. unset or empty -> standard unset message;
/// 2. not an absolute path -> standard non-absolute message;
/// 3. missing marker (`verex-ir/lakefile.toml`, then `verex-move/lakefile.toml`)
///    -> standard missing-marker message naming the first missing marker;
/// 4. canonical path (symlinks resolved, no trailing slash).
pub fn verex_root() -> Result<PathBuf> {
    resolve_verex_root(std::env::var_os(VEREX_ROOT_ENV).as_deref())
}

/// Resolves and validates a `VEREX_ROOT` value without consulting the process
/// environment.
pub fn resolve_verex_root(value: Option<&OsStr>) -> Result<PathBuf> {
    let Some(value) = value.filter(|value| !value.is_empty()) else {
        bail!("{VEREX_ROOT_UNSET_MESSAGE}");
    };
    let raw = value.to_string_lossy();
    let path = Path::new(value);
    if !path.is_absolute() {
        bail!("{VEREX_ROOT_ENV}={raw} is not an absolute path.");
    }
    for marker in VEREX_ROOT_MARKERS {
        if !path.join(marker).is_file() {
            bail!("{VEREX_ROOT_ENV}={raw} is not a verex checkout: {raw}/{marker} is missing.");
        }
    }
    std::fs::canonicalize(path)
        .with_context(|| format!("failed to canonicalize {VEREX_ROOT_ENV}={raw}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(label: &str) -> Self {
            let base = std::env::temp_dir();
            let pid = std::process::id();
            loop {
                let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
                let path = base.join(format!("move-model-exchange-roots-{label}-{pid}-{id}"));
                match std::fs::create_dir(&path) {
                    Ok(()) => return Self { path },
                    Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(err) => panic!("create temp test dir {}: {err}", path.display()),
                }
            }
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn unset_and_empty_return_standard_message() {
        assert_eq!(
            resolve_verex_root(None).unwrap_err().to_string(),
            VEREX_ROOT_UNSET_MESSAGE
        );
        assert_eq!(
            resolve_verex_root(Some(OsStr::new("")))
                .unwrap_err()
                .to_string(),
            VEREX_ROOT_UNSET_MESSAGE
        );
    }

    #[test]
    fn relative_path_returns_standard_message() {
        assert_eq!(
            resolve_verex_root(Some(OsStr::new("relative/verex")))
                .unwrap_err()
                .to_string(),
            "VEREX_ROOT=relative/verex is not an absolute path."
        );
    }

    #[test]
    fn missing_markers_reported_in_order() {
        let temp = TempDir::new("markers");
        let raw = temp.path.to_string_lossy();
        assert_eq!(
            resolve_verex_root(Some(temp.path.as_os_str()))
                .unwrap_err()
                .to_string(),
            format!("VEREX_ROOT={raw} is not a verex checkout: {raw}/verex-ir/lakefile.toml is missing.")
        );

        std::fs::create_dir_all(temp.path.join("verex-ir")).unwrap();
        std::fs::write(temp.path.join("verex-ir/lakefile.toml"), "").unwrap();
        assert_eq!(
            resolve_verex_root(Some(temp.path.as_os_str()))
                .unwrap_err()
                .to_string(),
            format!("VEREX_ROOT={raw} is not a verex checkout: {raw}/verex-move/lakefile.toml is missing.")
        );
    }

    #[test]
    fn valid_checkout_and_symlink_return_canonical_path() {
        let temp = TempDir::new("valid");
        let checkout = temp.path.join("checkout");
        std::fs::create_dir_all(checkout.join("verex-ir")).unwrap();
        std::fs::write(checkout.join("verex-ir/lakefile.toml"), "").unwrap();
        std::fs::create_dir_all(checkout.join("verex-move")).unwrap();
        std::fs::write(checkout.join("verex-move/lakefile.toml"), "").unwrap();

        let expected = std::fs::canonicalize(&checkout).unwrap();
        assert_eq!(
            resolve_verex_root(Some(checkout.as_os_str())).unwrap(),
            expected
        );

        #[cfg(unix)]
        {
            let link = temp.path.join("link");
            std::os::unix::fs::symlink(&checkout, &link).unwrap();
            assert_eq!(
                resolve_verex_root(Some(link.as_os_str())).unwrap(),
                expected
            );
        }
    }
}
