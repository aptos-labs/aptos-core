// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Stack analyzer for computing function frame sizes and call depths.
//!
//! This module performs pseudo-monomorphization starting from entry functions,
//! tracking stack usage and detecting recursion.

use crate::{resolver::TypeResolver, types::TypeName};
use move_binary_format::{access::ModuleAccess, file_format::Bytecode, CompiledModule};
use move_core_types::account_address::AccountAddress;
use std::collections::{BTreeMap, BTreeSet};

/// Module name substrings to skip during analysis (to avoid combinatorial explosion from DeFi AMM modules)
const SKIP_MODULE_SUBSTRINGS: &[&str] = &[
    "weighted",               // Thala weighted pools and all wrappers
    "thala",                  // All Thala-related modules
    "stable_swap",            // Thala stable swap adapters
    "stable_pool",            // Thala stable pools
    "panora_swap_aggregator", // Panora aggregator (30+ type params)
];

/// Check if a module name should be skipped
fn should_skip_module(module_name: &str) -> bool {
    SKIP_MODULE_SUBSTRINGS
        .iter()
        .any(|s| module_name.contains(s))
}

/// Unique identifier for a monomorphized function
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FunctionId {
    pub address: AccountAddress,
    pub module: String,
    pub name: String,
    pub type_args: Vec<TypeName>,
}

impl std::fmt::Display for FunctionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}::{}::{}", self.address, self.module, self.name)?;
        if !self.type_args.is_empty() {
            write!(f, "<")?;
            for (i, arg) in self.type_args.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", arg)?;
            }
            write!(f, ">")?;
        }
        Ok(())
    }
}

/// Stack usage information for a monomorphized function
#[derive(Debug, Clone)]
pub struct FunctionStackInfo {
    /// Size of locals + parameters on the stack
    pub frame_size: usize,
    /// Max call depth to reach this function from any entry (None if recursive)
    pub max_call_depth: Option<usize>,
    /// Max cumulative stack size to reach this function (None if recursive)
    pub max_stack_size: Option<usize>,
    /// Whether this is an entry function
    pub is_entry: bool,
    /// Whether this function is part of a recursive call chain
    pub is_recursive: bool,
}

/// Analyzer for computing stack usage across the call graph
pub struct StackAnalyzer<'a> {
    /// Type resolver for computing type sizes
    resolver: &'a mut TypeResolver<'a>,
    /// All loaded modules, keyed by (address, module_name)
    modules: BTreeMap<(AccountAddress, String), &'a CompiledModule>,
    /// Computed function information
    functions: BTreeMap<FunctionId, FunctionStackInfo>,
}

impl<'a> StackAnalyzer<'a> {
    pub fn new(modules: &'a [CompiledModule], resolver: &'a mut TypeResolver<'a>) -> Self {
        let module_map: BTreeMap<_, _> = modules
            .iter()
            .map(|m| ((*m.address(), m.name().to_string()), m))
            .collect();

        Self {
            resolver,
            modules: module_map,
            functions: BTreeMap::new(),
        }
    }

    /// Run the analysis starting from all entry functions
    pub fn analyze_all(&mut self) {
        // Collect all entry functions
        let entries: Vec<FunctionId> = self.collect_entry_functions();
        let total = entries.len();
        eprintln!("Found {} entry functions", total);

        // Process each entry function
        for (i, entry_id) in entries.into_iter().enumerate() {
            println!("{}/{} entry functions processed", i, total);
            self.analyze_function(&entry_id, true, 0, 0, &mut BTreeSet::new());
        }
        eprintln!("{} functions monomorphized", self.functions.len());
    }

    /// Collect all entry functions from all modules
    fn collect_entry_functions(&self) -> Vec<FunctionId> {
        let mut entries = Vec::new();

        for ((addr, module_name), module) in &self.modules {
            // Skip modules that cause combinatorial explosion
            if should_skip_module(module_name) {
                continue;
            }

            for func_def in &module.function_defs {
                if func_def.is_entry {
                    let func_handle = module.function_handle_at(func_def.function);
                    let name = module.identifier_at(func_handle.name).to_string();

                    // Create opaque type args for generic entry functions (normalized to avoid explosion)
                    let type_args: Vec<TypeName> = (0..func_handle.type_parameters.len())
                        .map(|_| TypeName::Opaque(0))
                        .collect();

                    entries.push(FunctionId {
                        address: *addr,
                        module: module_name.clone(),
                        name,
                        type_args,
                    });
                }
            }
        }

        entries
    }

    /// Analyze a function and its callees recursively
    ///
    /// Returns the frame size of this function (for callers to use)
    fn analyze_function(
        &mut self,
        func_id: &FunctionId,
        is_entry: bool,
        call_depth: usize,
        stack_size_so_far: usize,
        visiting: &mut BTreeSet<FunctionId>,
    ) -> Option<usize> {
        // Check for recursion
        if visiting.contains(func_id) {
            return None;
        }

        // Safeguard against extremely deep call chains
        if call_depth > 100 {
            eprintln!("Warning: call depth > 100 for {}", func_id);
            return None;
        }

        // Check if already analyzed
        if let Some(info) = self.functions.get_mut(func_id) {
            // Update depth metrics if this path is deeper
            if let Some(max_depth) = info.max_call_depth {
                if call_depth > max_depth {
                    info.max_call_depth = Some(call_depth);
                }
            }
            if let Some(max_stack) = info.max_stack_size {
                let new_stack = stack_size_so_far + info.frame_size;
                if new_stack > max_stack {
                    info.max_stack_size = Some(new_stack);
                }
            }
            return Some(info.frame_size);
        }

        // Find the module and function definition
        let module = match self.modules.get(&(func_id.address, func_id.module.clone())) {
            Some(m) => *m,
            None => return None,
        };

        let func_def = module.function_defs.iter().find(|def| {
            let handle = module.function_handle_at(def.function);
            module.identifier_at(handle.name).as_str() == func_id.name
        });

        let func_def = match func_def {
            Some(f) => f,
            None => return None,
        };

        let func_handle = module.function_handle_at(func_def.function);

        // Compute frame size using TypeResolver
        let params_size =
            self.compute_signature_size(module, func_handle.parameters, &func_id.type_args);
        let locals_size = func_def
            .code
            .as_ref()
            .map(|c| self.compute_signature_size(module, c.locals, &func_id.type_args))
            .unwrap_or(0);
        let frame_size = params_size + locals_size;

        // Create initial info
        let info = FunctionStackInfo {
            frame_size,
            max_call_depth: Some(call_depth),
            max_stack_size: Some(stack_size_so_far + frame_size),
            is_entry,
            is_recursive: false,
        };
        self.functions.insert(func_id.clone(), info);

        // Print progress
        let count = self.functions.len();
        if count % 100 == 0 {
            eprintln!("{} functions monomorphized: {}", count, func_id);
        }

        // Mark as visiting for recursion detection
        visiting.insert(func_id.clone());

        // Process callees
        let mut is_recursive = false;
        if let Some(code) = &func_def.code {
            let callees = self.collect_callees(module, &code.code, &func_id.type_args);

            for callee_id in callees {
                let result = self.analyze_function(
                    &callee_id,
                    false,
                    call_depth + 1,
                    stack_size_so_far + frame_size,
                    visiting,
                );

                if result.is_none() {
                    is_recursive = true;
                }

                if let Some(callee_info) = self.functions.get(&callee_id) {
                    if callee_info.is_recursive {
                        is_recursive = true;
                    }
                }
            }
        }

        // Unmark visiting
        visiting.remove(func_id);

        // Update recursive status
        if is_recursive {
            if let Some(info) = self.functions.get_mut(func_id) {
                info.is_recursive = true;
                info.max_call_depth = None;
                info.max_stack_size = None;
            }
        }

        Some(frame_size)
    }

    /// Compute total size of types in a signature using TypeResolver
    fn compute_signature_size(
        &mut self,
        module: &CompiledModule,
        sig_idx: move_binary_format::file_format::SignatureIndex,
        type_args: &[TypeName],
    ) -> usize {
        let sig = module.signature_at(sig_idx);
        let mut total = 0;

        for token in &sig.0 {
            if let Ok(type_name) = self
                .resolver
                .signature_token_to_type_name(module, token, type_args)
            {
                if let Ok(info) = self.resolver.resolve_type(&type_name) {
                    total += info.stack_size;
                }
            }
        }

        total
    }

    /// Collect all function callees from bytecode
    fn collect_callees(
        &self,
        module: &CompiledModule,
        code: &[Bytecode],
        func_type_args: &[TypeName],
    ) -> Vec<FunctionId> {
        let mut callees = Vec::new();

        for instr in code {
            match instr {
                Bytecode::Call(func_handle_idx) => {
                    let handle = module.function_handle_at(*func_handle_idx);
                    let module_handle = module.module_handle_at(handle.module);
                    let address = *module.address_identifier_at(module_handle.address);
                    let module_name = module.identifier_at(module_handle.name).to_string();

                    // Skip calls to modules that cause combinatorial explosion
                    if should_skip_module(&module_name) {
                        continue;
                    }

                    let name = module.identifier_at(handle.name).to_string();

                    callees.push(FunctionId {
                        address,
                        module: module_name,
                        name,
                        type_args: vec![],
                    });
                },
                Bytecode::CallGeneric(func_inst_idx) => {
                    let inst = module.function_instantiation_at(*func_inst_idx);
                    let handle = module.function_handle_at(inst.handle);
                    let module_handle = module.module_handle_at(handle.module);
                    let address = *module.address_identifier_at(module_handle.address);
                    let module_name = module.identifier_at(module_handle.name).to_string();

                    // Skip calls to modules that cause combinatorial explosion
                    if should_skip_module(&module_name) {
                        continue;
                    }

                    let name = module.identifier_at(handle.name).to_string();

                    // Resolve type arguments using TypeResolver
                    let type_params_sig = module.signature_at(inst.type_parameters);
                    let mut resolved_args = Vec::new();
                    let mut valid = true;

                    for token in &type_params_sig.0 {
                        match self.resolver.signature_token_to_type_name(
                            module,
                            token,
                            func_type_args,
                        ) {
                            Ok(t) => resolved_args.push(t),
                            Err(_) => {
                                valid = false;
                                break;
                            },
                        }
                    }

                    if valid {
                        callees.push(FunctionId {
                            address,
                            module: module_name,
                            name,
                            type_args: resolved_args,
                        });
                    }
                },
                _ => {},
            }
        }

        callees
    }

    /// Get results sorted by max stack size (descending)
    pub fn get_results(&self) -> Vec<(&FunctionId, &FunctionStackInfo)> {
        let mut results: Vec<_> = self.functions.iter().collect();

        results.sort_by(|a, b| match (a.1.max_stack_size, b.1.max_stack_size) {
            (Some(sa), Some(sb)) => sb.cmp(&sa),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.0.cmp(b.0),
        });

        results
    }

    /// Get count of analyzed functions
    pub fn function_count(&self) -> usize {
        self.functions.len()
    }
}
