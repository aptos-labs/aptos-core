// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright (c) The Aptos Labs
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Result};
use clap::Args;
use move_binary_format::{
    binary_views::BinaryIndexedView, file_format::{Bytecode, CodeUnit, CompiledScript, FunctionDefinitionIndex, FunctionHandleIndex, FunctionInstantiationIndex, SignatureIndex, SignatureToken, TableIndex, Visibility}, CompiledModule
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

pub struct QueryGraph<T, E>
where
    T: Eq + Clone + Hash,
    E: Eq + Clone + Hash,
{
    nodes: HashMap<T, NodeIndex>,
    graph: DiGraph<T, E>,
}

impl<T, E> QueryGraph<T, E>
where
    T: Eq + Clone + Hash,
    E: Eq + Clone + Hash,
{
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

    fn add_edge(&mut self, src: T, dst: T, weight: E) {
        if let (Some(src_index), Some(dst_index)) = (self.nodes.get(&src), self.nodes.get(&dst)) {
            self.graph.add_edge(*src_index, *dst_index, weight);
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
        let call_graph: QueryGraph<CGNode, String> = match &self.bytecode {
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
             &|_graph, edge| format!("label = \"{}\"", edge.weight()),
             &|_graph, (_, node)| format!("label=\"{}\"", node.func_sig),
        );
         Ok(format!("{:?}", dot))
     }

     fn dump_script_call_graph(&self, script: &CompiledScript) -> Result<QueryGraph<CGNode, String>> {
        let mut call_graph = QueryGraph::<CGNode, String>::new();
        let mut func_sig_map = HashMap::<FunctionHandleIndex, String>::new();

        // add the entry function node
        let script_entry_node = CGNode{
            func_sig: self.format_function_sig(None, None, Some(script), &mut func_sig_map)?
        };
        call_graph.add_node(script_entry_node.clone());

        // add child function nodes and edges accordingly
        let child_funcs = self.get_child_functions(Some(&script.code));
        for child in child_funcs.into_iter(){

            let mut edge_note = String::from(EMPTY);

            let child_sig = match child{
                ChildFuncType::Function(func_idx) => {
                    self.format_function_sig(Some(&func_idx), None, Some(script), &mut func_sig_map)?
                }
                ChildFuncType::Closure(closure_idx) => {
                    let closure_sig = self.bytecode.signature_at(closure_idx);
                    let type_str = if closure_sig.len() != 1 {
                        "<error: invalid closure signature>".to_string()
                    } else {
                        self.format_sig_token(&closure_sig.0[0])?
                    };
                    format!("CallClosure({})", type_str)
                }
                ChildFuncType::GenericFunction(func_init_idx) => {
                    let func_init = self.bytecode.function_instantiation_at(func_init_idx);
                    let func_handle_idx = func_init.handle;
                    edge_note = self.format_func_params(func_init.type_parameters)?;
                    self.format_function_sig(Some(&func_handle_idx), None, Some(script), &mut func_sig_map)?
                }
            };

            let child_node = CGNode{
                func_sig: child_sig,
            };
            call_graph.add_node(child_node.clone());
            call_graph.add_edge(script_entry_node.clone(), child_node.clone(), edge_note);
        }
        Ok(call_graph)
    }

    fn dump_module_call_graph(&self, module: &CompiledModule) -> Result<QueryGraph<CGNode, String>> {
        let mut call_graph = QueryGraph::<CGNode, String>::new();
        let mut func_sig_map = HashMap::<FunctionHandleIndex, String>::new();

        // collect all the functions with definitions and save their signatures
        // Why: the signature for the same function with and without definition is NOT identicail
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

            //create a node for the current defined function
            let node_with_def = CGNode{
                func_sig: func_sig_map.get(&function_handle_idx).unwrap().clone(),
            };
            call_graph.add_node(node_with_def.clone());

            //process all its child functions
            let children = self.get_child_functions(function_definition.code.as_ref());
            for child in children.into_iter(){
                let mut edge_note = String::from(EMPTY);

                //call a regular function
                let child_sig = match child{
                    ChildFuncType::Function(func_idx) => {
                        if func_sig_map.contains_key(&func_idx){
                            func_sig_map.get(&func_idx).unwrap().clone()
                        }else{
                            self.format_function_sig(Some(&func_idx), None, None, &mut func_sig_map)?
                        }
                    }
                    // call a closure
                    ChildFuncType::Closure(closure_idx) => {
                        let closure_sig = self.bytecode.signature_at(closure_idx);
                        let type_str = if closure_sig.len() != 1 {
                            "<error: invalid closure signature>".to_string()
                        } else {
                            self.format_sig_token(&closure_sig.0[0])?
                        };
                        format!("CallClosure({})", type_str)
                    }

                    //call a generic function
                    ChildFuncType::GenericFunction(func_init_idx) => {
                        let func_init = self.bytecode.function_instantiation_at(func_init_idx);
                        let func_handle_idx = func_init.handle;
                        edge_note = self.format_func_params(func_init.type_parameters)?;
                        if func_sig_map.contains_key(&func_handle_idx){
                            func_sig_map.get(&func_handle_idx).unwrap().clone()
                        }else{
                            self.format_function_sig(Some(&func_handle_idx), None, None, &mut func_sig_map)?
                        }
                    }
                };
                let child_node = CGNode{ func_sig: child_sig};
                call_graph.add_node(child_node.clone());
                call_graph.add_edge(node_with_def.clone(), child_node.clone(), edge_note);
            }
        };
        Ok(call_graph)
    }

    fn get_child_functions(&self, code: Option<&CodeUnit>) -> HashSet<ChildFuncType>{
        match code {
            Some(code) => {
                code.code
                .iter()
                .filter_map(|inst| self.disassemble_instruction(inst)) // returns Option<FunctionHandleIndex>
                .collect::<HashSet<ChildFuncType>>()
            },
            None => HashSet::<ChildFuncType>::new(),
        }
    }

    fn disassemble_instruction(&self,  instruction: &Bytecode) -> Option<ChildFuncType> {
        match instruction {
            Bytecode::Call(func_idx) => Some(ChildFuncType::Function(*func_idx)),
            Bytecode::CallClosure(sig_idx) => Some(ChildFuncType::Closure(*sig_idx)),
            Bytecode::CallGeneric(func_init_idx) => Some(ChildFuncType::GenericFunction(*func_init_idx)),
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
            let module_name: &str = module_id.name.as_str();
            let function_name: &str = self.bytecode.identifier_at(function_handle.name).as_str();
            let ty_params: &str = EMPTY; //function_handle.type_parameters;
            let params: String = self.format_func_params(function_handle.parameters)?;
            let ret_type: String = self.format_func_rets(function_handle.return_)?;

            let res = format!("{}{}{}{}::{}{}{}{}", entry_modifier, native_modifier, visibility_modifier, module_name, function_name, ty_params, params, ret_type);
            func_sig_map.insert(*function_handle_idx, res.clone());
            return Ok(res);
        }

        // If we get to this point, the function must be the script entry function
        // We never need to worry about caching the function sig
        if let Some(script) = s {
            let entry_modifier = ENTRY_MODIFIER;
            let module_name = SCRIPT_NAME;
            let function_name = MAIN_FUNCTION;
            let ty_params: &str = EMPTY; // script.type_parameters;
            let params: String = self.format_func_params(script.parameters)?;
            return Ok(format!("{}{}::{}{}", entry_modifier, module_name, function_name, params));
        }
        Ok(String::from(""))
    }


    fn format_func_params(&self, para_sig_idx: SignatureIndex) -> Result<String>{
        let params: Vec<String> = self.bytecode.signature_at(para_sig_idx).0.iter().map(|token|self.format_sig_token(token)).collect::<Result<Vec<String>>>()?;
        Ok(format!("({})", params.join(", ")))
    }

    fn format_func_rets(&self, ret_sig_idx: SignatureIndex) -> Result<String>{
        let rets: Vec<String> = self.bytecode.signature_at(ret_sig_idx).0.iter().map(|token|self.format_sig_token(token)).collect::<Result<Vec<String>>>()?;
        if rets.is_empty(){
            Ok(String::from(EMPTY))
        }else{
            Ok(format!(" -> {}", rets.join(", ")))
        }
    }

    // code mostly copied from the move disassembler
    fn format_sig_token(&self, token: &SignatureToken) -> Result<String> {
        let list_sig_vec = |toks: &Vec<SignatureToken>| {
            toks.into_iter()
                .map(|tok| self.format_sig_token(tok))
                .collect::<Result<Vec<String>>>()
        };
        Ok(match token {
            SignatureToken::Bool => "bool".to_string(),
            SignatureToken::U8 => "u8".to_string(),
            SignatureToken::U16 => "u16".to_string(),
            SignatureToken::U32 => "u32".to_string(),
            SignatureToken::U64 => "u64".to_string(),
            SignatureToken::U128 => "u128".to_string(),
            SignatureToken::U256 => "u256".to_string(),
            SignatureToken::Address => "address".to_string(),
            SignatureToken::Signer => "signer".to_string(),
            SignatureToken::Vector(inner) => format!("vector<{}>", self.format_sig_token(inner)?),
            SignatureToken::Struct(idx) => self.bytecode.identifier_at(self.bytecode.struct_handle_at(*idx).name).to_string(),
            SignatureToken::TypeParameter(idx) => format!("T{}", idx),
            SignatureToken::Reference(sig_tok) => format!("&{}",self.format_sig_token(sig_tok)?),
            SignatureToken::MutableReference(sig_tok) => format!("&mut {}",self.format_sig_token(sig_tok)?),
            SignatureToken::StructInstantiation(idx, type_args) => {
                let name = self.bytecode.identifier_at(
                        self.bytecode
                            .struct_handle_at(*idx)
                            .name,
                    ).to_string();
                let instantiation = list_sig_vec(type_args)?;
                if instantiation.is_empty() {
                    format!("{}", name)
                } else {
                    format!("{}<{}>", name,instantiation.join(", "))
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
    Closure(SignatureIndex)
}
