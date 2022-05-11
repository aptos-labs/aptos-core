// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// A list of metrics which will be made public
pub const PUBLIC_METRICS: &[&str] = &["aptos_connections"];

pub const PUBLIC_JSON_METRICS: &[&str] = &[
    // git revision of the build
    "revision",
    // system info
    "system_total_memory",
    "system_used_memory",
    "system_physical_core_count",
];
