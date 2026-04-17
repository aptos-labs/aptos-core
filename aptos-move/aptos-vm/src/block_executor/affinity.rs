// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! CPU-affinity helpers for the par_exec thread pool.
//!
//! Detects the physical cores (as opposed to hyperthread siblings) that the
//! current process is allowed to run on, so the block executor can pin one
//! rayon worker per physical core. Pinning is only beneficial when we have at
//! least as many physical cores as worker threads; the caller is expected to
//! skip pinning otherwise.
//!
//! Linux only. On other platforms, or if the topology cannot be determined
//! (e.g. sysfs is not readable), this module returns `None` and the caller
//! should fall back to unpinned scheduling.

/// Returns one logical CPU id per distinct physical core that the current
/// process may run on, sorted ascending. Returns `None` if the topology
/// cannot be determined.
///
/// The returned ids are always a subset of `core_affinity::get_core_ids()`
/// so pinning to any of them is safe with respect to cgroups or an inherited
/// affinity mask (e.g. under `taskset`).
#[cfg(target_os = "linux")]
pub(super) fn allowed_physical_cores() -> Option<Vec<core_affinity::CoreId>> {
    let allowed = core_affinity::get_core_ids()?;
    let allowed_ids: Vec<usize> = allowed.iter().map(|c| c.id).collect();
    let ids = group_by_physical_core(&allowed_ids, read_thread_siblings)?;
    Some(
        ids.into_iter()
            .map(|id| core_affinity::CoreId { id })
            .collect(),
    )
}

#[cfg(not(target_os = "linux"))]
pub(super) fn allowed_physical_cores() -> Option<Vec<core_affinity::CoreId>> {
    None
}

/// Groups `allowed` CPU ids by the sibling list returned by `read_siblings`,
/// keeping the lowest id in each group as the representative for that
/// physical core. Returns `None` if `read_siblings` fails for any CPU.
///
/// Pure function — does not touch sysfs — to make the grouping logic unit
/// testable.
fn group_by_physical_core<F>(allowed: &[usize], mut read_siblings: F) -> Option<Vec<usize>>
where
    F: FnMut(usize) -> Option<String>,
{
    use std::collections::BTreeMap;

    let mut representatives: BTreeMap<String, usize> = BTreeMap::new();
    for &cpu in allowed {
        let siblings = read_siblings(cpu)?;
        representatives
            .entry(siblings)
            .and_modify(|cur| {
                if cpu < *cur {
                    *cur = cpu;
                }
            })
            .or_insert(cpu);
    }
    let mut ids: Vec<usize> = representatives.into_values().collect();
    ids.sort_unstable();
    Some(ids)
}

#[cfg(target_os = "linux")]
fn read_thread_siblings(cpu: usize) -> Option<String> {
    use aptos_logger::warn;

    let path = format!(
        "/sys/devices/system/cpu/cpu{}/topology/thread_siblings_list",
        cpu
    );
    match std::fs::read_to_string(&path) {
        Ok(s) => Some(s.trim().to_string()),
        Err(e) => {
            warn!(
                cpu = cpu,
                path = %path,
                error = %e,
                "Failed to read thread_siblings_list; cannot detect physical cores",
            );
            None
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_ht_siblings_into_one_physical_core() {
        // 4 logical CPUs, 2 physical cores with HT enabled: {0,2} and {1,3}.
        let result = group_by_physical_core(&[0, 1, 2, 3], |cpu| match cpu {
            0 | 2 => Some("0,2".to_string()),
            1 | 3 => Some("1,3".to_string()),
            _ => None,
        });
        assert_eq!(result, Some(vec![0, 1]));
    }

    #[test]
    fn no_ht_returns_all_cpus() {
        // 4 logical CPUs, HT disabled: each CPU is its own sibling.
        let result = group_by_physical_core(&[0, 1, 2, 3], |cpu| Some(cpu.to_string()));
        assert_eq!(result, Some(vec![0, 1, 2, 3]));
    }

    #[test]
    fn respects_restricted_affinity_set() {
        // The process may only run on CPUs {1,2}. Their sibling lists still
        // reference CPUs outside the allowed set, but we never consider those,
        // so the representatives come only from the allowed CPUs.
        let result = group_by_physical_core(&[1, 2], |cpu| match cpu {
            0 | 2 => Some("0,2".to_string()),
            1 | 3 => Some("1,3".to_string()),
            _ => None,
        });
        assert_eq!(result, Some(vec![1, 2]));
    }

    #[test]
    fn returns_none_when_sibling_read_fails() {
        let result = group_by_physical_core(&[0, 1], |_| None);
        assert_eq!(result, None);
    }

    #[test]
    fn empty_input_yields_empty_output() {
        let result = group_by_physical_core(&[], |_| Some(String::new()));
        assert_eq!(result, Some(Vec::new()));
    }
}
