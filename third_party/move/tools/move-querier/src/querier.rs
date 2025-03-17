// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Tool to support query operations on a move package represented as a move model
//!
//! `load_package()` allows loading modules form bytecode files into the move model
//! `add_module()` allows adding a given CompiledModule into the move model
//! `query()` supports operations to dump call graph and dependency graph
//!
//! Results are saved as raw .dot files
//! For better displays, module addresses are truncated to 8-bytes hex values (if longer)
//! and function signatures are omitted

use anyhow::{anyhow, Context, Result};
use clap::Args;
use codespan::Span;
use move_binary_format::file_format::{CompiledModule, CompiledScript};
use move_bytecode_source_map::source_map::SourceMap;
use move_command_line_common::files::{FileHash, SOURCE_MAP_EXTENSION};
use move_model::{
    convert_script_to_module,
    model::{FunId, FunctionEnv, GlobalEnv, Loc, ModuleEnv, ModuleId, QualifiedId},
};
use petgraph::{
    dot::{Config, Dot},
    graph::{DiGraph, NodeIndex},
};
use std::{
    collections::{hash_map::Entry, BTreeMap, BTreeSet, HashMap, VecDeque},
    fs,
    hash::Hash,
    path::PathBuf,
    rc::Rc,
};

/// Constants for result file extension
const DEP_EXTENSION: &str = "dep.dot";
const CG_EXTENSION: &str = "cg.dot";
const UNKNOWN_EXTENSION: &str = "unknown";

/// Commands that we support while querying the package.
#[derive(Clone, Debug, Args)]
#[group(id = "cmd", required = true, multiple = false)]
pub struct QuerierCommand {
    /// Dump the dependency graph from the bytecode/package
    #[arg(long, group = "cmd")]
    dump_dep_graph: bool,
    /// Dump the call graph from the bytecode/package
    #[arg(long, group = "cmd")]
    dump_call_graph: bool,
}

impl QuerierCommand {
    pub fn new(dump_dep_graph: bool, dump_call_graph: bool) -> Self {
        Self {
            dump_dep_graph,
            dump_call_graph,
        }
    }

    /// get a proper extension for the query results.
    pub fn extension(&self) -> &'static str {
        if self.dump_dep_graph {
            return DEP_EXTENSION;
        }
        if self.dump_call_graph {
            return CG_EXTENSION;
        }
        UNKNOWN_EXTENSION
    }
}

pub struct Querier {
    cmd: QuerierCommand,
    target_modules: Vec<ModuleId>,
    env: GlobalEnv,
}

impl Querier {
    pub fn new(cmd: QuerierCommand) -> Self {
        Self {
            cmd,
            target_modules: Vec::<ModuleId>::new(),
            env: GlobalEnv::new(),
        }
    }

    /// Load a set of bytecode files into move model
    pub fn load_package(&mut self, package: &Vec<PathBuf>, source_map_path: PathBuf) -> Result<()> {
        for bytecode_path in package {
            let bytecode_bytes = std::fs::read(bytecode_path)
                .with_context(|| format!("reading `{}`", bytecode_path.display()))?;

            let file_name = bytecode_path
                .file_name()
                .expect("file name of bytecode required");
            let mut source_map_file = source_map_path.clone();
            source_map_file.push(file_name);
            source_map_file
                .as_path()
                .with_extension(SOURCE_MAP_EXTENSION);
            let source_map = self.get_source_map(
                bytecode_path.to_str().unwrap_or_default(),
                &source_map_file,
                &bytecode_bytes,
            );

            let module = if let Ok(script) = CompiledScript::deserialize(&bytecode_bytes) {
                convert_script_to_module(script, self.env.get_module_count())
            } else if let Ok(module) = CompiledModule::deserialize(&bytecode_bytes) {
                module
            } else {
                return Err(anyhow!(format!(
                    "Error: `{}` cannot be deserialized as a module or script",
                    bytecode_path.display()
                )));
            };

            move_bytecode_verifier::verify_module(&module)?;
            let module_id = self
                .env
                .load_compiled_module(/*with_dep_closure*/ true, module, source_map);
            if self.env.has_errors() {
                return Err(anyhow!(format!("Error: cannot load package")));
            }
            self.target_modules.push(module_id);
        }
        Ok(())
    }

    /// add a CompiledModule into move model
    pub fn add_module(&mut self, module: CompiledModule, source_map: SourceMap) -> Result<()> {
        move_bytecode_verifier::verify_module(&module)?;
        let module_id = self
            .env
            .load_compiled_module(/*with_dep_closure*/ true, module, source_map);
        if self.env.has_errors() {
            return Err(anyhow!(format!("Error: cannot load package")));
        }
        self.target_modules.push(module_id);
        Ok(())
    }

    /// Interface to run query commands
    /// All modules in the move model are processed as a package together
    /// Currently, only support dump-call-graph and dump-dependency graph
    pub fn query(&self) -> Result<String> {
        if self.cmd.dump_dep_graph {
            let dep_graph: QueryGraph<ModuleId, String> = self.build_dep_graph()?;

            if dep_graph.graph.node_count() == 0 {
                return Ok("".to_string());
            }
            let node_attr = |_graph: &DiGraph<ModuleId, String>, node: (NodeIndex, &ModuleId)| {
                let mod_env = self.env.get_module(*node.1);
                let mod_name = self.format_module_name(&mod_env);
                format!("label=\"{}\", shape=box, color=blue", mod_name)
            };
            let edge_attr = |_, _| "label=\"\"".to_string();

            let dot = Dot::with_attr_getters(
                &dep_graph.graph,
                &[Config::NodeIndexLabel],
                &edge_attr,
                &node_attr,
            );

            return Ok(format!("{:?}", dot));
        }

        if self.cmd.dump_call_graph {
            let call_graph: QueryGraph<QualifiedId<FunId>, String> = self.build_call_graph()?;
            if call_graph.graph.node_count() == 0 {
                return Ok("".to_string());
            }
            let node_attr = |_graph: &DiGraph<QualifiedId<FunId>, String>,
                             node: (NodeIndex, &QualifiedId<FunId>)| {
                let func_env = self.env.get_function(*node.1);
                let func_name = self.format_function_name(&func_env, true);
                format!("label=\"{}\", shape=box, color=blue", func_name)
            };
            let edge_attr = |_, _| "label=\"\"".to_string();

            let dot = Dot::with_attr_getters(
                &call_graph.graph,
                &[Config::NodeIndexLabel],
                &edge_attr,
                &node_attr,
            );
            return Ok(format!("{:?}", dot));
        }

        unreachable!("Not supported");
    }

    /// Build a cross-module dependency graph for the move model
    /// Results are represented as a DiGraph
    fn build_dep_graph(&self) -> Result<QueryGraph<ModuleId, String>> {
        let mut dep_graph = QueryGraph::<ModuleId, String>::new();
        let mut visited: BTreeSet<ModuleId> = BTreeSet::new();
        let mut queue: VecDeque<ModuleId> = VecDeque::new();

        for module_id in self.target_modules.iter() {
            dep_graph.add_node(*module_id);
            queue.push_back(*module_id);
            visited.insert(*module_id);
        }

        while let Some(module_id) = queue.pop_front() {
            let mod_env = self.env.get_module(module_id);
            let dep_list = mod_env.get_used_modules(false).clone();
            for dep_id in dep_list.iter().filter(|dep_id| **dep_id != module_id) {
                if !visited.contains(dep_id) {
                    visited.insert(*dep_id);
                    queue.push_back(*dep_id);
                    dep_graph.add_node(*dep_id);
                }
                dep_graph.add_edge(module_id, *dep_id, "".to_string());
            }
        }
        Ok(dep_graph)
    }

    /// Build a cross-module call graph for the move model
    /// Results are represented as a DiGraph
    fn build_call_graph(&self) -> Result<QueryGraph<QualifiedId<FunId>, String>> {
        let mut call_graph = QueryGraph::<QualifiedId<FunId>, String>::new();
        let mut visited: BTreeSet<QualifiedId<FunId>> = BTreeSet::new();
        let mut queue: VecDeque<QualifiedId<FunId>> = VecDeque::new();

        for module_id in self.target_modules.iter() {
            let mod_env = self.env.get_module(*module_id);
            let func_list = mod_env.get_functions();
            for function in func_list.into_iter() {
                let func_qid = function.get_qualified_id();
                call_graph.add_node(func_qid);
                visited.insert(func_qid);
                queue.push_back(func_qid);
            }
        }

        while let Some(function_qid) = queue.pop_front() {
            let func_env = self.env.get_function(function_qid);
            let callee_list = func_env.get_used_functions();
            if let Some(callees) = callee_list {
                for callee in callees.iter() {
                    if !visited.contains(callee) {
                        call_graph.add_node(*callee);
                        visited.insert(*callee);
                        queue.push_back(*callee);
                    }
                    call_graph.add_edge(function_qid, *callee, "".to_string());
                }
            }
        }
        Ok(call_graph)
    }

    fn get_source_map(
        &mut self,
        input_path: &str,
        source_map_file: &PathBuf,
        bytes: &[u8],
    ) -> SourceMap {
        match fs::read(source_map_file) {
            Ok(bytes) => bcs::from_bytes::<SourceMap>(&bytes)
                .unwrap_or_else(|_| self.empty_source_map(input_path, &bytes)),
            _ => self.empty_source_map(input_path, bytes),
        }
    }

    pub fn empty_source_map(&mut self, name: &str, unique_bytes: &[u8]) -> SourceMap {
        let file_id = self.env.add_source(
            FileHash::new_from_bytes(unique_bytes),
            Rc::new(BTreeMap::new()),
            name,
            "",
            true,
            true,
        );
        let loc = Loc::new(file_id, Span::new(0, 0));
        SourceMap::new(self.env.to_ir_loc(&loc), None)
    }

    /// Format a module name for printing
    /// Module addresses are truncated to 8-bytes hex values (if longer)
    fn format_module_name(&self, module: &ModuleEnv<'_>) -> String {
        let mod_name = module.get_name();
        let addr = format!("{}", self.env.display(mod_name.addr()));
        let name = format!("{}", self.env.display(&mod_name.name()));
        let short_addr = if addr.starts_with("0x") && addr.len() > 10 {
            &addr[..10]
        } else {
            &addr
        };
        format!("{}::{}", short_addr, name)
    }

    /// Format a module name for printing
    /// Function typed parameters, arguments, and return types are suppressed
    ///     for better printing
    fn format_function_name(&self, function: &FunctionEnv<'_>, print_module: bool) -> String {
        if print_module {
            format!(
                "{}::{}",
                self.format_module_name(&function.module_env),
                function.get_name_str()
            )
        } else {
            function.get_name_str()
        }
    }
}

/// A generic struct to support query results that shall be represented as a graph.
/// nodes: keep all the nodes
/// graph: graph represented as a DiGraph

pub struct QueryGraph<T, E>
where
    T: Eq + Clone + Hash,
    E: Eq + Clone + Hash,
{
    nodes: HashMap<T, NodeIndex>,
    graph: DiGraph<T, E>,
}

impl<T, E> Default for QueryGraph<T, E>
where
    T: Eq + Clone + Hash,
    E: Eq + Clone + Hash,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T, E> QueryGraph<T, E>
where
    T: Eq + Clone + Hash,
    E: Eq + Clone + Hash,
{
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            graph: DiGraph::new(),
        }
    }

    fn add_node(&mut self, val: T) {
        if let Entry::Vacant(e) = self.nodes.entry(val) {
            let node_index = self.graph.add_node(e.key().clone());
            e.insert(node_index);
        }
    }

    fn add_edge(&mut self, src: T, dst: T, weight: E) {
        if let (Some(src_index), Some(dst_index)) = (self.nodes.get(&src), self.nodes.get(&dst)) {
            self.graph.add_edge(*src_index, *dst_index, weight);
        }
    }
}
