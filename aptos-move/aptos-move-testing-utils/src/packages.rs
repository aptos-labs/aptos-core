// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Aptos framework package management utilities.
//!
//! This module provides functionality for managing Aptos framework packages,
//! including detection, downloading from GitHub, and validation.

use anyhow::Result;
use aptos_framework::APTOS_PACKAGES;
use std::{
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::TempDir;

/// Directory names for Aptos framework packages.
pub const APTOS_PACKAGES_DIR_NAMES: [&str; 7] = [
    "aptos-framework",
    "move-stdlib",
    "aptos-stdlib",
    "aptos-token",
    "aptos-token-objects",
    "aptos-trading",
    "aptos-experimental",
];

/// Checks if a package name corresponds to an Aptos framework package.
///
/// # Arguments
/// * `package_name` - The name of the package to check
///
/// # Returns
/// `true` if the package is an Aptos framework package, `false` otherwise
///
/// # Example
/// ```
/// use aptos_move_testing_utils::is_aptos_package;
///
/// assert!(is_aptos_package("AptosFramework"));
/// assert!(!is_aptos_package("my-custom-package"));
/// ```
pub fn is_aptos_package(package_name: &str) -> bool {
    APTOS_PACKAGES.contains(&package_name)
}

/// Gets the directory name for an Aptos framework package.
///
/// # Arguments
/// * `package_name` - The name of the package
///
/// # Returns
/// `Some(directory_name)` if the package is an Aptos framework package, `None` otherwise
///
/// # Example
/// ```
/// use aptos_move_testing_utils::get_aptos_dir;
///
/// assert_eq!(get_aptos_dir("AptosFramework"), Some("aptos-framework"));
/// assert_eq!(get_aptos_dir("unknown"), None);
/// ```
pub fn get_aptos_dir(package_name: &str) -> Option<&str> {
    if is_aptos_package(package_name) {
        for i in 0..APTOS_PACKAGES.len() {
            if APTOS_PACKAGES[i] == package_name {
                return Some(APTOS_PACKAGES_DIR_NAMES[i]);
            }
        }
    }
    None
}

/// Downloads Aptos framework packages from GitHub.
///
/// This function clones the aptos-core repository from the specified branch
/// and copies the framework packages to the target path.
///
/// # Arguments
/// * `path` - The target directory to copy the framework packages to
/// * `branch_opt` - Optional branch name (defaults to "main")
///
/// # Returns
/// `Ok(())` on success, or an error if cloning or copying fails
///
/// # Example
/// ```no_run
/// use std::path::PathBuf;
/// use aptos_move_testing_utils::download_aptos_packages;
///
/// # async fn example() {
/// let target_path = PathBuf::from("/tmp/aptos-packages");
/// download_aptos_packages(&target_path, None).await.unwrap();
/// # }
/// ```
pub async fn download_aptos_packages(path: &Path, branch_opt: Option<String>) -> Result<()> {
    let git_url = "https://github.com/aptos-labs/aptos-core";
    let tmp_dir = TempDir::new()?;
    let branch = branch_opt.unwrap_or_else(|| "main".to_string());

    Command::new("git")
        .args([
            "clone",
            "--branch",
            &branch,
            git_url,
            tmp_dir.path().to_str().unwrap(),
            "--depth",
            "1",
        ])
        .output()
        .map_err(|_| anyhow::anyhow!("Failed to clone Git repository"))?;

    let source_framework_path = PathBuf::from(tmp_dir.path()).join("aptos-move/framework");
    for package_name in APTOS_PACKAGES {
        let source_framework_path =
            source_framework_path.join(get_aptos_dir(package_name).unwrap());
        let target_framework_path = PathBuf::from(path).join(get_aptos_dir(package_name).unwrap());
        Command::new("cp")
            .arg("-r")
            .arg(source_framework_path)
            .arg(target_framework_path)
            .output()
            .map_err(|_| anyhow::anyhow!("Failed to copy"))?;
    }

    Ok(())
}

/// Checks if all Aptos framework packages are available at the specified path.
///
/// # Arguments
/// * `path` - The directory to check for framework packages
///
/// # Returns
/// `true` if all framework packages exist, `false` otherwise
///
/// # Example
/// ```no_run
/// use std::path::PathBuf;
/// use aptos_move_testing_utils::check_aptos_packages_availability;
///
/// let packages_path = PathBuf::from("/tmp/aptos-packages");
/// if check_aptos_packages_availability(packages_path) {
///     println!("All packages are available");
/// }
/// ```
pub fn check_aptos_packages_availability(path: PathBuf) -> bool {
    if !path.exists() {
        return false;
    }
    for package in APTOS_PACKAGES {
        if !path.join(get_aptos_dir(package).unwrap()).exists() {
            return false;
        }
    }
    true
}

/// Prepares Aptos framework packages by downloading them if necessary.
///
/// This function checks if the packages already exist at the specified path.
/// If they don't exist or if `force_override_framework` is true, it downloads
/// them from GitHub.
///
/// # Arguments
/// * `path` - The target directory for framework packages
/// * `branch_opt` - Optional branch name to clone from (defaults to "main")
/// * `force_override_framework` - If true, re-downloads packages even if they exist
///
/// # Example
/// ```no_run
/// use std::path::PathBuf;
/// use aptos_move_testing_utils::prepare_aptos_packages;
///
/// # async fn example() {
/// let packages_path = PathBuf::from("/tmp/aptos-packages");
/// prepare_aptos_packages(packages_path, None, false).await;
/// # }
/// ```
pub async fn prepare_aptos_packages(
    path: PathBuf,
    branch_opt: Option<String>,
    force_override_framework: bool,
) {
    let mut download_flag = true;
    if path.exists() {
        if force_override_framework {
            std::fs::remove_dir_all(path.clone()).unwrap();
        } else {
            let mut need_download = false;
            for package_name in APTOS_PACKAGES {
                let target_framework_path = path.clone().join(get_aptos_dir(package_name).unwrap());
                if !target_framework_path.exists() {
                    need_download = true;
                    break;
                }
            }
            if need_download {
                std::fs::remove_dir_all(path.clone()).unwrap();
            } else {
                download_flag = false;
            }
        }
    }
    if download_flag {
        println!("Downloading aptos packages...");
        std::fs::create_dir_all(path.clone()).unwrap();
        download_aptos_packages(&path, branch_opt).await.unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_aptos_package() {
        assert!(is_aptos_package("AptosFramework"));
        assert!(is_aptos_package("MoveStdlib"));
        assert!(is_aptos_package("AptosStdlib"));
        assert!(is_aptos_package("AptosToken"));
        assert!(is_aptos_package("AptosTokenObjects"));
        assert!(!is_aptos_package("custom-package"));
        assert!(!is_aptos_package("unknown"));
    }

    #[test]
    fn test_get_aptos_dir() {
        assert_eq!(get_aptos_dir("AptosFramework"), Some("aptos-framework"));
        assert_eq!(get_aptos_dir("MoveStdlib"), Some("move-stdlib"));
        assert_eq!(get_aptos_dir("AptosStdlib"), Some("aptos-stdlib"));
        assert_eq!(get_aptos_dir("unknown"), None);
    }

    #[test]
    fn test_check_aptos_packages_availability_nonexistent_path() {
        let path = PathBuf::from("/tmp/nonexistent-aptos-packages-test-12345");
        assert!(!check_aptos_packages_availability(path));
    }
}
