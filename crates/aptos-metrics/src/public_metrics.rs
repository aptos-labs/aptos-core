// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// A list of metrics which will be made public
pub const PUBLIC_METRICS: &[&str] = &[
    // aptos metrics
    "aptos_connections",
    "aptos_state_sync_version",
    // binary metadata
    "revision",
    // system info
    "system_name",
    "system_kernel_version",
    "system_os_version",
    "system_total_memory",
    "system_used_memory",
    "system_physical_core_count",
];
