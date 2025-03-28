// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright (c) The Aptos Labs
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Result};
use clap::Args;
use move_binary_format::{
    binary_views::BinaryIndexedView, file_format::{Bytecode, CodeUnit, CompiledScript, FunctionDefinitionIndex, FunctionHandleIndex, SignatureToken, TableIndex, Visibility}, CompiledModule
};
use petgraph::dot::{Dot, Config};
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

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
#[derive(Copy, Clone, Debug, Args)]
#[group(id = "cmd", required = false, multiple = false)]
pub struct QuerierOptions {
    /// Dump the call graph(s) from bytecode, which should only be used together with `aptos move query`
    #[arg(long, group = "cmd")]
    dump_call_graph: bool,
    /// Check the type of the bytecode (`script`, `module`, or `unknown`), which should only be used together with `aptos move query`
    #[arg(long, group = "cmd")]
    check_bytecode_type: bool,
}

impl QuerierOptions {
    // check if a valid query command is provided
    pub fn has_any_true(&self) -> bool {
        self.dump_call_graph || self.check_bytecode_type
    }
    // get a proper extension for the query results
    pub fn extension(&self) -> &'static str {
        if self.dump_call_graph {
            return CG_EXTENSION;
        }
        if self.check_bytecode_type{
            return TYPE_EXTENSION;
        }
       UNKNOWN_EXTENSION
    }
}

/// Represents an instance of a querier. The querier now supports dumping call graphs and checking bytecode tyoe
pub struct Querier {
    options: QuerierOptions,
    bytecode_bytes: Vec<u8>,
}

impl Querier {
    /// Creates a new querier.
    pub fn new(options: QuerierOptions, bytecode_bytes: Vec<u8>) -> Self {
        Self {
            options,
            bytecode_bytes,
        }
    }

    ///Main interface to run the query actions
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
                "Query Error: The bytecode cannot be resolved as either a module or a script"
            ));
        };

        if self.options.dump_call_graph {
            let cgbuider = CGBuilder::new(bytecode);
            return cgbuider.dump_call_graph();
        }

        if self.options.check_bytecode_type {
            let btchecker = BytecodeTypeChecker::new(bytecode);
            return btchecker.get_bytecode_type();
        }

        unreachable!("Not supported");
    }

}

pub struct BytecodeTypeChecker<'view> {
    bytecode: BinaryIndexedView<'view>,
}

impl<'view> BytecodeTypeChecker<'view>{
    pub fn new(bytecode: BinaryIndexedView<'view>) ->Self {
        Self {
            bytecode,
        }
    }

    pub fn get_bytecode_type(&self) -> Result<String> {
        match &self.bytecode {
            BinaryIndexedView::Script(_) => {
                Ok(String::from(SCRIPT_CODE))
            }
            BinaryIndexedView::Module(_) => {
                Ok(String::from(MODULE_CODE))
            }
        }
    }
}

// node structure for call graphs
#[derive(Clone, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub struct CGNode{
    func_sig: String,
}

impl std::fmt::Debug for CGNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Print nothing here to avoid the default label.
        write!(f, "")
    }
}

pub struct QueryGraph<T: Eq + Clone + Hash>{
    nodes: HashMap<T, NodeIndex>,
    graph: DiGraph<T, ()>,
}

impl<T: Eq + Clone + Hash> QueryGraph<T> {
    pub fn new() -> Self {
       Self{
        nodes: HashMap::new(),
        graph: DiGraph::new(),
       }
    }

    fn add_node(&mut self, val: T) {
        if !self.nodes.contains_key(&val){
            let node_index = self.graph.add_node(val.clone());
            self.nodes.insert(val, node_index);
        }
    }

    fn add_edge(&mut self, src: T, dst: T) {
        if let (Some(src_index), Some(dst_index)) = (self.nodes.get(&src), self.nodes.get(&dst)) {
            self.graph.add_edge(*src_index, *dst_index, ());
        }
    }
}

pub struct CGBuilder<'view> {
    bytecode: BinaryIndexedView<'view>,
}

impl<'view> CGBuilder<'view>{
    pub fn new(bytecode: BinaryIndexedView<'view>) ->Self {
        Self {
            bytecode,
        }
    }

    fn dump_call_graph(&self) -> Result<String> {
        let call_graph: QueryGraph<CGNode> = match &self.bytecode {
             BinaryIndexedView::Script(script) => {
                 self.dump_script_call_graph(*script)?
             }
             BinaryIndexedView::Module(module) => {
                 self.dump_module_call_graph(*module)?
             }
         };

         let dot = Dot::with_attr_getters(
            &call_graph.graph,
            &[Config::EdgeNoLabel],
             &|_graph, _| "".to_string(),
             &|_graph, (_, node)| format!("label=\"{}\"", node.func_sig),
        );
         Ok(format!("{:?}", dot))
     }

     fn dump_script_call_graph(&self, script: &CompiledScript) -> Result<QueryGraph<CGNode>> {
        let mut call_graph = QueryGraph::<CGNode>::new();
        let mut func_sig_map = HashMap::<FunctionHandleIndex, String>::new();

        // add the entry function node
        let script_entry_node = CGNode{
            func_sig: self.format_function_sig(None, None, Some(script), &mut func_sig_map)?
        };
        call_graph.add_node(script_entry_node.clone());

        // add child function nodes and edges accordingly
        let child_funcs = self.get_child_functions(Some(&script.code));
        for func in child_funcs.into_iter(){
            let child_node = CGNode{
                func_sig: self.format_function_sig(Some(&func), None, Some(script), &mut func_sig_map)?
            };
            call_graph.add_node(child_node.clone());
            call_graph.add_edge(script_entry_node.clone(), child_node.clone());
        }
        Ok(call_graph)
    }

    fn dump_module_call_graph(&self, module: &CompiledModule) -> Result<QueryGraph<CGNode>> {
        let mut call_graph = QueryGraph::<CGNode>::new();
        let mut func_sig_map = HashMap::<FunctionHandleIndex, String>::new();

        // collect all the functions with definitions and save their signatures
        // Why: the function signature for a defined function and a non-defined function is NOT identicail
        for func_idx in (0..module.function_defs.len()).into_iter(){
                let function_definition_index = FunctionDefinitionIndex(func_idx as TableIndex);
                let function_definition  = self.bytecode.function_def_at(function_definition_index)?;
                let function_handle_idx = function_definition.function;

                // This function cache the func sig inside func_sig_map
                self.format_function_sig(Some(&function_handle_idx), Some(&function_definition_index), None, &mut func_sig_map)?;
        };

        // process children of functions with definitions;
        // Warning: never merge this loop with the loop above, as merging will mess up the signature of functions
        for func_idx in (0..module.function_defs.len()).into_iter(){
            let function_definition_index = FunctionDefinitionIndex(func_idx as TableIndex);
            let function_definition  = self.bytecode.function_def_at(function_definition_index)?;
            let function_handle_idx = function_definition.function;

            //create a node for the current function
            let node_with_def = CGNode{
                func_sig: func_sig_map.get(&function_handle_idx).unwrap().clone(),
            };
            call_graph.add_node(node_with_def.clone());

            let child_funcs = self.get_child_functions(function_definition.code.as_ref());
            for func in child_funcs.into_iter(){
                let child_node = CGNode{
                    // Signature has been created
                    func_sig: if func_sig_map.contains_key(&func){
                        func_sig_map.get(&function_handle_idx).unwrap().clone()
                    }else{
                        self.format_function_sig(Some(&func), None, None, &mut func_sig_map)?
                    }
                };
                call_graph.add_node(child_node.clone());
                call_graph.add_edge(node_with_def.clone(), child_node.clone());
            }
        };
        Ok(call_graph)
    }

    fn get_child_functions(&self, code: Option<&CodeUnit>) -> HashSet<FunctionHandleIndex>{
        match code {
            Some(code) => {
                code.code
                .iter()
                .filter_map(|inst| self.disassemble_instruction(inst)) // returns Option<FunctionHandleIndex>
                .collect::<HashSet<FunctionHandleIndex>>()
            },
            None => HashSet::<FunctionHandleIndex>::new(),
        }
    }

    fn disassemble_instruction(&self,  instruction: &Bytecode) -> Option<FunctionHandleIndex> {
        match instruction {
            Bytecode::Call(method_idx) => Some(*method_idx),
            _ => None,
        }
    }

    // Create a signature for a given function
    //  Script Entry Function: No Function Handle, No Function Definition, Only Script
    //  Script Non-Entry Function: Yes Function Handle, No Function Definition, No Script
    //  Module Internal Function: Yes Function Handle, Yes Function Definition, No Script
    //  Module External/Native/Imported Function: Yes Function Handle, No Function Definition, No Script
    fn format_function_sig(&self, fh: Option<&FunctionHandleIndex>, fd: Option<&FunctionDefinitionIndex>,
        s: Option<&CompiledScript>, func_sig_map: &mut HashMap<FunctionHandleIndex, String>) -> Result<String> {
        // Function handler is given
        if let Some(function_handle_idx) = fh {
            //Cache the function sig; not just for efficiency, but also for avoiding creating different func sigs for the same function (defined by function handle index)
            if func_sig_map.contains_key(function_handle_idx){
                return Ok(func_sig_map.get(function_handle_idx).unwrap().clone());
            }

            let function_handle = self.bytecode.function_handle_at(*function_handle_idx);

            let (entry_modifier, native_modifier, visibility_modifier) = match fd{
                // function definition avaialble
                Some(function_def_index) => {
                    let function_def = self.bytecode.function_def_at(*function_def_index)?;
                    let entry = if function_def.is_entry{ ENTRY_MODIFIER } else { EMPTY };
                    let native = if function_def.is_native(){ NATIVE_MODIFIER } else { EMPTY };
                    let visibility = match function_def.visibility{
                        Visibility::Private => PRIVATE_FUNCTION,
                        Visibility::Friend => FRIEND_FUNCTION,
                        Visibility::Public => PUBLIC_FUNCTION
                    };
                    (entry, native, visibility)
                }
                // function definition not avaialble
                 _ => { (EMPTY, EMPTY, EMPTY) }
            };

            let module_handle = self.bytecode.module_handle_at(function_handle.module);
            let module_id = self.bytecode.module_id_for_handle(module_handle);
            let module_name = module_id.name.as_str();
            let function_name = self.bytecode.identifier_at(function_handle.name).as_str();
            let ty_params: &str = EMPTY;
            let params: Vec<String>= self.bytecode.signature_at(function_handle.parameters).0.iter().map(|token|self.format_sig_token(token)).collect();
            let ret_type: Vec<String>= self.bytecode.signature_at(function_handle.return_).0.iter().map(|token|self.format_sig_token(token)).collect();

            let res = format!("{}{}{}{}::{}{}{}{}", entry_modifier, native_modifier, visibility_modifier, module_name, function_name, ty_params, params.join(", "), ret_type.join(", "));
            func_sig_map.insert(*function_handle_idx, res.clone());
            return Ok(res);
        }

        // If we get to this point, the function must be the script entry function
        // We never need to worry about caching the function sig
        if let Some(script) = s {
            let entry_modifier = ENTRY_MODIFIER;
            let module_name = SCRIPT_NAME;
            let function_name = MAIN_FUNCTION;
            let params: Vec<String> = self.bytecode.signature_at(script.parameters).0.iter().map(|token|self.format_sig_token(token)).collect();
            return Ok(format!("{}{}::{}({})", entry_modifier, module_name, function_name, params.join(", ")));
        }
        Ok(String::from(""))
    }

    // code mostly copied from the move disassembler code
    fn format_sig_token(&self, token: &SignatureToken) -> String {
        match token {
            SignatureToken::Bool => "bool".to_string(),
            SignatureToken::U8 => "u8".to_string(),
            SignatureToken::U16 => "u16".to_string(),
            SignatureToken::U32 => "u32".to_string(),
            SignatureToken::U64 => "u64".to_string(),
            SignatureToken::U128 => "u128".to_string(),
            SignatureToken::U256 => "u256".to_string(),
            SignatureToken::Address => "address".to_string(),
            SignatureToken::Signer => "signer".to_string(),
            SignatureToken::Vector(inner) => format!("vector<{}>", self.format_sig_token(inner)),
            SignatureToken::Struct(idx) => self.bytecode.identifier_at(self.bytecode.struct_handle_at(*idx).name).to_string(),
            SignatureToken::TypeParameter(idx) => format!("T{}", idx),
            SignatureToken::StructInstantiation(idx, type_args) => {
                let args: Vec<String> = type_args.iter().map(|token| self.format_sig_token(token)).collect();
                format!("struct_instantiation({:?}, <{}>)", idx, args.join(", "))
            },
            SignatureToken::Reference(sig_tok) => format!(
                "&{}",
                self.format_sig_token(&*sig_tok)
            ),
            _ => "unknown".to_string(),
        }
    }

}
