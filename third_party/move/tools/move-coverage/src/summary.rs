// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::coverage_map::{
    ExecCoverageMap, ExecCoverageMapWithModules, ModuleCoverageMap, TraceMap,
};
use move_binary_format::{
    access::ModuleAccess,
    control_flow_graph::{BlockId, ControlFlowGraph, VMControlFlowGraph},
    file_format::{Bytecode, CodeOffset},
    CompiledModule,
};
use move_core_types::{identifier::Identifier, language_storage::ModuleId};
use petgraph::{algo::tarjan_scc, Graph};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{self, Write},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleSummary {
    pub module_name: ModuleId,
    pub function_summaries: BTreeMap<Identifier, FunctionSummary>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionSummary {
    pub fn_is_native: bool,
    pub total: u64,
    pub covered: u64,
}

/// Information about a function's control flow structure for path coverage analysis.
///
/// Note: This structure is used internally by `summarize_path_cov` and captures
/// static control flow information. Dynamic dispatch through closures (`CallClosure`)
/// is not represented in this structure.
pub struct FunctionInfo {
    pub fn_name: Identifier,
    pub fn_entry: CodeOffset,
    /// All exit points of the function (Ret, Abort, AbortMsg bytecodes)
    pub fn_exits: BTreeSet<CodeOffset>,
    /// Branch points mapping from source instruction to possible destinations
    pub fn_branches: BTreeMap<CodeOffset, BTreeSet<CodeOffset>>,
    /// Total number of statically-determined paths through this function
    pub fn_num_paths: u64,
}

impl ModuleSummary {
    /// Summarizes the modules coverage in CSV format
    pub fn summarize_csv<W: Write>(&self, summary_writer: &mut W) -> io::Result<()> {
        let module = format!(
            "{}::{}",
            self.module_name.address().to_hex(),
            self.module_name.name()
        );

        let mut format_line = |fn_name, covered, uncovered| {
            writeln!(
                summary_writer,
                "{},{},{},{}",
                module, fn_name, covered, uncovered
            )
        };

        for (fn_name, fn_summary) in self
            .function_summaries
            .iter()
            .filter(|(_, summary)| !summary.fn_is_native)
        {
            format_line(fn_name, fn_summary.covered, fn_summary.total)?;
        }

        Ok(())
    }

    /// Summarizes the modules coverage, and returns the total module coverage in a human-readable
    /// format.
    pub fn summarize_human<W: Write>(
        &self,
        summary_writer: &mut W,
        summarize_function_coverage: bool,
    ) -> io::Result<(u64, u64)> {
        let mut all_total = 0;
        let mut all_covered = 0;

        writeln!(
            summary_writer,
            "Module {}::{}",
            self.module_name.address().to_hex(),
            self.module_name.name()
        )?;

        for (fn_name, fn_summary) in self.function_summaries.iter() {
            all_total += fn_summary.total;
            all_covered += fn_summary.covered;

            if summarize_function_coverage {
                let native = if fn_summary.fn_is_native {
                    "native "
                } else {
                    ""
                };
                writeln!(summary_writer, "\t{}fun {}", native, fn_name)?;
                writeln!(summary_writer, "\t\ttotal: {}", fn_summary.total)?;
                writeln!(summary_writer, "\t\tcovered: {}", fn_summary.covered)?;
                writeln!(
                    summary_writer,
                    "\t\t% coverage: {:.2}",
                    fn_summary.percent_coverage()
                )?;
            }
        }

        let covered_percentage = (all_covered as f64) / (all_total as f64) * 100f64;
        writeln!(
            summary_writer,
            ">>> % Module coverage: {:.2}",
            covered_percentage
        )?;
        Ok((all_total, all_covered))
    }
}

impl FunctionSummary {
    pub fn percent_coverage(&self) -> f64 {
        (self.covered as f64) / (self.total as f64) * 100f64
    }
}

pub fn summarize_inst_cov_by_module(
    module: &CompiledModule,
    module_map: Option<&ModuleCoverageMap>,
) -> ModuleSummary {
    let module_name = module.self_id();
    let function_summaries: BTreeMap<_, _> = module
        .function_defs()
        .iter()
        .map(|function_def| {
            let fn_handle = module.function_handle_at(function_def.function);
            let fn_name = module.identifier_at(fn_handle.name).to_owned();

            let fn_summmary = match &function_def.code {
                None => FunctionSummary {
                    fn_is_native: true,
                    total: 0,
                    covered: 0,
                },
                Some(code_unit) => {
                    let total_number_of_instructions = code_unit.code.len() as u64;
                    let covered_instructions = module_map
                        .and_then(|fn_map| {
                            fn_map
                                .function_maps
                                .get(&fn_name)
                                .map(|function_map| function_map.len())
                        })
                        .unwrap_or(0) as u64;
                    FunctionSummary {
                        fn_is_native: false,
                        total: total_number_of_instructions,
                        covered: covered_instructions,
                    }
                },
            };

            (fn_name, fn_summmary)
        })
        .collect();

    ModuleSummary {
        module_name,
        function_summaries,
    }
}

pub fn summarize_inst_cov(
    module: &CompiledModule,
    coverage_map: &ExecCoverageMap,
) -> ModuleSummary {
    let module_name = module.self_id();
    let module_map = coverage_map
        .module_maps
        .get(&(*module_name.address(), module_name.name().to_owned()));
    summarize_inst_cov_by_module(module, module_map)
}

/// Summarizes path coverage for a module based on execution traces.
///
/// This function analyzes the control flow graph of each function to identify
/// possible execution paths, then examines the trace map to determine which
/// paths were actually executed.
///
/// # Limitations
///
/// **Closures**: This analysis has limited support for closures (`CallClosure` bytecode).
/// When a closure is invoked, the target function is determined dynamically at runtime
/// from the closure value, not statically from the bytecode. As a result:
/// - The path coverage analysis cannot accurately trace calls through closures
/// - Functions invoked via closures may not have their paths properly attributed
/// - The total path count may be inaccurate for code using closures extensively
///
/// **Exit Points**: The analysis recognizes `Ret`, `Abort`, and `AbortMsg` as function
/// exit points. Paths ending in aborts are counted as valid execution paths.
pub fn summarize_path_cov(module: &CompiledModule, trace_map: &TraceMap) -> ModuleSummary {
    let module_name = module.self_id();

    // collect branching information per function
    let func_info: BTreeMap<_, _> = module
        .function_defs()
        .iter()
        .filter_map(|function_def| {
            match &function_def.code {
                None => None,
                Some(code_unit) => {
                    // build control-flow graph
                    let fn_cfg = VMControlFlowGraph::new(code_unit.code.as_slice());

                    // get function entry and exit points (returns and aborts)
                    let fn_entry = fn_cfg.block_start(fn_cfg.entry_block_id());
                    let mut fn_exits: BTreeSet<CodeOffset> = BTreeSet::new();
                    for block_id in fn_cfg.blocks().into_iter() {
                        for i in fn_cfg.block_start(block_id)..=fn_cfg.block_end(block_id) {
                            match &code_unit.code[i as usize] {
                                Bytecode::Ret | Bytecode::Abort | Bytecode::AbortMsg => {
                                    fn_exits.insert(i);
                                },
                                _ => {},
                            }
                        }
                    }

                    // convert into strongly connected components (SCC) graph
                    let mut fn_dgraph: Graph<BlockId, ()> = Graph::new();

                    let block_to_node: BTreeMap<_, _> = fn_cfg
                        .blocks()
                        .into_iter()
                        .map(|block_id| (block_id, fn_dgraph.add_node(block_id)))
                        .collect();

                    for block_id in fn_cfg.blocks().into_iter() {
                        for succ_block_id in fn_cfg.successors(block_id).iter() {
                            fn_dgraph.add_edge(
                                *block_to_node.get(&block_id).unwrap(),
                                *block_to_node.get(succ_block_id).unwrap(),
                                (),
                            );
                        }
                    }

                    let scc_iter = tarjan_scc(&fn_dgraph).into_iter();

                    // collect branching points
                    let mut fn_branches: BTreeMap<CodeOffset, BTreeSet<CodeOffset>> =
                        BTreeMap::new();

                    let mut path_nums: BTreeMap<usize, BTreeMap<usize, usize>> = BTreeMap::new();
                    let mut inst_locs: BTreeMap<CodeOffset, usize> = BTreeMap::new();
                    for (scc_idx, scc) in scc_iter.enumerate() {
                        // collect locations (i.e., offsets) in this SCC
                        for node_idx in scc.iter() {
                            let block_id = *fn_dgraph.node_weight(*node_idx).unwrap();
                            for i in fn_cfg.block_start(block_id)..=fn_cfg.block_end(block_id) {
                                // there is no way we could assign the same instruction twice
                                assert!(inst_locs.insert(i, scc_idx).is_none());
                            }
                        }

                        // collect branches out of this SCC
                        let mut exits: BTreeSet<(CodeOffset, CodeOffset)> = BTreeSet::new();
                        for node_idx in scc.iter() {
                            let block_id = *fn_dgraph.node_weight(*node_idx).unwrap();
                            let term_inst_id = fn_cfg.block_end(block_id);
                            for dest in
                                Bytecode::get_successors(term_inst_id, code_unit.code.as_slice())
                                    .into_iter()
                            {
                                if *inst_locs.get(&dest).unwrap() != scc_idx {
                                    assert!(exits.insert((term_inst_id, dest)));
                                }
                            }
                        }

                        // calculate number of possible paths
                        if exits.is_empty() {
                            // this is the termination scc
                            assert!(path_nums.insert(scc_idx, BTreeMap::new()).is_none());
                            path_nums.get_mut(&scc_idx).unwrap().insert(scc_idx, 1);
                        } else {
                            // update reachability map
                            let mut reachability: BTreeMap<usize, usize> = BTreeMap::new();
                            for (_, dst) in exits.iter() {
                                let dst_scc_idx = inst_locs.get(dst).unwrap();
                                for (path_end_scc, path_end_reach_set) in path_nums.iter() {
                                    let reach_from_dst =
                                        if let Some(v) = path_end_reach_set.get(dst_scc_idx) {
                                            *v
                                        } else {
                                            0
                                        };
                                    let reach_from_scc =
                                        reachability.entry(*path_end_scc).or_insert(0);
                                    *reach_from_scc += reach_from_dst;
                                }
                            }

                            for (path_end_scc, path_end_reachability) in reachability.into_iter() {
                                assert!(path_nums
                                    .get_mut(&path_end_scc)
                                    .unwrap()
                                    .insert(scc_idx, path_end_reachability)
                                    .is_none());
                            }

                            // move to branch info if there are more than one branches
                            if exits.len() > 1 {
                                for (src, dst) in exits.into_iter() {
                                    fn_branches.entry(src).or_default().insert(dst);
                                }
                            }
                        }
                    }

                    // calculate path num
                    let entry_scc = inst_locs
                        .get(&fn_cfg.block_start(fn_cfg.entry_block_id()))
                        .unwrap();
                    let mut fn_num_paths: u64 = 0;
                    for (_, path_end_reachability) in path_nums {
                        fn_num_paths += if let Some(v) = path_end_reachability.get(entry_scc) {
                            *v as u64
                        } else {
                            0
                        };
                    }

                    // use function name as key
                    let fn_name = module
                        .identifier_at(module.function_handle_at(function_def.function).name)
                        .to_owned();
                    Some((fn_name.clone(), FunctionInfo {
                        fn_name,
                        fn_entry,
                        fn_exits,
                        fn_branches,
                        fn_num_paths,
                    }))
                },
            }
        })
        .collect();

    // examine the trace and check the path covered
    let mut func_path_cov_stats: BTreeMap<
        Identifier,
        BTreeMap<BTreeSet<(CodeOffset, CodeOffset)>, u64>,
    > = BTreeMap::new();

    for (_, trace) in trace_map.exec_maps.iter() {
        let mut call_stack: Vec<&FunctionInfo> = Vec::new();
        let mut path_stack: Vec<BTreeSet<(CodeOffset, CodeOffset)>> = Vec::new();
        let mut path_store: Vec<(Identifier, BTreeSet<(CodeOffset, CodeOffset)>)> = Vec::new();
        for (index, record) in trace.iter().enumerate().filter(|(_, e)| {
            e.module_addr == *module_name.address()
                && e.module_name.as_ident_str() == module_name.name()
        }) {
            let (info, is_call) = if let Some(last) = call_stack.last() {
                if last.fn_name.as_ident_str() != record.func_name.as_ident_str() {
                    // calls into a new function
                    (func_info.get(&record.func_name).unwrap(), true)
                } else if last.fn_entry == record.func_pc {
                    // recursive calls into itself
                    (*last, true)
                } else {
                    // execution stayed within the function
                    (*last, false)
                }
            } else {
                // fresh into the module
                (func_info.get(&record.func_name).unwrap(), true)
            };

            // push stacks if we call into a new function
            if is_call {
                assert_eq!(info.fn_entry, record.func_pc);
                call_stack.push(info);
                path_stack.push(BTreeSet::new());
            }
            let path = path_stack.last_mut().unwrap();

            // check if branching
            if let Some(dests) = info.fn_branches.get(&record.func_pc) {
                // the nest instruction must be within the same function
                let next_record = trace.get(index + 1).unwrap();
                assert_eq!(record.func_name, next_record.func_name);

                // add the transition to path
                if dests.contains(&next_record.func_pc) {
                    assert!(path.insert((record.func_pc, next_record.func_pc)));
                }
            }

            // pop stacks if we exited (returned or aborted)
            if info.fn_exits.contains(&record.func_pc) {
                call_stack.pop().unwrap();
                // save the full path temporarily in path_store
                path_store.push((record.func_name.clone(), path_stack.pop().unwrap()));
            }
        }

        // check if all calls were matched with exits
        if !call_stack.is_empty() {
            // execution ended unexpectedly (e.g., VM error or incomplete trace)
            // TODO: it is better to confirm this by adding a trace record
            call_stack.clear();
            path_stack.clear();
            path_store.clear();
        } else {
            // record path only when execution finishes properly
            for (func_name, path) in path_store.into_iter() {
                let path_count = func_path_cov_stats
                    .entry(func_name)
                    .or_default()
                    .entry(path)
                    .or_insert(0);
                *path_count += 1;
            }
        }
    }

    // calculate function summaries
    let function_summaries: BTreeMap<_, _> = module
        .function_defs()
        .iter()
        .map(|function_def| {
            let fn_handle = module.function_handle_at(function_def.function);
            let fn_name = module.identifier_at(fn_handle.name).to_owned();

            let fn_summmary = match &function_def.code {
                None => FunctionSummary {
                    fn_is_native: true,
                    total: 0,
                    covered: 0,
                },
                Some(_) => FunctionSummary {
                    fn_is_native: false,
                    total: func_info.get(&fn_name).unwrap().fn_num_paths,
                    covered: match func_path_cov_stats.get(&fn_name) {
                        None => 0,
                        Some(pathset) => pathset.len() as u64,
                    },
                },
            };

            (fn_name, fn_summmary)
        })
        .collect();

    ModuleSummary {
        module_name,
        function_summaries,
    }
}

impl ExecCoverageMapWithModules {
    pub fn into_module_summaries(self) -> BTreeMap<String, ModuleSummary> {
        let compiled_modules = self.compiled_modules;
        self.module_maps
            .into_iter()
            .map(|((module_path, _, _), module_cov)| {
                let module_summary = summarize_inst_cov_by_module(
                    compiled_modules.get(&module_path).unwrap(),
                    Some(&module_cov),
                );
                (module_path, module_summary)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_binary_format::file_format::{
        self, CodeUnit, FunctionDefinition, FunctionHandle, FunctionHandleIndex, IdentifierIndex,
        ModuleHandleIndex, SignatureIndex, Visibility,
    };
    use move_core_types::account_address::AccountAddress;

    fn test_addr() -> AccountAddress {
        AccountAddress::from_hex_literal("0x1").unwrap()
    }

    #[test]
    fn test_function_summary_percent_coverage() {
        let summary = FunctionSummary {
            fn_is_native: false,
            total: 100,
            covered: 50,
        };
        assert!((summary.percent_coverage() - 50.0).abs() < f64::EPSILON);

        let summary_full = FunctionSummary {
            fn_is_native: false,
            total: 100,
            covered: 100,
        };
        assert!((summary_full.percent_coverage() - 100.0).abs() < f64::EPSILON);

        let summary_zero = FunctionSummary {
            fn_is_native: false,
            total: 100,
            covered: 0,
        };
        assert!((summary_zero.percent_coverage() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_module_summary_csv_output() {
        let module_name = ModuleId::new(test_addr(), Identifier::new("TestModule").unwrap());
        let mut function_summaries = BTreeMap::new();

        function_summaries.insert(
            Identifier::new("test_func").unwrap(),
            FunctionSummary {
                fn_is_native: false,
                total: 10,
                covered: 5,
            },
        );
        function_summaries.insert(
            Identifier::new("native_func").unwrap(),
            FunctionSummary {
                fn_is_native: true,
                total: 0,
                covered: 0,
            },
        );

        let summary = ModuleSummary {
            module_name,
            function_summaries,
        };

        let mut output = Vec::new();
        summary.summarize_csv(&mut output).unwrap();
        let output_str = String::from_utf8(output).unwrap();

        // Native functions should be excluded from CSV output
        assert!(!output_str.contains("native_func"));
        // Non-native function should be included
        assert!(output_str.contains("test_func"));
        assert!(output_str.contains("0000000000000000000000000000000000000000000000000000000000000001::TestModule"));
    }

    #[test]
    fn test_module_summary_human_output() {
        let module_name = ModuleId::new(test_addr(), Identifier::new("TestModule").unwrap());
        let mut function_summaries = BTreeMap::new();

        function_summaries.insert(
            Identifier::new("func1").unwrap(),
            FunctionSummary {
                fn_is_native: false,
                total: 10,
                covered: 5,
            },
        );
        function_summaries.insert(
            Identifier::new("func2").unwrap(),
            FunctionSummary {
                fn_is_native: false,
                total: 20,
                covered: 20,
            },
        );

        let summary = ModuleSummary {
            module_name,
            function_summaries,
        };

        let mut output = Vec::new();
        let (total, covered) = summary.summarize_human(&mut output, true).unwrap();

        assert_eq!(total, 30);
        assert_eq!(covered, 25);

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("Module"));
        assert!(output_str.contains("TestModule"));
        assert!(output_str.contains("func1"));
        assert!(output_str.contains("func2"));
        assert!(output_str.contains("% coverage"));
    }

    #[test]
    fn test_module_summary_human_output_without_function_details() {
        let module_name = ModuleId::new(test_addr(), Identifier::new("TestModule").unwrap());
        let mut function_summaries = BTreeMap::new();

        function_summaries.insert(
            Identifier::new("func1").unwrap(),
            FunctionSummary {
                fn_is_native: false,
                total: 10,
                covered: 5,
            },
        );

        let summary = ModuleSummary {
            module_name,
            function_summaries,
        };

        let mut output = Vec::new();
        summary.summarize_human(&mut output, false).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        // Should contain module info
        assert!(output_str.contains("Module"));
        // Should NOT contain detailed function info (no tab-indented output)
        assert!(!output_str.contains("\tfun"));
    }

    #[test]
    fn test_function_info_creation() {
        let fn_name = Identifier::new("test_func").unwrap();
        let mut fn_exits = BTreeSet::new();
        fn_exits.insert(5); // Ret at PC 5
        fn_exits.insert(10); // Abort at PC 10

        let mut fn_branches = BTreeMap::new();
        let mut branch_targets = BTreeSet::new();
        branch_targets.insert(3);
        branch_targets.insert(7);
        fn_branches.insert(2, branch_targets);

        let info = FunctionInfo {
            fn_name: fn_name.clone(),
            fn_entry: 0,
            fn_exits: fn_exits.clone(),
            fn_branches: fn_branches.clone(),
            fn_num_paths: 2,
        };

        assert_eq!(info.fn_name, fn_name);
        assert_eq!(info.fn_entry, 0);
        assert!(info.fn_exits.contains(&5));
        assert!(info.fn_exits.contains(&10));
        assert_eq!(info.fn_exits.len(), 2);
        assert_eq!(info.fn_num_paths, 2);
    }

    #[test]
    fn test_bytecode_exit_detection() {
        // Test that Ret, Abort, and AbortMsg are all considered exit points
        use move_binary_format::file_format::Bytecode;

        let test_cases = vec![
            (Bytecode::Ret, true),
            (Bytecode::Abort, true),
            (Bytecode::AbortMsg, true),
            (Bytecode::Pop, false),
            (Bytecode::LdU64(0), false),
            (Bytecode::Branch(0), false),
            (Bytecode::BrTrue(0), false),
        ];

        for (bytecode, is_exit) in test_cases {
            let is_match = matches!(
                bytecode,
                Bytecode::Ret | Bytecode::Abort | Bytecode::AbortMsg
            );
            assert_eq!(
                is_match, is_exit,
                "Bytecode {:?} should {}be an exit",
                bytecode,
                if is_exit { "" } else { "not " }
            );
        }
    }

    /// Helper to create a test module with specified functions
    fn create_test_module_with_functions(
        module_name: &str,
        functions: Vec<(&str, Vec<Bytecode>)>,
    ) -> CompiledModule {
        let mut module = file_format::empty_module();
        module.identifiers[0] = Identifier::new(module_name).unwrap();

        for (func_name, code) in functions {
            // Add function handle
            module.function_handles.push(FunctionHandle {
                module: ModuleHandleIndex(0),
                name: IdentifierIndex(module.identifiers.len() as u16),
                parameters: SignatureIndex(0),
                return_: SignatureIndex(0),
                type_parameters: vec![],
                access_specifiers: None,
                attributes: vec![],
            });
            module
                .identifiers
                .push(Identifier::new(func_name).unwrap());

            // Add function definition with the specified code
            module.function_defs.push(FunctionDefinition {
                function: FunctionHandleIndex((module.function_defs.len()) as u16),
                visibility: Visibility::Private,
                is_entry: false,
                acquires_global_resources: vec![],
                code: Some(CodeUnit {
                    locals: SignatureIndex(0),
                    code,
                }),
            });
        }

        module
    }

    #[test]
    fn test_summarize_inst_cov_with_module() {
        // Create a module with two functions
        let module = create_test_module_with_functions(
            "TestModule",
            vec![
                ("returns_normally", vec![Bytecode::LdU64(0), Bytecode::Pop, Bytecode::Ret]),
                ("aborts", vec![Bytecode::LdU64(1), Bytecode::Abort]),
            ],
        );

        // Create coverage map with partial coverage
        let addr = AccountAddress::ZERO;
        let module_name = Identifier::new("TestModule").unwrap();
        let mut coverage_map = crate::coverage_map::ModuleCoverageMap::new(addr, module_name.clone());

        // Cover first two instructions of returns_normally
        let func1 = Identifier::new("returns_normally").unwrap();
        coverage_map.insert(func1.clone(), 0);
        coverage_map.insert(func1.clone(), 1);

        // Cover first instruction of aborts
        let func2 = Identifier::new("aborts").unwrap();
        coverage_map.insert(func2.clone(), 0);

        let summary = summarize_inst_cov_by_module(&module, Some(&coverage_map));

        // Check returns_normally: 3 total instructions, 2 covered
        let func1_summary = summary.function_summaries.get(&func1).unwrap();
        assert_eq!(func1_summary.total, 3);
        assert_eq!(func1_summary.covered, 2);
        assert!(!func1_summary.fn_is_native);

        // Check aborts: 2 total instructions, 1 covered
        let func2_summary = summary.function_summaries.get(&func2).unwrap();
        assert_eq!(func2_summary.total, 2);
        assert_eq!(func2_summary.covered, 1);
        assert!(!func2_summary.fn_is_native);
    }

    #[test]
    fn test_summarize_inst_cov_no_coverage() {
        let module = create_test_module_with_functions(
            "TestModule",
            vec![("func", vec![Bytecode::Ret])],
        );

        let summary = summarize_inst_cov_by_module(&module, None);

        let func = Identifier::new("func").unwrap();
        let func_summary = summary.function_summaries.get(&func).unwrap();
        assert_eq!(func_summary.total, 1);
        assert_eq!(func_summary.covered, 0);
    }

    #[test]
    fn test_path_coverage_detects_abort_exits() {
        // Create a module with a function that has conditional abort
        // Code: if (true) abort else return
        // Bytecodes: LdTrue, BrFalse(3), LdU64(0), Abort, Ret
        let module = create_test_module_with_functions(
            "TestModule",
            vec![(
                "conditional_abort",
                vec![
                    Bytecode::LdTrue,     // 0
                    Bytecode::BrFalse(4), // 1: if false, jump to Ret
                    Bytecode::LdU64(0),   // 2: load abort code
                    Bytecode::Abort,      // 3: abort
                    Bytecode::Ret,        // 4: return
                ],
            )],
        );

        // Create empty trace map (no executions)
        let trace_map = TraceMap {
            exec_maps: BTreeMap::new(),
        };

        let summary = summarize_path_cov(&module, &trace_map);

        let func = Identifier::new("conditional_abort").unwrap();
        let func_summary = summary.function_summaries.get(&func).unwrap();

        // Should have 2 paths: one through abort (0->1->2->3), one through ret (0->1->4)
        assert_eq!(func_summary.total, 2, "Should detect 2 paths (abort and ret)");
        assert_eq!(func_summary.covered, 0, "No paths covered (empty trace)");
    }

    #[test]
    fn test_path_coverage_with_abortmsg() {
        // Create a module with AbortMsg
        let module = create_test_module_with_functions(
            "TestModule",
            vec![(
                "abort_with_message",
                vec![
                    Bytecode::LdU64(1),                    // 0: error code
                    Bytecode::VecPack(SignatureIndex(0), 0), // 1: empty vector for message
                    Bytecode::AbortMsg,                    // 2: abort with message
                ],
            )],
        );

        let trace_map = TraceMap {
            exec_maps: BTreeMap::new(),
        };

        let summary = summarize_path_cov(&module, &trace_map);

        let func = Identifier::new("abort_with_message").unwrap();
        let func_summary = summary.function_summaries.get(&func).unwrap();

        // Should have 1 path through AbortMsg
        assert_eq!(func_summary.total, 1, "Should detect 1 path through AbortMsg");
    }

    #[test]
    fn test_path_coverage_multiple_exit_types() {
        // Create a module with Ret, Abort, and AbortMsg in different functions
        let module = create_test_module_with_functions(
            "TestModule",
            vec![
                ("returns", vec![Bytecode::Ret]),
                ("aborts", vec![Bytecode::LdU64(0), Bytecode::Abort]),
                (
                    "aborts_msg",
                    vec![
                        Bytecode::LdU64(0),
                        Bytecode::VecPack(SignatureIndex(0), 0),
                        Bytecode::AbortMsg,
                    ],
                ),
            ],
        );

        let trace_map = TraceMap {
            exec_maps: BTreeMap::new(),
        };

        let summary = summarize_path_cov(&module, &trace_map);

        // Each function should have exactly 1 path
        for name in ["returns", "aborts", "aborts_msg"] {
            let func = Identifier::new(name).unwrap();
            let func_summary = summary.function_summaries.get(&func).unwrap();
            assert_eq!(
                func_summary.total, 1,
                "Function {} should have 1 path",
                name
            );
        }
    }
}
