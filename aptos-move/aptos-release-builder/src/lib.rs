// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod components;
mod utils;
pub mod validate;

pub use components::{ExecutionMode, ReleaseConfig, ReleaseEntry};
use once_cell::sync::{Lazy, OnceCell};
use std::{
    env,
    path::{Path, PathBuf},
};

// Update me after branch cut.
const RELEASE_CONFIG: &str = include_str!("../data/release.yaml");

static CURRENT_RELEASE_CONFIG: Lazy<ReleaseConfig> =
    Lazy::new(|| ReleaseConfig::parse(RELEASE_CONFIG).expect("YAML NOT PARSABLE"));

/// Returns the release bundle with which the last testnet was build or updated.
pub fn current_release_config() -> &'static ReleaseConfig {
    &CURRENT_RELEASE_CONFIG
}

static APTOS_CORE_PATH: OnceCell<PathBuf> = OnceCell::new();

fn aptos_core_path_at_compile_time() -> PathBuf {
    let mut path = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
    path.pop();
    path.pop();
    path = path.canonicalize().unwrap();
    path
}

pub fn initialize_aptos_core_path(overriden_path: Option<PathBuf>) {
    if let Some(path) = overriden_path {
        APTOS_CORE_PATH.set(path).unwrap();
    } else {
        APTOS_CORE_PATH
            .set(aptos_core_path_at_compile_time())
            .unwrap();
    };
}

pub(crate) fn aptos_core_path() -> PathBuf {
    APTOS_CORE_PATH
        .get_or_init(aptos_core_path_at_compile_time)
        .clone()
}

pub(crate) fn aptos_framework_path() -> PathBuf {
    let mut path = aptos_core_path();
    path.push("aptos-move/framework/aptos-framework");
    path
}
