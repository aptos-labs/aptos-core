// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use std::collections::BTreeMap;

/// CPU topology information distinguishing physical cores from HT siblings.
#[derive(Debug, Clone)]
pub struct CpuTopology {
    /// One CPU ID per physical core (lowest-numbered sibling in each group).
    pub physical_core_ids: Vec<usize>,
    /// The HT sibling of each physical core (if any).
    pub ht_sibling_ids: Vec<usize>,
    /// L3 cache ID -> list of physical core IDs in that CCX group.
    pub ccx_groups: BTreeMap<usize, Vec<usize>>,
}

impl CpuTopology {
    /// Detect CPU topology from Linux sysfs. Returns `None` on non-Linux or detection failure.
    #[cfg(target_os = "linux")]
    pub fn detect() -> Option<Self> {
        Self::detect_with_root("/sys")
    }

    #[cfg(not(target_os = "linux"))]
    pub fn detect() -> Option<Self> {
        None
    }

    /// Internal detection logic, parameterized by sysfs root for testability.
    #[cfg(target_os = "linux")]
    fn detect_with_root(sysfs_root: &str) -> Option<Self> {
        let available_cpus = Self::get_available_cpus(sysfs_root)?;
        if available_cpus.is_empty() {
            return None;
        }

        // Group CPUs by their thread_siblings_list to find physical cores
        let mut sibling_groups: BTreeMap<Vec<usize>, Vec<usize>> = BTreeMap::new();
        for &cpu in &available_cpus {
            let siblings_path = format!(
                "{}/devices/system/cpu/cpu{}/topology/thread_siblings_list",
                sysfs_root, cpu
            );
            let siblings = std::fs::read_to_string(&siblings_path).ok()?;
            let mut group = parse_cpu_list(siblings.trim());
            // Only keep siblings that are in our available set
            group.retain(|id| available_cpus.contains(id));
            group.sort();
            sibling_groups.entry(group).or_default().push(cpu);
        }

        let mut physical_core_ids = Vec::new();
        let mut ht_sibling_ids = Vec::new();
        for (group, _) in &sibling_groups {
            // Lowest-numbered in each sibling group is the physical core
            if let Some(&physical) = group.first() {
                physical_core_ids.push(physical);
                for &id in group.iter().skip(1) {
                    ht_sibling_ids.push(id);
                }
            }
        }
        physical_core_ids.sort();
        ht_sibling_ids.sort();

        // Build CCX groups from L3 cache IDs
        let mut ccx_groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for &cpu in &physical_core_ids {
            let cache_id_path = format!(
                "{}/devices/system/cpu/cpu{}/cache/index3/id",
                sysfs_root, cpu
            );
            if let Ok(contents) = std::fs::read_to_string(&cache_id_path) {
                if let Ok(cache_id) = contents.trim().parse::<usize>() {
                    ccx_groups.entry(cache_id).or_default().push(cpu);
                }
            }
        }

        // If no CCX info available, put all physical cores in one group
        if ccx_groups.is_empty() {
            ccx_groups.insert(0, physical_core_ids.clone());
        }

        Some(CpuTopology {
            physical_core_ids,
            ht_sibling_ids,
            ccx_groups,
        })
    }

    /// Get available CPUs from cgroup cpuset or fallback to online CPUs.
    #[cfg(target_os = "linux")]
    fn get_available_cpus(sysfs_root: &str) -> Option<Vec<usize>> {
        // Try cgroup v2 first
        let cgroup_v2 = std::fs::read_to_string("/sys/fs/cgroup/cpuset.cpus.effective");
        if let Ok(contents) = cgroup_v2 {
            let cpus = parse_cpu_list(contents.trim());
            if !cpus.is_empty() {
                return Some(cpus);
            }
        }

        // Try cgroup v1
        let cgroup_v1 = std::fs::read_to_string("/sys/fs/cgroup/cpuset/cpuset.cpus");
        if let Ok(contents) = cgroup_v1 {
            let cpus = parse_cpu_list(contents.trim());
            if !cpus.is_empty() {
                return Some(cpus);
            }
        }

        // Fallback to online CPUs
        let online_path = format!("{}/devices/system/cpu/online", sysfs_root);
        let contents = std::fs::read_to_string(&online_path).ok()?;
        let cpus = parse_cpu_list(contents.trim());
        if cpus.is_empty() {
            None
        } else {
            Some(cpus)
        }
    }

    /// Select `count` physical cores spread round-robin across CCX groups
    /// for maximum memory bandwidth.
    pub fn physical_cores_spread_across_ccx(&self, count: usize) -> Vec<usize> {
        let count = count.min(self.physical_core_ids.len());
        if self.ccx_groups.len() <= 1 {
            // Single CCX or no CCX info: just return first `count` physical cores
            return self.physical_core_ids[..count].to_vec();
        }

        // Round-robin across CCX groups
        let mut iterators: Vec<std::slice::Iter<usize>> = self
            .ccx_groups
            .values()
            .map(|cores| cores.iter())
            .collect();

        let mut result = Vec::with_capacity(count);
        let mut idx = 0;
        while result.len() < count {
            let iter_idx = idx % iterators.len();
            if let Some(&core_id) = iterators[iter_idx].next() {
                result.push(core_id);
            }
            idx += 1;
            // Safety: if we've exhausted all iterators, break
            if idx >= self.physical_core_ids.len() + iterators.len() {
                break;
            }
        }

        result
    }
}

/// Parse a CPU list string like "0-3,5,7-9" into a sorted Vec of CPU IDs.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn parse_cpu_list(s: &str) -> Vec<usize> {
    let mut result = Vec::new();
    if s.is_empty() {
        return result;
    }
    for part in s.split(',') {
        let part = part.trim();
        if let Some((start, end)) = part.split_once('-') {
            if let (Ok(start), Ok(end)) = (start.trim().parse::<usize>(), end.trim().parse::<usize>()) {
                for cpu in start..=end {
                    result.push(cpu);
                }
            }
        } else if let Ok(cpu) = part.parse::<usize>() {
            result.push(cpu);
        }
    }
    result.sort();
    result.dedup();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cpu_list_single() {
        assert_eq!(parse_cpu_list("0"), vec![0]);
        assert_eq!(parse_cpu_list("5"), vec![5]);
    }

    #[test]
    fn test_parse_cpu_list_range() {
        assert_eq!(parse_cpu_list("0-3"), vec![0, 1, 2, 3]);
        assert_eq!(parse_cpu_list("4-7"), vec![4, 5, 6, 7]);
    }

    #[test]
    fn test_parse_cpu_list_mixed() {
        assert_eq!(parse_cpu_list("0,24"), vec![0, 24]);
        assert_eq!(parse_cpu_list("0-3,5,7-9"), vec![0, 1, 2, 3, 5, 7, 8, 9]);
    }

    #[test]
    fn test_parse_cpu_list_cgroup_style() {
        assert_eq!(
            parse_cpu_list("0-23,48-71"),
            (0..=23).chain(48..=71).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_parse_cpu_list_empty() {
        assert_eq!(parse_cpu_list(""), Vec::<usize>::new());
    }

    #[test]
    fn test_ccx_spread_single_group() {
        let topo = CpuTopology {
            physical_core_ids: vec![0, 1, 2, 3],
            ht_sibling_ids: vec![4, 5, 6, 7],
            ccx_groups: BTreeMap::from([(0, vec![0, 1, 2, 3])]),
        };
        assert_eq!(topo.physical_cores_spread_across_ccx(2), vec![0, 1]);
        assert_eq!(topo.physical_cores_spread_across_ccx(4), vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_ccx_spread_multiple_groups() {
        let topo = CpuTopology {
            physical_core_ids: vec![0, 1, 2, 3, 4, 5],
            ht_sibling_ids: vec![6, 7, 8, 9, 10, 11],
            ccx_groups: BTreeMap::from([
                (0, vec![0, 1, 2]),
                (1, vec![3, 4, 5]),
            ]),
        };
        // Round-robin: pick from group 0, then group 1, etc.
        let spread = topo.physical_cores_spread_across_ccx(4);
        assert_eq!(spread.len(), 4);
        // Should alternate: 0 (ccx0), 3 (ccx1), 1 (ccx0), 4 (ccx1)
        assert_eq!(spread, vec![0, 3, 1, 4]);
    }

    #[test]
    fn test_ccx_spread_three_groups() {
        let topo = CpuTopology {
            physical_core_ids: vec![0, 1, 2, 3, 4, 5, 6, 7, 8],
            ht_sibling_ids: vec![],
            ccx_groups: BTreeMap::from([
                (0, vec![0, 1, 2]),
                (1, vec![3, 4, 5]),
                (2, vec![6, 7, 8]),
            ]),
        };
        let spread = topo.physical_cores_spread_across_ccx(6);
        assert_eq!(spread, vec![0, 3, 6, 1, 4, 7]);
    }

    #[test]
    fn test_ccx_spread_count_exceeds_cores() {
        let topo = CpuTopology {
            physical_core_ids: vec![0, 1],
            ht_sibling_ids: vec![2, 3],
            ccx_groups: BTreeMap::from([(0, vec![0, 1])]),
        };
        assert_eq!(topo.physical_cores_spread_across_ccx(10), vec![0, 1]);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_detect_on_linux() {
        // This test just verifies detect() doesn't panic on the current system.
        // It may return None in containers without sysfs.
        let _topo = CpuTopology::detect();
    }
}
