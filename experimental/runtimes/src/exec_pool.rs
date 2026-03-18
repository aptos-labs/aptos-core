// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::cpu_topology::CpuTopology;
use rayon::ThreadPool;

/// Builds a Rayon thread pool pinned to physical cores when topology is detectable.
/// Falls back to an unpinned pool with `fallback_threads` on non-Linux or detection failure.
pub fn build_pinned_exec_pool(thread_name_prefix: &str, fallback_threads: usize) -> ThreadPool {
    match CpuTopology::detect() {
        Some(topo) => build_pinned_pool(thread_name_prefix, fallback_threads, &topo),
        None => {
            aptos_logger::info!(
                "CPU topology detection unavailable, creating unpinned pool '{}' with {} threads",
                thread_name_prefix,
                fallback_threads,
            );
            build_unpinned_pool(thread_name_prefix, fallback_threads)
        },
    }
}

#[cfg(target_os = "linux")]
fn build_pinned_pool(
    thread_name_prefix: &str,
    fallback_threads: usize,
    topo: &CpuTopology,
) -> ThreadPool {
    use crate::common::{new_cpu_set, pin_cpu_set};
    use libc::CPU_SET;

    let num_threads = fallback_threads.min(topo.physical_core_ids.len());
    let selected_cores = topo.physical_cores_spread_across_ccx(num_threads);
    let actual_threads = selected_cores.len();

    aptos_logger::info!(
        "Creating pinned pool '{}' with {} threads on physical cores: {:?} ({} physical, {} HT, {} CCX groups)",
        thread_name_prefix,
        actual_threads,
        selected_cores,
        topo.physical_core_ids.len(),
        topo.ht_sibling_ids.len(),
        topo.ccx_groups.len(),
    );

    // Build per-thread cpu sets for 1:1 pinning
    let core_sets: Vec<_> = selected_cores
        .iter()
        .map(|&core_id| {
            let mut cpu_set = new_cpu_set();
            unsafe { CPU_SET(core_id, &mut cpu_set) };
            cpu_set
        })
        .collect();

    let core_sets = std::sync::Arc::new(core_sets);
    let prefix = thread_name_prefix.to_string();

    aptos_runtimes::spawn_rayon_thread_pool_with_start_hook(
        prefix,
        Some(actual_threads),
        move || {
            // Determine which thread index we are by thread name
            let thread = std::thread::current();
            let name = thread.name().unwrap_or("");
            // Thread names are formatted as "{prefix}-{index}"
            if let Some(idx_str) = name.rsplit('-').next() {
                if let Ok(idx) = idx_str.parse::<usize>() {
                    if idx < core_sets.len() {
                        pin_cpu_set(core_sets[idx])();
                    }
                }
            }
        },
    )
}

#[cfg(not(target_os = "linux"))]
fn build_pinned_pool(
    thread_name_prefix: &str,
    fallback_threads: usize,
    _topo: &CpuTopology,
) -> ThreadPool {
    // On non-Linux, topology detection returns None so this shouldn't be reached,
    // but handle gracefully.
    build_unpinned_pool(thread_name_prefix, fallback_threads)
}

fn build_unpinned_pool(thread_name_prefix: &str, num_threads: usize) -> ThreadPool {
    aptos_runtimes::spawn_rayon_thread_pool(thread_name_prefix.to_string(), Some(num_threads))
}
