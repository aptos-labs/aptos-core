// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use sysinfo::{System, SystemExt};

// Use to expose non-numeric metrics
pub fn get_json_metrics() -> HashMap<String, String> {
    let mut json_metrics: HashMap<String, String> = HashMap::new();
    json_metrics = add_revision_hash(json_metrics);
    json_metrics = add_system_info(json_metrics);
    json_metrics
}

fn add_revision_hash(mut json_metrics: HashMap<String, String>) -> HashMap<String, String> {
    json_metrics.insert("revision".to_string(), env!("GIT_REV").to_string());
    json_metrics
}

fn add_system_info(mut json_metrics: HashMap<String, String>) -> HashMap<String, String> {
    let mut sys = System::new();
    sys.refresh_system();

    if let Some(name) = sys.name() {
        json_metrics.insert("system_name".to_string(), name);
    }
    if let Some(kernel_version) = sys.kernel_version() {
        json_metrics.insert("system_kernel_version".to_string(), kernel_version);
    }
    if let Some(os_version) = sys.os_version() {
        json_metrics.insert("system_os_version".to_string(), os_version);
    }

    json_metrics
}

pub fn get_git_rev() -> String {
    env!("GIT_REV").to_string()
}
