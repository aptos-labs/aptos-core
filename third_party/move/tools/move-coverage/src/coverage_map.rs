// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use anyhow::{format_err, Context, Result};
use move_binary_format::file_format::{CodeOffset, CompiledModule};
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{btree_map::Entry, BTreeMap},
    fs::File,
    io::{BufRead, BufReader, Read, Write},
    path::Path,
};

/// Map from code offset in a function to the number of times it was executed.
pub type FunctionCoverage = BTreeMap<u64, u64>;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CoverageMap {
    pub exec_maps: BTreeMap<String, ExecCoverageMap>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleCoverageMap {
    pub module_addr: AccountAddress,
    pub module_name: Identifier,
    pub function_maps: BTreeMap<Identifier, FunctionCoverage>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecCoverageMap {
    pub exec_id: String,
    pub module_maps: BTreeMap<(AccountAddress, Identifier), ModuleCoverageMap>,
}

#[derive(Debug)]
pub struct ExecCoverageMapWithModules {
    pub module_maps: BTreeMap<(String, AccountAddress, Identifier), ModuleCoverageMap>,
    pub compiled_modules: BTreeMap<String, CompiledModule>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TraceEntry {
    pub module_addr: AccountAddress,
    pub module_name: Identifier,
    pub func_name: Identifier,
    pub func_pc: CodeOffset,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TraceMap {
    pub exec_maps: BTreeMap<String, Vec<TraceEntry>>,
}

impl CoverageMap {
    /// Takes in a file containing a raw VM trace, and returns an updated coverage map.
    pub fn update_coverage_from_trace_file<P: AsRef<Path> + std::fmt::Debug>(
        mut self,
        filename: &P,
    ) -> Result<Self> {
        let file = File::open(filename)
            .map_err(|e| format_err!("Unable to open coverage trace file {:?}: {}", filename, e))?;
        for line in BufReader::new(file).lines() {
            let line = line?;
            let mut splits = line.split(',');
            // Use a dummy key so that the data structure of the coverage map does not need to be changed
            let exec_id = "dummy_exec_id";
            let context = splits.next().unwrap();
            let pc = splits.next().unwrap().parse::<u64>().unwrap();

            let mut context_segs: Vec<_> = context.split("::").collect();
            let is_script = context_segs.len() == 2;
            if !is_script {
                let func_name = Identifier::new(context_segs.pop().unwrap()).unwrap();
                let module_name = Identifier::new(context_segs.pop().unwrap()).unwrap();
                let module_addr =
                    AccountAddress::from_hex_literal(context_segs.pop().unwrap()).unwrap();
                self.insert(exec_id, module_addr, module_name, func_name, pc);
            } else {
                // Don't count scripts (for now)
                assert_eq!(context_segs.pop().unwrap(), "main",);
                assert_eq!(context_segs.pop().unwrap(), "Script",);
            }
        }
        Ok(self)
    }

    /// Takes in a file containing a raw VM trace, and returns a coverage map.
    pub fn from_trace_file<P: AsRef<Path> + std::fmt::Debug>(filename: &P) -> Result<Self> {
        let empty_module_map = CoverageMap {
            exec_maps: BTreeMap::new(),
        };
        empty_module_map
            .update_coverage_from_trace_file(filename)
            .with_context(|| format!("Updating coverage from trace file {:?}", filename))
    }

    /// Takes in a file containing a serialized coverage map and returns a coverage map.
    pub fn from_binary_file<P: AsRef<Path> + std::fmt::Debug>(filename: &P) -> Result<Self> {
        let mut bytes = Vec::new();
        File::open(filename)
            .map_err(|e| format_err!("{}: Coverage map file '{:?}' doesn't exist", e, filename))?
            .read_to_end(&mut bytes)
            .ok()
            .ok_or_else(|| format_err!("Unable to read coverage map"))?;
        bcs::from_bytes(&bytes)
            .with_context(|| format!("Deserializing coverage map from binary file {:?}", filename))
    }

    // add entries in a cascading manner
    pub fn insert(
        &mut self,
        exec_id: &str,
        module_addr: AccountAddress,
        module_name: Identifier,
        func_name: Identifier,
        pc: u64,
    ) {
        let exec_entry = self
            .exec_maps
            .entry(exec_id.to_owned())
            .or_insert_with(|| ExecCoverageMap::new(exec_id.to_owned()));
        exec_entry.insert(module_addr, module_name, func_name, pc);
    }

    pub fn to_unified_exec_map(&self) -> ExecCoverageMap {
        let mut unified_map = ExecCoverageMap::new(String::new());
        for (_, exec_map) in self.exec_maps.iter() {
            for ((module_addr, module_name), module_map) in exec_map.module_maps.iter() {
                for (func_name, func_map) in module_map.function_maps.iter() {
                    for (pc, count) in func_map.iter() {
                        unified_map.insert_multi(
                            *module_addr,
                            module_name.clone(),
                            func_name.clone(),
                            *pc,
                            *count,
                        );
                    }
                }
            }
        }
        unified_map
    }

    pub fn merge(&mut self, other: Self) {
        for (id, exec_map) in other.exec_maps {
            match self.exec_maps.entry(id) {
                Entry::Vacant(e) => {
                    e.insert(exec_map);
                },
                Entry::Occupied(mut e) => e.get_mut().merge(exec_map),
            }
        }
    }
}

impl ModuleCoverageMap {
    pub fn new(module_addr: AccountAddress, module_name: Identifier) -> Self {
        ModuleCoverageMap {
            module_addr,
            module_name,
            function_maps: BTreeMap::new(),
        }
    }

    pub fn insert_multi(&mut self, func_name: Identifier, pc: u64, count: u64) {
        let func_entry = self.function_maps.entry(func_name).or_default();
        let pc_entry = func_entry.entry(pc).or_insert(0);
        *pc_entry += count;
    }

    pub fn insert(&mut self, func_name: Identifier, pc: u64) {
        self.insert_multi(func_name, pc, 1);
    }

    pub fn merge(&mut self, another: ModuleCoverageMap) {
        for (key, val) in another.function_maps {
            match self.function_maps.entry(key) {
                Entry::Vacant(e) => {
                    e.insert(val);
                },
                Entry::Occupied(mut e) => {
                    for (pc, count) in val {
                        match e.get_mut().entry(pc) {
                            Entry::Vacant(c) => {
                                c.insert(count);
                            },
                            Entry::Occupied(mut c) => *c.get_mut() += count,
                        }
                    }
                },
            }
        }
    }

    pub fn get_function_coverage(&self, func_name: &IdentStr) -> Option<&FunctionCoverage> {
        self.function_maps.get(func_name)
    }
}

impl ExecCoverageMap {
    pub fn new(exec_id: String) -> Self {
        ExecCoverageMap {
            exec_id,
            module_maps: BTreeMap::new(),
        }
    }

    pub fn insert_multi(
        &mut self,
        module_addr: AccountAddress,
        module_name: Identifier,
        func_name: Identifier,
        pc: u64,
        count: u64,
    ) {
        let module_entry = self
            .module_maps
            .entry((module_addr, module_name.clone()))
            .or_insert_with(|| ModuleCoverageMap::new(module_addr, module_name));
        module_entry.insert_multi(func_name, pc, count);
    }

    pub fn insert(
        &mut self,
        module_addr: AccountAddress,
        module_name: Identifier,
        func_name: Identifier,
        pc: u64,
    ) {
        self.insert_multi(module_addr, module_name, func_name, pc, 1);
    }

    pub fn merge(&mut self, other: ExecCoverageMap) {
        for (mod_id, mod_map) in other.module_maps {
            match self.module_maps.entry(mod_id) {
                Entry::Vacant(e) => {
                    e.insert(mod_map);
                },
                Entry::Occupied(mut e) => e.get_mut().merge(mod_map),
            }
        }
    }

    pub fn into_coverage_map_with_modules(
        self,
        modules: BTreeMap<AccountAddress, BTreeMap<Identifier, (String, CompiledModule)>>,
    ) -> ExecCoverageMapWithModules {
        let retained: BTreeMap<(String, AccountAddress, Identifier), ModuleCoverageMap> = self
            .module_maps
            .into_iter()
            .filter_map(|((module_addr, module_name), module_cov)| {
                modules.get(&module_addr).and_then(|func_map| {
                    func_map.get(&module_name).map(|(module_path, _)| {
                        ((module_path.clone(), module_addr, module_name), module_cov)
                    })
                })
            })
            .collect();

        let compiled_modules = modules
            .into_iter()
            .flat_map(|(_, module_map)| {
                module_map
                    .into_iter()
                    .map(|(_, (module_path, compiled_module))| (module_path, compiled_module))
            })
            .collect();

        ExecCoverageMapWithModules {
            module_maps: retained,
            compiled_modules,
        }
    }
}

impl ExecCoverageMapWithModules {
    pub fn empty() -> Self {
        Self {
            module_maps: BTreeMap::new(),
            compiled_modules: BTreeMap::new(),
        }
    }

    pub fn merge(&mut self, another: ExecCoverageMapWithModules) {
        for ((module_path, module_addr, module_name), val) in another.module_maps {
            self.module_maps
                .entry((module_path.clone(), module_addr, module_name.clone()))
                .or_insert_with(|| ModuleCoverageMap::new(module_addr, module_name))
                .merge(val);
        }

        for (module_path, compiled_module) in another.compiled_modules {
            self.compiled_modules
                .entry(module_path)
                .or_insert(compiled_module);
        }
    }
}

impl TraceMap {
    /// Takes in a file containing a raw VM trace, and returns an updated coverage map.
    pub fn update_from_trace_file<P: AsRef<Path> + std::fmt::Debug>(
        mut self,
        filename: &P,
    ) -> Result<Self> {
        let file = File::open(filename)?;
        for line in BufReader::new(file).lines() {
            let line = line?;
            let mut splits = line.split(',');
            // Use a dummy key so that the data structure of the coverage map does not need to be changed
            let exec_id = "dummy_exec_id";
            let context = splits.next().unwrap();
            let pc = splits.next().unwrap().parse::<u64>().unwrap();

            let mut context_segs: Vec<_> = context.split("::").collect();
            let is_script = context_segs.len() == 2;
            if !is_script {
                let func_name = Identifier::new(context_segs.pop().unwrap()).unwrap();
                let module_name = Identifier::new(context_segs.pop().unwrap()).unwrap();
                let module_addr =
                    AccountAddress::from_hex_literal(context_segs.pop().unwrap()).unwrap();
                self.insert(exec_id, module_addr, module_name, func_name, pc);
            } else {
                // Don't count scripts (for now)
                assert_eq!(context_segs.pop().unwrap(), "main",);
                assert_eq!(context_segs.pop().unwrap(), "Script",);
            }
        }
        Ok(self)
    }

    // Takes in a file containing a raw VM trace, and returns a parsed trace.
    pub fn from_trace_file<P: AsRef<Path> + std::fmt::Debug>(filename: &P) -> Result<Self> {
        let trace_map = TraceMap {
            exec_maps: BTreeMap::new(),
        };
        trace_map.update_from_trace_file(filename)
    }

    // Takes in a file containing a serialized trace and deserialize it.
    pub fn from_binary_file<P: AsRef<Path> + std::fmt::Debug>(filename: &P) -> Result<Self> {
        let mut bytes = Vec::new();
        File::open(filename)
            .ok()
            .and_then(|mut file| file.read_to_end(&mut bytes).ok())
            .ok_or_else(|| format_err!("Error while reading in coverage map binary"))?;
        bcs::from_bytes(&bytes)
            .with_context(|| format!("Deserializing {:?} into coverage map", filename))
    }

    // add entries in a cascading manner
    pub fn insert(
        &mut self,
        exec_id: &str,
        module_addr: AccountAddress,
        module_name: Identifier,
        func_name: Identifier,
        pc: u64,
    ) {
        let exec_entry = self.exec_maps.entry(exec_id.to_owned()).or_default();
        exec_entry.push(TraceEntry {
            module_addr,
            module_name,
            func_name,
            func_pc: pc as CodeOffset,
        });
    }
}

pub fn output_map_to_file<M: Serialize, P: AsRef<Path> + std::fmt::Debug>(
    file_name: &P,
    data: &M,
) -> Result<()> {
    let bytes = bcs::to_bytes(data)?;
    let mut file = File::create(file_name)?;
    file.write_all(&bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn test_addr() -> AccountAddress {
        AccountAddress::from_hex_literal("0x1").unwrap()
    }

    fn test_addr_2() -> AccountAddress {
        AccountAddress::from_hex_literal("0x2").unwrap()
    }

    #[test]
    fn test_coverage_map_insert() {
        let mut coverage_map = CoverageMap::default();
        let addr = test_addr();
        let module_name = Identifier::new("TestModule").unwrap();
        let func_name = Identifier::new("test_func").unwrap();

        coverage_map.insert("exec1", addr, module_name.clone(), func_name.clone(), 0);
        coverage_map.insert("exec1", addr, module_name.clone(), func_name.clone(), 1);
        coverage_map.insert("exec1", addr, module_name.clone(), func_name.clone(), 0);

        let exec_map = coverage_map.exec_maps.get("exec1").unwrap();
        let module_map = exec_map.module_maps.get(&(addr, module_name)).unwrap();
        let func_cov = module_map.function_maps.get(&func_name).unwrap();

        assert_eq!(func_cov.get(&0), Some(&2)); // PC 0 was hit twice
        assert_eq!(func_cov.get(&1), Some(&1)); // PC 1 was hit once
    }

    #[test]
    fn test_coverage_map_merge() {
        let mut map1 = CoverageMap::default();
        let mut map2 = CoverageMap::default();
        let addr = test_addr();
        let module_name = Identifier::new("TestModule").unwrap();
        let func_name = Identifier::new("test_func").unwrap();

        // Insert into map1
        map1.insert("exec1", addr, module_name.clone(), func_name.clone(), 0);
        map1.insert("exec1", addr, module_name.clone(), func_name.clone(), 1);

        // Insert into map2 with same exec_id
        map2.insert("exec1", addr, module_name.clone(), func_name.clone(), 0);
        map2.insert("exec1", addr, module_name.clone(), func_name.clone(), 2);

        // Insert into map2 with different exec_id
        map2.insert("exec2", addr, module_name.clone(), func_name.clone(), 5);

        map1.merge(map2);

        // Check exec1 was merged
        let exec_map = map1.exec_maps.get("exec1").unwrap();
        let module_map = exec_map.module_maps.get(&(addr, module_name.clone())).unwrap();
        let func_cov = module_map.function_maps.get(&func_name).unwrap();

        assert_eq!(func_cov.get(&0), Some(&2)); // PC 0 was in both maps
        assert_eq!(func_cov.get(&1), Some(&1)); // PC 1 was only in map1
        assert_eq!(func_cov.get(&2), Some(&1)); // PC 2 was only in map2

        // Check exec2 was added
        let exec_map2 = map1.exec_maps.get("exec2").unwrap();
        let module_map2 = exec_map2.module_maps.get(&(addr, module_name)).unwrap();
        let func_cov2 = module_map2.function_maps.get(&func_name).unwrap();
        assert_eq!(func_cov2.get(&5), Some(&1));
    }

    #[test]
    fn test_to_unified_exec_map() {
        let mut coverage_map = CoverageMap::default();
        let addr = test_addr();
        let module_name = Identifier::new("TestModule").unwrap();
        let func_name = Identifier::new("test_func").unwrap();

        // Insert across multiple exec_ids
        coverage_map.insert("exec1", addr, module_name.clone(), func_name.clone(), 0);
        coverage_map.insert("exec1", addr, module_name.clone(), func_name.clone(), 0);
        coverage_map.insert("exec2", addr, module_name.clone(), func_name.clone(), 0);
        coverage_map.insert("exec2", addr, module_name.clone(), func_name.clone(), 1);

        let unified = coverage_map.to_unified_exec_map();
        let module_map = unified.module_maps.get(&(addr, module_name)).unwrap();
        let func_cov = module_map.function_maps.get(&func_name).unwrap();

        // PC 0: 2 from exec1 + 1 from exec2 = 3
        assert_eq!(func_cov.get(&0), Some(&3));
        // PC 1: 1 from exec2 = 1
        assert_eq!(func_cov.get(&1), Some(&1));
    }

    #[test]
    fn test_module_coverage_map_merge() {
        let addr = test_addr();
        let module_name = Identifier::new("TestModule").unwrap();
        let func1 = Identifier::new("func1").unwrap();
        let func2 = Identifier::new("func2").unwrap();

        let mut map1 = ModuleCoverageMap::new(addr, module_name.clone());
        map1.insert(func1.clone(), 0);
        map1.insert(func1.clone(), 1);

        let mut map2 = ModuleCoverageMap::new(addr, module_name.clone());
        map2.insert(func1.clone(), 0);
        map2.insert(func2.clone(), 0);

        map1.merge(map2);

        let func1_cov = map1.function_maps.get(&func1).unwrap();
        assert_eq!(func1_cov.get(&0), Some(&2));
        assert_eq!(func1_cov.get(&1), Some(&1));

        let func2_cov = map1.function_maps.get(&func2).unwrap();
        assert_eq!(func2_cov.get(&0), Some(&1));
    }

    #[test]
    fn test_exec_coverage_map_merge_different_modules() {
        let addr1 = test_addr();
        let addr2 = test_addr_2();
        let module1 = Identifier::new("Module1").unwrap();
        let module2 = Identifier::new("Module2").unwrap();
        let func = Identifier::new("func").unwrap();

        let mut exec_map1 = ExecCoverageMap::new("test".to_string());
        exec_map1.insert(addr1, module1.clone(), func.clone(), 0);

        let mut exec_map2 = ExecCoverageMap::new("test".to_string());
        exec_map2.insert(addr2, module2.clone(), func.clone(), 0);

        exec_map1.merge(exec_map2);

        assert!(exec_map1.module_maps.contains_key(&(addr1, module1)));
        assert!(exec_map1.module_maps.contains_key(&(addr2, module2)));
    }

    #[test]
    fn test_coverage_map_from_trace_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "0x1::TestModule::test_func,0").unwrap();
        writeln!(temp_file, "0x1::TestModule::test_func,1").unwrap();
        writeln!(temp_file, "0x1::TestModule::test_func,0").unwrap();
        writeln!(temp_file, "0x1::TestModule::other_func,5").unwrap();
        temp_file.flush().unwrap();

        let coverage_map = CoverageMap::from_trace_file(&temp_file.path()).unwrap();
        let unified = coverage_map.to_unified_exec_map();

        let addr = test_addr();
        let module_name = Identifier::new("TestModule").unwrap();
        let module_map = unified.module_maps.get(&(addr, module_name)).unwrap();

        let func1 = Identifier::new("test_func").unwrap();
        let func1_cov = module_map.function_maps.get(&func1).unwrap();
        assert_eq!(func1_cov.get(&0), Some(&2));
        assert_eq!(func1_cov.get(&1), Some(&1));

        let func2 = Identifier::new("other_func").unwrap();
        let func2_cov = module_map.function_maps.get(&func2).unwrap();
        assert_eq!(func2_cov.get(&5), Some(&1));
    }

    #[test]
    fn test_trace_file_ignores_scripts() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "0x1::TestModule::test_func,0").unwrap();
        writeln!(temp_file, "Script::main,0").unwrap();
        writeln!(temp_file, "0x1::TestModule::test_func,1").unwrap();
        temp_file.flush().unwrap();

        let coverage_map = CoverageMap::from_trace_file(&temp_file.path()).unwrap();
        let unified = coverage_map.to_unified_exec_map();

        // Should only have one module (no Script)
        assert_eq!(unified.module_maps.len(), 1);

        let addr = test_addr();
        let module_name = Identifier::new("TestModule").unwrap();
        assert!(unified.module_maps.contains_key(&(addr, module_name)));
    }

    #[test]
    fn test_trace_map_from_trace_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "0x1::TestModule::test_func,0").unwrap();
        writeln!(temp_file, "0x1::TestModule::test_func,1").unwrap();
        writeln!(temp_file, "0x1::TestModule::test_func,2").unwrap();
        temp_file.flush().unwrap();

        let trace_map = TraceMap::from_trace_file(&temp_file.path()).unwrap();
        let traces = trace_map.exec_maps.get("dummy_exec_id").unwrap();

        assert_eq!(traces.len(), 3);
        assert_eq!(traces[0].func_pc, 0);
        assert_eq!(traces[1].func_pc, 1);
        assert_eq!(traces[2].func_pc, 2);
    }

    #[test]
    fn test_binary_serialization_roundtrip() {
        let mut coverage_map = CoverageMap::default();
        let addr = test_addr();
        let module_name = Identifier::new("TestModule").unwrap();
        let func_name = Identifier::new("test_func").unwrap();

        coverage_map.insert("exec1", addr, module_name.clone(), func_name.clone(), 0);
        coverage_map.insert("exec1", addr, module_name.clone(), func_name.clone(), 1);

        let temp_file = NamedTempFile::new().unwrap();
        output_map_to_file(&temp_file.path(), &coverage_map).unwrap();

        let loaded = CoverageMap::from_binary_file(&temp_file.path()).unwrap();
        let exec_map = loaded.exec_maps.get("exec1").unwrap();
        let module_map = exec_map.module_maps.get(&(addr, module_name)).unwrap();
        let func_cov = module_map.function_maps.get(&func_name).unwrap();

        assert_eq!(func_cov.get(&0), Some(&1));
        assert_eq!(func_cov.get(&1), Some(&1));
    }

    #[test]
    fn test_exec_coverage_map_with_modules_merge() {
        let mut map1 = ExecCoverageMapWithModules::empty();
        let mut map2 = ExecCoverageMapWithModules::empty();

        let addr = test_addr();
        let module_name = Identifier::new("TestModule").unwrap();
        let func_name = Identifier::new("test_func").unwrap();

        let mut module_cov1 = ModuleCoverageMap::new(addr, module_name.clone());
        module_cov1.insert(func_name.clone(), 0);

        let mut module_cov2 = ModuleCoverageMap::new(addr, module_name.clone());
        module_cov2.insert(func_name.clone(), 0);
        module_cov2.insert(func_name.clone(), 1);

        map1.module_maps.insert(
            ("path1".to_string(), addr, module_name.clone()),
            module_cov1,
        );
        map2.module_maps.insert(
            ("path1".to_string(), addr, module_name.clone()),
            module_cov2,
        );

        map1.merge(map2);

        let merged = map1
            .module_maps
            .get(&("path1".to_string(), addr, module_name))
            .unwrap();
        let func_cov = merged.function_maps.get(&func_name).unwrap();

        assert_eq!(func_cov.get(&0), Some(&2));
        assert_eq!(func_cov.get(&1), Some(&1));
    }
}
