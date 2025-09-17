// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_coverage::coverage_map::{CoverageMap, ExecCoverageMap, ModuleCoverageMap};
use std::collections::BTreeSet;

pub mod oneshot;
pub mod sequence;
pub mod tracing;

pub(crate) fn count_coverage_entries(coverage: &ExecCoverageMap) -> usize {
    coverage
        .module_maps
        .values()
        .flat_map(|m| m.function_maps.values())
        .map(|f| f.len())
        .sum()
}

pub(crate) fn collect_coverage_keys(coverage: &ExecCoverageMap) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for (module, module_map) in &coverage.module_maps {
        for (function, entries) in &module_map.function_maps {
            for entry in entries {
                keys.insert(format!("{module:?}:{function}:{entry:?}"));
            }
        }
    }
    keys
}

pub(crate) fn coverage_delta(
    coverage: &ExecCoverageMap,
    coverage_at_last_report: &mut usize,
) -> usize {
    let current = count_coverage_entries(coverage);
    let delta = current.saturating_sub(*coverage_at_last_report);
    *coverage_at_last_report = current;
    delta
}

pub(crate) fn merge_coverage(coverage: &mut ExecCoverageMap, new_map: CoverageMap) -> bool {
    let mut found_new = false;
    for new_exec_map in new_map.exec_maps.into_values() {
        for (key, new_module_map) in new_exec_map.module_maps {
            let module_map = coverage.module_maps.entry(key).or_insert_with(|| {
                ModuleCoverageMap::new(new_module_map.module_addr, new_module_map.module_name)
            });
            for (ident, new_func_map) in new_module_map.function_maps {
                let func_map = module_map.function_maps.entry(ident.clone()).or_default();
                for (pos, count) in new_func_map {
                    if count == 0 {
                        continue;
                    }
                    if !func_map.contains_key(&pos) {
                        found_new = true;
                    }
                    let entry = func_map.entry(pos).or_insert(0);
                    *entry += count;
                }
            }
        }
    }
    found_new
}

pub(crate) fn clone_exec_coverage_map(coverage: &ExecCoverageMap) -> ExecCoverageMap {
    ExecCoverageMap {
        exec_id: coverage.exec_id.clone(),
        module_maps: coverage
            .module_maps
            .iter()
            .map(|(key, module_map)| {
                (key.clone(), ModuleCoverageMap {
                    module_addr: module_map.module_addr,
                    module_name: module_map.module_name.clone(),
                    function_maps: module_map.function_maps.clone(),
                })
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        clone_exec_coverage_map, collect_coverage_keys, count_coverage_entries, coverage_delta,
        merge_coverage,
    };
    use move_core_types::{account_address::AccountAddress, identifier::Identifier};
    use move_coverage::coverage_map::{CoverageMap, ExecCoverageMap};
    use std::collections::BTreeMap;

    fn ident(name: &str) -> Identifier {
        Identifier::new(name).unwrap()
    }

    fn coverage_map(exec_id: &str, entries: &[(u64, u64)]) -> CoverageMap {
        let mut exec = ExecCoverageMap::new(exec_id.to_string());
        for (pc, count) in entries {
            exec.insert_multi(AccountAddress::ONE, ident("m"), ident("f"), *pc, *count);
        }
        CoverageMap {
            exec_maps: BTreeMap::from([(exec_id.to_string(), exec)]),
        }
    }

    #[test]
    fn test_merge_coverage_accumulates_counts_and_discovers_new_entries() {
        let mut coverage = ExecCoverageMap::new("base".to_string());
        coverage.insert_multi(AccountAddress::ONE, ident("m"), ident("f"), 1, 2);

        let found_new = merge_coverage(&mut coverage, coverage_map("exec-1", &[(1, 3), (4, 7)]));
        assert!(found_new);
        assert_eq!(count_coverage_entries(&coverage), 2);

        let function_map = &coverage
            .module_maps
            .get(&(AccountAddress::ONE, ident("m")))
            .unwrap()
            .function_maps[&ident("f")];
        assert_eq!(function_map[&1], 5);
        assert_eq!(function_map[&4], 7);
    }

    #[test]
    fn test_merge_coverage_ignores_zero_count_entries() {
        let mut coverage = ExecCoverageMap::new("base".to_string());
        let found_new = merge_coverage(&mut coverage, coverage_map("exec-1", &[(9, 0)]));
        assert!(!found_new);
        assert_eq!(count_coverage_entries(&coverage), 0);
    }

    #[test]
    fn test_collect_coverage_keys_and_delta_tracking() {
        let mut coverage = ExecCoverageMap::new("base".to_string());
        coverage.insert(AccountAddress::ONE, ident("m"), ident("f"), 1);
        coverage.insert(AccountAddress::ONE, ident("m"), ident("f"), 7);

        let keys = collect_coverage_keys(&coverage);
        assert_eq!(keys.len(), 2);
        assert!(keys.iter().all(|key| key.contains("f")));

        let mut last_report = 1;
        assert_eq!(coverage_delta(&coverage, &mut last_report), 1);
        assert_eq!(last_report, 2);

        last_report = 5;
        assert_eq!(coverage_delta(&coverage, &mut last_report), 0);
        assert_eq!(last_report, 2);
    }

    #[test]
    fn test_clone_exec_coverage_map_preserves_contents() {
        let mut coverage = ExecCoverageMap::new("base".to_string());
        coverage.insert_multi(AccountAddress::ONE, ident("m"), ident("f"), 3, 9);
        let cloned = clone_exec_coverage_map(&coverage);

        assert_eq!(cloned.exec_id, "base");
        assert_eq!(count_coverage_entries(&cloned), 1);
        assert_eq!(
            cloned.module_maps[&(AccountAddress::ONE, ident("m"))].function_maps[&ident("f")][&3],
            9
        );
    }
}
