// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Result};
use clap::Args;
use move_binary_format::{
    binary_views::BinaryIndexedView,
    file_format::{
        Bytecode, CodeUnit, CompiledScript, FunctionDefinitionIndex, FunctionHandleIndex,
        FunctionInstantiationIndex, ModuleHandle, SignatureIndex, SignatureToken, TableIndex,
        Visibility,
    },
    CompiledModule,
};
use move_core_types::{
    ability::{Ability, AbilitySet},
    language_storage::ModuleId,
};
use petgraph::{
    graph::{DiGraph, NodeIndex},
    visit::EdgeRef,
};
use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    fmt::Display,
    hash::Hash,
};

//Constants for result file extension
const CG_EXTENSION: &str = "mv.cg.dot";
const TYPE_EXTENSION: &str = "mv.type";
const UNKNOWN_EXTENSION: &str = "mv.unknown";

//Constants for bytecode type
const SCRIPT_CODE: &str = "script";
const MODULE_CODE: &str = "module";
const _UNKNOWN_CODE: &str = "unknown";

//Constants for function signatures
const MAIN_FUNCTION: &str = "main";
const SCRIPT_NAME: &str = "script";
const ENTRY_MODIFIER: &str = "entry ";
const NATIVE_MODIFIER: &str = "native ";
const PUBLIC_FUNCTION: &str = "public ";
const FRIEND_FUNCTION: &str = "public(friend) ";
const PRIVATE_FUNCTION: &str = "";
const EMPTY: &str = "";

/// Holds the commands that we support while querying the bytecode.
#[derive(Clone, Debug, Args)]
#[group(id = "cmd", required = false, multiple = false)]
pub struct QuerierOptions {
    /// Dump the call graph(s) from bytecode, which should only be used together with the query tool
    #[arg(long, group = "cmd")]
    dump_call_graph: bool,
    /// Check the type of the bytecode (`script`, `module`, or `unknown`), which should only be used together with the query tool
    #[arg(long, group = "cmd")]
    check_bytecode_type: bool,
}

impl QuerierOptions {
    pub fn new(dump_call_graph: bool, check_bytecode_type: bool) -> Self {
        Self {
            dump_call_graph,
            check_bytecode_type,
        }
    }

    /// check if a valid query command is provided.
    pub fn has_any_true(&self) -> bool {
        self.dump_call_graph || self.check_bytecode_type
    }

    /// get a proper extension for the query results.
    pub fn extension(&self) -> &'static str {
        if self.dump_call_graph {
            return CG_EXTENSION;
        }
        if self.check_bytecode_type {
            return TYPE_EXTENSION;
        }
        UNKNOWN_EXTENSION
    }
}

/// Represents an instance of a querier. It now supports dumping call graphs and checking bytecode tyoe.
pub struct Querier {
    options: QuerierOptions,
    bytecode_bytes: Vec<u8>,
}

impl Querier {
    pub fn new(options: QuerierOptions, bytecode_bytes: Vec<u8>) -> Self {
        Self {
            options,
            bytecode_bytes,
        }
    }

    ///Main interface to run query commands.
    pub fn query(&self) -> Result<String> {
        let module: CompiledModule;
        let script: CompiledScript;

        //Get a binary view of the bytecode
        let bytecode = if let Ok(bytecode) = CompiledScript::deserialize(&self.bytecode_bytes) {
            script = bytecode;
            BinaryIndexedView::Script(&script)
        } else if let Ok(bytecode) = CompiledModule::deserialize(&self.bytecode_bytes) {
            module = bytecode;
            BinaryIndexedView::Module(&module)
        } else {
            return Err(anyhow!(
                "Error: The bytecode cannot be resolved as a module or a script"
            ));
        };

        if self.options.check_bytecode_type {
            let btchecker = BytecodeTypeChecker::new(bytecode);
            return btchecker.get_bytecode_type();
        }

        if self.options.dump_call_graph {
            let cgbuider = CGBuilder::new(bytecode);
            return cgbuider.dump_call_graph();
        }

        unreachable!("Not supported");
    }
}

/// Struct to support checking bytecode type
pub struct BytecodeTypeChecker<'view> {
    bytecode: BinaryIndexedView<'view>,
}

impl<'view> BytecodeTypeChecker<'view> {
    pub fn new(bytecode: BinaryIndexedView<'view>) -> Self {
        Self { bytecode }
    }

    /// Retrieve bytecode type
    pub fn get_bytecode_type(&self) -> Result<String> {
        match &self.bytecode {
            //need to check script first, as some scripts can be deserialized as modules
            // e.g., ./ecosystem/indexer-grpc/indexer-transaction-generator/imported_transactions/move_fixtures/fa_double_transfer/script.mv
            BinaryIndexedView::Script(_) => Ok(String::from(SCRIPT_CODE)),
            BinaryIndexedView::Module(_) => Ok(String::from(MODULE_CODE)),
        }
    }
}

/// Main struct to support building call graphs
/// bytecode: deserialized move bytecode
/// module_aliases: mapping from moduleid to unique name (identically-named modules at different addresses can exist)
pub struct CGBuilder<'view> {
    bytecode: BinaryIndexedView<'view>,
    module_aliases: HashMap<ModuleId, String>,
}

impl<'view> CGBuilder<'view> {
    pub fn new(bytecode: BinaryIndexedView<'view>) -> Self {
        let module_aliases = Self::create_module_alias(&bytecode);
        Self {
            bytecode,
            module_aliases,
        }
    }

    // Code borrowed from disassembler.
    // If the code being queried imports multiple modules of the same name,
    // `module_aliases`` will be used to create unique names for each module,
    // e.g., for `use 0xA::M; use 0xB::M`, this will contain [(0xA, M) -> M, (0xB, M) -> 1M]
    // Move identifiers cannot begin with an integer,
    // so this is guaranteed not to conflict with other module names.
    fn create_module_alias(bytecode: &BinaryIndexedView<'view>) -> HashMap<ModuleId, String> {
        let mut module_names: HashMap<String, i32> = HashMap::new();
        let mut module_aliases: HashMap<ModuleId, String> = HashMap::new();
        //rename the self module to 0{selfmodulename}
        module_names.extend(bytecode.self_id().map(|id| (id.name().to_string(), 0)));
        for h in bytecode.module_handles() {
            let id = bytecode.module_id_for_handle(h);
            let module_name = id.name().to_string();
            module_names
                .entry(module_name.clone())
                .and_modify(|name_count| {
                    // This module imports >1 modules named `name`-- add alias <count><module_name> for `id`.
                    module_aliases.insert(id, format!("{}{}", name_count, module_name));
                    *name_count += 1;
                })
                .or_insert(0);
        }
        module_aliases
    }

    fn dump_call_graph(&self) -> Result<String> {
        let call_graph: QueryGraph<CGNode, String> = match self.bytecode {
            BinaryIndexedView::Script(script) => self.dump_script_call_graph(script)?,
            BinaryIndexedView::Module(module) => self.dump_module_call_graph(module)?,
        };
        Ok(call_graph.print_sorted_dotgraph())
    }

    fn dump_script_call_graph(
        &self,
        script: &CompiledScript,
    ) -> Result<QueryGraph<CGNode, String>> {
        let mut call_graph = QueryGraph::<CGNode, String>::new();
        // add imported module info as graph description
        call_graph.add_label(self.get_module_description());
        // map to keep track of function signatures
        let mut func_sig_map = HashMap::<FunctionHandleIndex, String>::new();
        // add the entry function node
        let script_entry_node = CGNode {
            func_sig: self.format_function_sig(None, None, Some(script), &mut func_sig_map)?,
        };
        call_graph.add_node(script_entry_node.clone());
        // add child function nodes and edges accordingly
        let child_funcs = self.get_child_functions(Some(&script.code));
        for child in child_funcs.into_iter() {
            let (child_sig, edge_note) =
                self.format_child_function_sig(child, &mut func_sig_map)?;
            let child_node = CGNode {
                func_sig: child_sig,
            };
            call_graph.add_node(child_node.clone());
            call_graph.add_edge(script_entry_node.clone(), child_node.clone(), edge_note);
        }
        Ok(call_graph)
    }

    fn dump_module_call_graph(
        &self,
        module: &CompiledModule,
    ) -> Result<QueryGraph<CGNode, String>> {
        let mut call_graph = QueryGraph::<CGNode, String>::new();
        call_graph.add_label(self.get_module_description());
        let mut func_sig_map = HashMap::<FunctionHandleIndex, String>::new();

        // collect all the functions with definitions and save their signatures
        // Why: the signature for the same function with and without (access to) definition is NOT identicail
        //     e.g., entry, visibility, and native modifiers need access to func definition
        for func_idx in 0..module.function_defs.len() {
            let function_definition_index = FunctionDefinitionIndex(func_idx as TableIndex);
            let function_definition = self.bytecode.function_def_at(function_definition_index)?;
            let function_handle_idx = function_definition.function;
            // This function cache the func sig inside func_sig_map
            self.format_function_sig(
                Some(&function_handle_idx),
                Some(&function_definition_index),
                None,
                &mut func_sig_map,
            )?;
        }

        // process children of functions with definitions;
        // Warning: never merge this loop with the loop above, as merging will mess up the signature of functions
        for func_idx in 0..module.function_defs.len() {
            let function_definition_index = FunctionDefinitionIndex(func_idx as TableIndex);
            let function_definition = self.bytecode.function_def_at(function_definition_index)?;
            let function_handle_idx = function_definition.function;
            // add a node for the current function coming with definition
            let node_with_def = CGNode {
                func_sig: func_sig_map.get(&function_handle_idx).unwrap().clone(),
            };
            call_graph.add_node(node_with_def.clone());

            //process all its child functions
            let children = self.get_child_functions(function_definition.code.as_ref());
            for child in children.into_iter() {
                let (child_sig, edge_note) =
                    self.format_child_function_sig(child, &mut func_sig_map)?;
                let child_node = CGNode {
                    func_sig: child_sig,
                };
                call_graph.add_node(child_node.clone());
                call_graph.add_edge(node_with_def.clone(), child_node.clone(), edge_note);
            }
        }
        Ok(call_graph)
    }

    //***************************************************************************
    // Helper functions
    //***************************************************************************

    // Code copied from disassembler
    // Create a description of the module, including
    //  self_address::self_name
    //  use imported_module_address::imported_module_name
    fn get_module_description(&self) -> String {
        let mut description = String::new();
        match self.bytecode.self_id() {
            Some(module_id) => {
                let module_string = format!(
                    "Module: {}::{}\n",
                    module_id.address().to_hex(),
                    module_id.name.as_str()
                );
                description.push_str(&module_string);
            },
            None => {
                description.push_str("Script\n");
            },
        }

        let import_strings = self
            .bytecode
            .module_handles()
            .iter()
            .filter_map(|h| self.get_import_string(h))
            .collect::<Vec<String>>();

        description.push_str(import_strings.join("\n").as_str());
        description
    }

    fn get_import_string(&self, module_handle: &ModuleHandle) -> Option<String> {
        let module_id = self.bytecode.module_id_for_handle(module_handle);
        if self.is_self_id(&module_id) {
            // No need to import self handle
            None
        } else if let Some(alias) = self.module_aliases.get(&module_id) {
            Some(format!(
                "    use {}::{} as {};",
                module_id.address().to_hex(),
                module_id.name(),
                alias
            ))
        } else {
            Some(format!(
                "    use {}::{};",
                module_id.address().to_hex(),
                module_id.name()
            ))
        }
    }

    fn is_self_id(&self, mid: &ModuleId) -> bool {
        self.bytecode
            .self_id()
            .map(|id| &id == mid)
            .unwrap_or(false)
    }

    fn get_child_functions(&self, code: Option<&CodeUnit>) -> HashSet<ChildFuncType> {
        match code {
            Some(code) => code
                .code
                .iter()
                .filter_map(Self::disassemble_instruction)
                .collect::<HashSet<ChildFuncType>>(),
            None => HashSet::<ChildFuncType>::new(),
        }
    }

    fn disassemble_instruction(instruction: &Bytecode) -> Option<ChildFuncType> {
        match instruction {
            Bytecode::Call(func_idx) => Some(ChildFuncType::Function(*func_idx)),
            Bytecode::CallClosure(sig_idx) => Some(ChildFuncType::Closure(*sig_idx)),
            Bytecode::CallGeneric(func_init_idx) => {
                Some(ChildFuncType::GenericFunction(*func_init_idx))
            },
            _ => None,
        }
    }

    //***************************************************************************
    // Function signature format help functions
    //***************************************************************************

    // The call target could be a regular function, a generic function, or a closure.
    // Regular function: return its signatures
    // Generic function: return its signatures and a note indicating how the typed parameters are initialized by the caller
    // Closure: return the closure signature
    fn format_child_function_sig(
        &self,
        child: ChildFuncType,
        func_sig_map: &mut HashMap<FunctionHandleIndex, String>,
    ) -> Result<(String, String)> {
        let mut edge_note = String::from(EMPTY);
        let child_sig = match child {
            ChildFuncType::Function(func_idx) => {
                if func_sig_map.contains_key(&func_idx) {
                    func_sig_map.get(&func_idx).unwrap().clone()
                } else {
                    // cannot be a script entry point;
                    // cannot be a function with definition (all those have been added into the sig map)
                    self.format_function_sig(Some(&func_idx), None, None, func_sig_map)?
                }
            },
            // call a closure
            ChildFuncType::Closure(closure_idx) => {
                let closure_sig = self.bytecode.signature_at(closure_idx);
                let type_str = if closure_sig.len() != 1 {
                    String::from("<error: invalid closure signature>")
                } else {
                    self.format_sig_token(&closure_sig.0[0])?
                };
                format!("CallClosure({})", type_str)
            },
            //call a generic function
            ChildFuncType::GenericFunction(func_init_idx) => {
                let func_init = self.bytecode.function_instantiation_at(func_init_idx);
                let func_handle_idx = func_init.handle;
                edge_note = self.format_typarams_init(func_init.type_parameters)?;
                if func_sig_map.contains_key(&func_handle_idx) {
                    func_sig_map.get(&func_handle_idx).unwrap().clone()
                } else {
                    self.format_function_sig(Some(&func_handle_idx), None, None, func_sig_map)?
                }
            },
        };
        Ok((child_sig, edge_note))
    }

    // Create a signature for a given function
    //  Script Entry Function: No Function Handle, No Function Definition, Only Script
    //  Script Non-Entry Function: Yes Function Handle, No Function Definition, No Script
    //  Module Internal Function: Yes Function Handle, Yes Function Definition, No Script
    //  Module External/Native/Imported Function: Yes Function Handle, No Function Definition, No Script
    fn format_function_sig(
        &self,
        fh: Option<&FunctionHandleIndex>,
        fd: Option<&FunctionDefinitionIndex>,
        s: Option<&CompiledScript>,
        func_sig_map: &mut HashMap<FunctionHandleIndex, String>,
    ) -> Result<String> {
        // Function handler is given
        if let Some(function_handle_idx) = fh {
            if func_sig_map.contains_key(function_handle_idx) {
                return Ok(func_sig_map.get(function_handle_idx).unwrap().clone());
            }

            let function_handle = self.bytecode.function_handle_at(*function_handle_idx);
            let (entry_modifier, native_modifier, visibility_modifier) = match fd {
                // function definition provided
                Some(function_def_index) => {
                    let function_def = self.bytecode.function_def_at(*function_def_index)?;
                    let entry = if function_def.is_entry {
                        ENTRY_MODIFIER
                    } else {
                        EMPTY
                    };
                    let native = if function_def.is_native() {
                        NATIVE_MODIFIER
                    } else {
                        EMPTY
                    };
                    let visibility = match function_def.visibility {
                        Visibility::Private => PRIVATE_FUNCTION,
                        Visibility::Friend => FRIEND_FUNCTION,
                        Visibility::Public => PUBLIC_FUNCTION,
                    };
                    (entry, native, visibility)
                },
                // function definition not avaialble
                _ => (EMPTY, EMPTY, EMPTY),
            };

            let module_handle = self.bytecode.module_handle_at(function_handle.module);
            let module_id = self.bytecode.module_id_for_handle(module_handle);
            let module_name: String = if self.is_self_id(&module_id) {
                String::from("")
            } else {
                format!(
                    "{}::",
                    self.module_aliases
                        .get(&module_id)
                        .cloned()
                        .unwrap_or_else(|| module_id.name().to_string())
                )
            };
            let function_name: &str = self.bytecode.identifier_at(function_handle.name).as_str();
            let ty_params: String = self.format_type_params(&function_handle.type_parameters)?;
            let params: String = self.format_func_params(function_handle.parameters)?;
            let ret_type: String = self.format_func_rets(function_handle.return_)?;
            let res = format!(
                "{}{}{}{}{}{}{}{}",
                entry_modifier,
                native_modifier,
                visibility_modifier,
                module_name,
                function_name,
                ty_params,
                params,
                ret_type
            );
            //Cache the function sig;
            //  not just for efficiency, but also for avoiding creating different func sigs for the same function
            func_sig_map.insert(*function_handle_idx, res.clone());
            return Ok(res);
        }

        // If we get to this point, the function must be the script entry function
        // We never need to worry about caching the function sig
        if let Some(script) = s {
            let entry_modifier = ENTRY_MODIFIER;
            let module_name = SCRIPT_NAME;
            let function_name = MAIN_FUNCTION;
            let ty_params: String = self.format_type_params(&script.type_parameters)?;
            let params: String = self.format_func_params(script.parameters)?;
            return Ok(format!(
                "{}{}::{}{}{}",
                entry_modifier, module_name, function_name, ty_params, params
            ));
        }
        Ok(String::from(""))
    }

    fn format_type_params(&self, ablities: &[AbilitySet]) -> Result<String> {
        let format_ability = |a: Ability| {
            match a {
                Ability::Copy => "copy",
                Ability::Drop => "drop",
                Ability::Store => "store",
                Ability::Key => "key",
            }
            .to_string()
        };

        if ablities.is_empty() {
            Ok(String::from(EMPTY))
        } else {
            let generic_strs: Vec<String> = ablities
                .iter()
                .enumerate()
                .map(|(index, ability)| {
                    let ability_strs = if *ability == AbilitySet::EMPTY {
                        String::from("")
                    } else {
                        let one_ability_str: Vec<String> =
                            ability.into_iter().map(format_ability).collect();
                        format!(": {}", one_ability_str.join(" + "))
                    };
                    format!("T{}{}", index, ability_strs)
                })
                .collect();
            Ok(format!("<{}>", generic_strs.join(", ")))
        }
    }

    fn format_typarams_init(&self, typarams_sig_idx: SignatureIndex) -> Result<String> {
        let typarams: Vec<String> = self
            .bytecode
            .signature_at(typarams_sig_idx)
            .0
            .iter()
            .enumerate()
            .map(|(index, tok)| Ok(format!("{} as T{}", self.format_sig_token(tok)?, index)))
            .collect::<Result<Vec<String>>>()?;

        if typarams.is_empty() {
            Ok(String::from(EMPTY))
        } else {
            Ok(format!("[{}]", typarams.join(", ")))
        }
    }

    fn format_func_params(&self, para_sig_idx: SignatureIndex) -> Result<String> {
        let params: Vec<String> = self
            .bytecode
            .signature_at(para_sig_idx)
            .0
            .iter()
            .map(|token| self.format_sig_token(token))
            .collect::<Result<Vec<String>>>()?;

        Ok(format!("({})", params.join(", ")))
    }

    fn format_func_rets(&self, ret_sig_idx: SignatureIndex) -> Result<String> {
        let rets: Vec<String> = self
            .bytecode
            .signature_at(ret_sig_idx)
            .0
            .iter()
            .map(|token| self.format_sig_token(token))
            .collect::<Result<Vec<String>>>()?;

        if rets.is_empty() {
            Ok(String::from(EMPTY))
        } else {
            Ok(format!(" -> {}", rets.join(", ")))
        }
    }

    // code mostly copied from the move disassembler
    fn format_sig_token(&self, token: &SignatureToken) -> Result<String> {
        let list_sig_vec = |toks: &Vec<SignatureToken>| {
            toks.iter()
                .map(|tok| self.format_sig_token(tok))
                .collect::<Result<Vec<String>>>()
        };
        Ok(match token {
            SignatureToken::Bool => String::from("bool"),
            SignatureToken::U8 => String::from("u8"),
            SignatureToken::U16 => String::from("u16"),
            SignatureToken::U32 => String::from("u32"),
            SignatureToken::U64 => String::from("u64"),
            SignatureToken::U128 => String::from("u128"),
            SignatureToken::U256 => String::from("u256"),
            SignatureToken::Address => String::from("address"),
            SignatureToken::Signer => String::from("signer"),
            SignatureToken::Vector(inner) => format!("vector<{}>", self.format_sig_token(inner)?),
            SignatureToken::Struct(idx) => self
                .bytecode
                .identifier_at(self.bytecode.struct_handle_at(*idx).name)
                .to_string(),
            SignatureToken::TypeParameter(idx) => format!("T{}", idx),
            SignatureToken::Reference(sig_tok) => format!("&{}", self.format_sig_token(sig_tok)?),
            SignatureToken::MutableReference(sig_tok) => {
                format!("&mut {}", self.format_sig_token(sig_tok)?)
            },
            SignatureToken::StructInstantiation(idx, type_args) => {
                let name = self
                    .bytecode
                    .identifier_at(self.bytecode.struct_handle_at(*idx).name)
                    .to_string();
                let instantiation = list_sig_vec(type_args)?;
                if instantiation.is_empty() {
                    name
                } else {
                    format!("{}<{}>", name, instantiation.join(", "))
                }
            },
            SignatureToken::Function(args, results, abilities) => {
                format!(
                    "|{}|{}{}",
                    list_sig_vec(args)?.join(","),
                    list_sig_vec(results)?.join(","),
                    abilities.display_postfix()
                )
            },
        })
    }
}

#[derive(Clone, Hash, Eq, PartialEq, PartialOrd, Ord)]
enum ChildFuncType {
    Function(FunctionHandleIndex),
    GenericFunction(FunctionInstantiationIndex),
    Closure(SignatureIndex),
}

/// A generic struct to support query results that shall be represented as a graph.
/// nodes: keep all the nodes
/// graph: graph represented as a DiGraph
/// label: title/description to add into the graph
pub struct QueryGraph<T, E>
where
    T: Eq + Clone + Hash + Display + Ord,
    E: Eq + Clone + Hash + Display + Ord,
{
    nodes: HashMap<T, NodeIndex>,
    graph: DiGraph<T, E>,
    label: String,
}

impl<T, E> Default for QueryGraph<T, E>
where
    T: Eq + Clone + Hash + Display + Ord,
    E: Eq + Clone + Hash + Display + Ord,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T, E> QueryGraph<T, E>
where
    T: Eq + Clone + Hash + Display + Ord,
    E: Eq + Clone + Hash + Display + Ord,
{
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            graph: DiGraph::new(),
            label: String::new(),
        }
    }

    fn add_label(&mut self, label: String) {
        self.label = label;
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

    // Print the graph in DOT format
    // The graph is sorted by node labels and edges are sorted by source, target, and weight
    // Why not using  dot = Dot::with_attr_getters: the node indexes are not deterministic from run to run, making testing impossible.
    fn print_sorted_dotgraph(self) -> String {
        //config graph label
        let mut output = String::new();
        output.push_str("digraph G {\n");
        output.push_str(&format!("    label = \"{}\";\n", self.label));
        output.push_str("    labelloc = \"t\";\n");
        output.push_str("    labeljust = \"l\";\n");
        output.push_str("    fontsize = 36;\n\n");

        //config graph settings
        output.push_str(
            "    graph [\n\
            \trankdir=LR,\n\
            \tranksep=1.0,\n\
            \tnodesep=0.75,\n\
            \tsplines=true,\n\
            \tconcentrate=false\n    ];\n\n",
        );

        //config node
        output.push_str(
            "    node [\n\
            \tshape=ellipse,\n\
            \tfontsize=24\n    ];\n\n",
        );

        //config edge
        output.push_str(
            "    edge [\n\
            \tarrowsize=1.5\n    ];\n\n",
        );

        let mut nodeidx_to_idx: HashMap<NodeIndex, usize> = HashMap::new();

        // Sort nodes by their labels
        let mut nodes: Vec<_> = self.graph.node_indices().collect();
        nodes.sort_by_key(|node| &self.graph[*node]);

        // Create a mapping from node indices to their sorted order
        for (idx, node) in nodes.iter().enumerate() {
            nodeidx_to_idx.insert(*node, idx);
            let label = &self.graph[*node];
            output.push_str(&format!("    {} [label=\"{}\"];\n", idx, label));
        }

        // Sort edges by source, target, and weight
        let mut edges: Vec<_> = self.graph.edge_references().collect();
        edges.sort_by_key(|edge| {
            (
                nodeidx_to_idx.get(&edge.source()).unwrap(),
                nodeidx_to_idx.get(&edge.target()).unwrap(),
                format!("{}", edge.weight()),
            )
        });

        // Print edges by replacing node indices with their sorted order
        for edge in edges {
            output.push_str(&format!(
                "    {} -> {} [label=\"{}\", fontsize=24];\n",
                nodeidx_to_idx.get(&edge.source()).unwrap(),
                nodeidx_to_idx.get(&edge.target()).unwrap(),
                edge.weight()
            ));
        }

        output.push_str("}\n");
        output
    }
}

// node structure for call graphs
#[derive(Clone, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub struct CGNode {
    func_sig: String,
}

// Custom Display implementation for DOT output
impl std::fmt::Display for CGNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.func_sig)
    }
}
