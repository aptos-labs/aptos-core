// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use once_cell::sync::Lazy;
use std::collections::HashMap;

static EMBEDDED_CONFIGS: Lazy<HashMap<&str, &str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    // Land blocking / docker-build-test
    m.insert("compat", include_str!("../config/compat.yaml"));
    m.insert(
        "framework_upgrade",
        include_str!("../config/framework_upgrade.yaml"),
    );
    m.insert(
        "realistic_env_max_load",
        include_str!("../config/realistic_env_max_load.yaml"),
    );
    m.insert(
        "land_blocking",
        include_str!("../config/realistic_env_max_load.yaml"),
    ); // alias
    m.insert(
        "realistic_env_max_load_large",
        include_str!("../config/realistic_env_max_load_large.yaml"),
    );
    m.insert(
        "consensus_only_realistic_env_max_tps",
        include_str!("../config/consensus_only_realistic_env_max_tps.yaml"),
    );
    m.insert(
        "multiregion_benchmark_test",
        include_str!("../config/multiregion_benchmark_test.yaml"),
    );
    // Forge stable
    m.insert(
        "realistic_env_load_sweep",
        include_str!("../config/realistic_env_load_sweep.yaml"),
    );
    m.insert(
        "realistic_env_workload_sweep",
        include_str!("../config/realistic_env_workload_sweep.yaml"),
    );
    m.insert(
        "realistic_env_orderbook_workload_sweep",
        include_str!("../config/realistic_env_orderbook_workload_sweep.yaml"),
    );
    m.insert(
        "realistic_env_graceful_overload",
        include_str!("../config/realistic_env_graceful_overload.yaml"),
    );
    m.insert(
        "realistic_env_graceful_workload_sweep",
        include_str!("../config/realistic_env_graceful_workload_sweep.yaml"),
    );
    m.insert(
        "realistic_env_fairness_workload_sweep",
        include_str!("../config/realistic_env_fairness_workload_sweep.yaml"),
    );
    m.insert(
        "realistic_network_tuned_for_throughput",
        include_str!("../config/realistic_network_tuned_for_throughput.yaml"),
    );
    m.insert(
        "consensus_stress_test",
        include_str!("../config/consensus_stress_test.yaml"),
    );
    m.insert("workload_mix", include_str!("../config/workload_mix.yaml"));
    m.insert(
        "single_vfn_perf",
        include_str!("../config/single_vfn_perf.yaml"),
    );
    m.insert(
        "fullnode_reboot_stress_test",
        include_str!("../config/fullnode_reboot_stress_test.yaml"),
    );
    m.insert(
        "changing_working_quorum_test",
        include_str!("../config/changing_working_quorum_test.yaml"),
    );
    m.insert(
        "changing_working_quorum_test_high_load",
        include_str!("../config/changing_working_quorum_test_high_load.yaml"),
    );
    m.insert(
        "pfn_const_tps_with_realistic_env",
        include_str!("../config/pfn_const_tps_with_realistic_env.yaml"),
    );
    m
});

/// Get an embedded YAML config by suite name
pub fn get(name: &str) -> Option<&'static str> {
    EMBEDDED_CONFIGS.get(name).copied()
}
