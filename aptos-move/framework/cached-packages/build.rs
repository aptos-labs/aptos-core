// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::{Context, Result};
use aptos_framework::ReleaseTarget;
use sha2::{Digest, Sha256};
use std::{
    env::current_dir,
    fs,
    io::Read,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

/// Directories containing Move source files that affect the framework build.
const FRAMEWORK_DIRS: &[&str] = &[
    "aptos-experimental",
    "aptos-trading",
    "aptos-token-objects",
    "aptos-token",
    "aptos-framework",
    "aptos-stdlib",
    "move-stdlib",
];

/// Compute a SHA256 hash of all inputs that affect the framework build:
/// - All .move files and Move.toml files in framework directories
/// - Compiler crate versions (via DEP_* environment variables from their build.rs)
fn compute_framework_hash(framework_root: &Path) -> Result<String> {
    let mut hasher = Sha256::new();
    let mut file_count = 0;

    // Hash Move source files
    for dir_name in FRAMEWORK_DIRS {
        let dir_path = framework_root.join(dir_name);
        if !dir_path.exists() {
            continue;
        }

        // Collect all relevant files and sort them for deterministic hashing
        let mut files: Vec<PathBuf> = WalkDir::new(&dir_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let path = e.path();
                path.is_file()
                    && (path.extension().is_some_and(|ext| ext == "move")
                        || path.file_name().is_some_and(|name| name == "Move.toml"))
            })
            .map(|e| e.path().to_path_buf())
            .collect();

        // Sort for deterministic ordering
        files.sort();

        for file_path in files {
            // Hash the relative path to detect renames/moves
            let relative_path = file_path.strip_prefix(framework_root).unwrap_or(&file_path);
            hasher.update(relative_path.to_string_lossy().as_bytes());
            hasher.update(b"\0");

            // Hash the file contents
            let mut file = fs::File::open(&file_path)
                .with_context(|| format!("Failed to open {:?}", file_path))?;
            let mut contents = Vec::new();
            file.read_to_end(&mut contents)
                .with_context(|| format!("Failed to read {:?}", file_path))?;
            hasher.update(&contents);
            hasher.update(b"\0");

            file_count += 1;
        }
    }

    // Include move-package build marker to detect compiler changes.
    // When move-package rebuilds, it writes a new timestamp to this marker file.
    let marker_path = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            framework_root
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("target")
        })
        .join(".move_package_build_marker");

    if let Ok(marker_content) = fs::read_to_string(&marker_path) {
        hasher.update(b"move-package-build:");
        hasher.update(marker_content.trim().as_bytes());
        hasher.update(b"\0");
    }

    // Include file count in hash to detect additions/deletions
    hasher.update(file_count.to_string().as_bytes());

    let hash = hasher.finalize();
    Ok(format!("{:x}", hash))
}

/// Get a stable cache directory that persists across OUT_DIR changes.
fn get_cache_dir(workspace_root: &Path) -> PathBuf {
    // Use CARGO_TARGET_DIR if set, otherwise default to workspace_root/target
    std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| workspace_root.join("target"))
        .join(".framework_cache")
}

/// Check if cached artifacts are valid and can be reused.
/// Returns Some(cached_mrb_path) if cache hit, None if rebuild needed.
fn check_cache(cache_dir: &Path, current_hash: &str) -> Option<PathBuf> {
    let hash_file = cache_dir.join("hash.txt");
    let cached_mrb = cache_dir.join("head.mrb");

    // Both files must exist
    if !hash_file.exists() || !cached_mrb.exists() {
        return None;
    }

    // Hash must match
    match fs::read_to_string(&hash_file) {
        Ok(cached_hash) if cached_hash.trim() == current_hash => Some(cached_mrb),
        _ => None,
    }
}

/// Emit rerun-if-changed directives for all framework source directories.
fn emit_rerun_if_changed(framework_root: &Path) {
    // Always rerun if build.rs changes
    println!("cargo:rerun-if-changed=build.rs");

    for dir_name in FRAMEWORK_DIRS {
        let sources_dir = framework_root.join(dir_name).join("sources");
        let move_toml = framework_root.join(dir_name).join("Move.toml");

        if sources_dir.exists() {
            println!("cargo:rerun-if-changed={}", sources_dir.display());
        }
        if move_toml.exists() {
            println!("cargo:rerun-if-changed={}", move_toml.display());
        }
    }

    // Compiler crate changes are tracked via a marker file written by
    // move-package's build.rs. When move-package rebuilds, it writes a new
    // timestamp to target/.move_package_build_marker, which changes the hash.
}

fn main() -> Result<()> {
    // Set the below variable to skip the building step. This might be useful if the build
    // is broken so it can be debugged with the old outdated artifacts.
    if std::env::var("SKIP_FRAMEWORK_BUILD").is_ok() {
        println!("cargo:warning=SKIP_FRAMEWORK_BUILD is set, skipping framework build.");
        return Ok(());
    }

    let current_dir = current_dir().expect("Should be able to get current dir");
    // Get the framework root directory (parent of cached-packages)
    let mut framework_root = current_dir;
    framework_root.pop();

    // Get workspace root (aptos-core)
    let mut workspace_root = framework_root.clone();
    workspace_root.pop(); // aptos-move/framework -> aptos-move
    workspace_root.pop(); // aptos-move -> aptos-core

    // Emit rerun-if-changed directives
    emit_rerun_if_changed(&framework_root);

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR defined"));
    let output_path = out_dir.join("head.mrb");
    let cache_dir = get_cache_dir(&workspace_root);

    // Compute hash of all inputs (Move sources + compiler crates)
    let current_hash =
        compute_framework_hash(&framework_root).context("Failed to compute framework hash")?;

    // Check if we have a valid cached build
    if let Some(cached_mrb) = check_cache(&cache_dir, &current_hash) {
        // Copy cached artifact to OUT_DIR
        if output_path.exists() {
            // Already have the output, nothing to do
            println!("cargo:warning=Framework sources unchanged, skipping rebuild.");
            return Ok(());
        }

        println!("cargo:warning=Framework sources unchanged, copying from cache...");
        fs::copy(&cached_mrb, &output_path).context("Failed to copy cached framework")?;
        return Ok(());
    }

    // Build the framework
    println!("cargo:warning=Building Move framework (this may take a few minutes)...");
    ReleaseTarget::Head
        .create_release(true, Some(output_path.clone()))
        .context("Failed to create release")?;

    // Cache the result
    fs::create_dir_all(&cache_dir).context("Failed to create cache directory")?;
    fs::copy(&output_path, cache_dir.join("head.mrb")).context("Failed to cache framework")?;
    fs::write(cache_dir.join("hash.txt"), &current_hash).context("Failed to write hash file")?;

    println!("cargo:warning=Framework build complete and cached.");

    Ok(())
}
