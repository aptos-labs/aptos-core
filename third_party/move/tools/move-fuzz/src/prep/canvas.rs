// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::prep::{
    graph::{FlowGraph, FlowGraphEdge, FlowGraphNode},
    ident::{DatatypeIdent, FunctionIdent},
    model::Model,
    typing::{SimpleType, TypeBase, TypeItem, TypeMode},
};
use itertools::Itertools;
use move_core_types::ability::AbilitySet;
use petgraph::{algo::toposort, visit::EdgeRef, Direction};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt::Display, fs, path::Path};

/// Types that can be argument of driver function
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BasicInput {
    Bool,
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    U64,
    I64,
    U128,
    I128,
    U256,
    I256,
    String,
    Address,
    Signer,
    ObjectKnown {
        ident: DatatypeIdent,
        type_args: Vec<TypeBase>,
    },
    ObjectParam {
        index: usize,
    },
    Vector(Box<Self>),
}

impl BasicInput {
    pub fn with_depth(self, depth: usize) -> Self {
        assert!(!matches!(self, Self::Vector(_)));
        if depth == 0 {
            return self;
        }
        Self::Vector(self.with_depth(depth - 1).into())
    }
}

impl Display for BasicInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BasicInput::Bool => write!(f, "bool"),
            BasicInput::U8 => write!(f, "u8"),
            BasicInput::I8 => write!(f, "i8"),
            BasicInput::U16 => write!(f, "u16"),
            BasicInput::I16 => write!(f, "i16"),
            BasicInput::U32 => write!(f, "u32"),
            BasicInput::I32 => write!(f, "i32"),
            BasicInput::U64 => write!(f, "u64"),
            BasicInput::I64 => write!(f, "i64"),
            BasicInput::U128 => write!(f, "u128"),
            BasicInput::I128 => write!(f, "i128"),
            BasicInput::U256 => write!(f, "u256"),
            BasicInput::I256 => write!(f, "i256"),
            BasicInput::String => write!(f, "std::string::String"),
            BasicInput::Address => write!(f, "address"),
            BasicInput::Signer => write!(f, "signer"),
            BasicInput::ObjectKnown { ident, type_args } => {
                let type_args_str = display_type_bases_as_generics(type_args);
                write!(f, "aptos_framework::object::Object<{ident}{type_args_str}>")
            },
            BasicInput::ObjectParam { index } => {
                write!(f, "aptos_framework::object::Object<T{index}>")
            },
            BasicInput::Vector(element) => write!(f, "vector<{}>", element),
        }
    }
}

/// A pre-selected lambda binding for a Function-typed parameter
#[derive(Debug, Clone)]
pub struct LambdaBinding {
    pub fn_params: Vec<TypeItem>,
    pub fn_ident: FunctionIdent,
    pub fn_type_args: Vec<TypeBase>,
}

#[derive(Debug, Copy, Clone)]
pub enum DriverVariable {
    Param(usize),
    Local(usize),
}

impl Display for DriverVariable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DriverVariable::Param(index) => write!(f, "p{}", index),
            DriverVariable::Local(index) => write!(f, "v{}", index),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DriverStatement {
    Call {
        ident: FunctionIdent,
        type_args: Vec<TypeBase>,
        arguments: Vec<DriverVariable>,
        ret_binds: Vec<Option<DriverVariable>>,
    },
    Copy {
        src: DriverVariable,
        dst: DriverVariable,
    },
    Deref {
        src: DriverVariable,
        dst: DriverVariable,
    },
    Freeze {
        src: DriverVariable,
        dst: DriverVariable,
    },
    ImmBorrow {
        src: DriverVariable,
        dst: DriverVariable,
    },
    MutBorrow {
        src: DriverVariable,
        dst: DriverVariable,
    },
    VectorToElement {
        src: DriverVariable,
        dst: DriverVariable,
    },
    ElementToVector {
        src: DriverVariable,
        dst: DriverVariable,
    },

    // lambda support
    Lambda {
        fn_ident: FunctionIdent,
        fn_type_args: Vec<TypeBase>,
        fn_params: Vec<TypeItem>,
        dst: DriverVariable,
    },

    // the following is only expected to be used as argument bridges
    Arg2Bitvec {
        src: DriverVariable,
        depth: usize,
        dst: DriverVariable,
    },
}

impl Display for DriverStatement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DriverStatement::Call {
                ident,
                type_args,
                arguments,
                ret_binds,
            } => {
                let type_args_str = display_type_bases_as_generics(type_args);
                let args_str = arguments.iter().map(|arg| arg.to_string()).join(", ");
                let rets_str = if ret_binds.is_empty() {
                    "".to_string()
                } else if ret_binds.len() == 1 {
                    match &ret_binds[0] {
                        Some(var) => format!("let {var} = "),
                        None => "let _ = ".to_string(),
                    }
                } else {
                    format!(
                        "let ({}) = ",
                        ret_binds
                            .iter()
                            .map(|opt| match opt {
                                Some(var) => format!("{var}"),
                                None => "_".to_string(),
                            })
                            .join(", ")
                    )
                };
                write!(f, "{rets_str}{ident}{type_args_str}({args_str});")
            },
            DriverStatement::Copy { src, dst } => write!(f, "let {dst} = {src};"),
            DriverStatement::Deref { src, dst } => write!(f, "let {dst} = *{src};"),
            DriverStatement::Freeze { src, dst } => write!(f, "let {dst} = {src};"),
            DriverStatement::ImmBorrow { src, dst } => write!(f, "let {dst} = &{src};"),
            DriverStatement::MutBorrow { src, dst } => write!(f, "let {dst} = &mut {src};"),
            DriverStatement::VectorToElement { src, dst } => {
                write!(f, "let {dst} = std::vector::pop_back(&mut {src});")
            },
            DriverStatement::ElementToVector { src, dst } => {
                write!(f, "let {dst} = std::vector::singleton({src});")
            },
            DriverStatement::Lambda {
                fn_ident,
                fn_type_args,
                fn_params,
                dst,
            } => {
                let fn_inst = display_type_bases_as_generics(fn_type_args);
                let lambda_params = (0..fn_params.len()).map(|i| format!("a{i}")).join(", ");
                let call_args = (0..fn_params.len()).map(|i| format!("a{i}")).join(", ");
                write!(
                    f,
                    "let {dst} = |{lambda_params}| {fn_ident}{fn_inst}({call_args});"
                )
            },
            DriverStatement::Arg2Bitvec { src, depth, dst } => {
                write!(
                    f,
                    "{}",
                    format_bitvec_bridge(&src.to_string(), &dst.to_string(), *depth, "        ",)
                )
            },
        }
    }
}

/// A data structure that holds the details on how to generate a driver
#[derive(Debug, Clone)]
pub struct DriverCanvas {
    generics: Vec<AbilitySet>,
    parameters: Vec<BasicInput>,
    statements: Vec<DriverStatement>,
    local_var_count: usize,
}

impl DriverCanvas {
    /// Try to build a canvas based on a flow graph with pre-selected lambda bindings.
    ///
    /// Returns `None` when the flow graph is incomplete and cannot supply all variables
    /// needed to materialize the driver.
    pub fn try_build(
        model: &Model,
        flow: &FlowGraph,
        lambda_bindings: &BTreeMap<usize, LambdaBinding>,
    ) -> Option<Self> {
        let FlowGraph { graph, generics } = flow;

        // create a new canvas
        let mut canvas = Self::new();

        // register generics
        for abilities in generics.values() {
            canvas.new_param(*abilities);
        }

        // populate statements, register parameters as needed
        let mut type_node_vars: BTreeMap<_, DriverVariable> = BTreeMap::new();

        // track the single signer parameter so all &signer / &mut signer / signer usages share it
        let mut signer_var = None;
        for node in toposort(&graph, None).ok()? {
            match graph.node_weight(node).unwrap() {
                FlowGraphNode::Datatype(_) => {
                    let src = type_node_vars.get(&node).copied()?;

                    // analyze all outgoing edges to see how this datatype is used
                    let outgoing_edges: Vec<_> = graph
                        .edges_directed(node, Direction::Outgoing)
                        .map(|e| (e.weight().clone(), e.target()))
                        .collect();

                    for (weight, target) in outgoing_edges {
                        let dst = match &weight {
                            FlowGraphEdge::Def(_) => {
                                unreachable!(
                                    "unexpected outgoing edge of def kind for a datatype node"
                                )
                            },
                            FlowGraphEdge::Use(_) => {
                                // will be handled at function node
                                continue;
                            },
                            FlowGraphEdge::Copy => canvas.new_stmt_copy(src),
                            FlowGraphEdge::Deref => canvas.new_stmt_deref(src),
                            FlowGraphEdge::Freeze => canvas.new_stmt_freeze(src),
                            FlowGraphEdge::ImmBorrow => canvas.new_stmt_imm_borrow(src),
                            FlowGraphEdge::MutBorrow => canvas.new_stmt_mut_borrow(src),
                            FlowGraphEdge::VectorToElement => {
                                canvas.new_stmt_vector_to_element(src)
                            },
                            FlowGraphEdge::ElementToVector => {
                                canvas.new_stmt_element_to_vector(src)
                            },
                        };

                        // add the receiver node to mapping
                        let exists = type_node_vars.insert(target, dst);
                        assert!(exists.is_none());
                    }
                },
                FlowGraphNode::Function(func) => {
                    // collect incoming edges
                    let mut dt_vars = BTreeMap::new();
                    for edge in graph.edges_directed(node, Direction::Incoming) {
                        match edge.weight() {
                            FlowGraphEdge::Use(param) => {
                                let dt_node = edge.source();
                                let var = type_node_vars.get(&dt_node).copied()?;
                                let exists = dt_vars.insert(*param, var);
                                assert!(exists.is_none());
                            },
                            _ => unreachable!(
                                "unexpected incoming edge of non-use kind for a function node"
                            ),
                        }
                    }

                    // build the call
                    let decl = model.function_registry.lookup_decl(&func.ident);

                    let mut args = vec![];
                    for (index, item) in decl.parameters.iter().enumerate() {
                        let arg_ty = model
                            .datatype_registry
                            .instantiate_type_ref(item, &func.type_args);

                        // helper: get or create the single signer parameter
                        let signer_singleton = |canvas: &mut DriverCanvas,
                                                signer_var: &mut Option<DriverVariable>|
                         -> DriverVariable {
                            match *signer_var {
                                Some(var) => var,
                                None => {
                                    let var = canvas.new_input(BasicInput::Signer, 0);
                                    *signer_var = Some(var);
                                    var
                                },
                            }
                        };

                        let arg_var = match arg_ty {
                            TypeItem::Base(ty_base) => match TypeMode::convert(&ty_base) {
                                TypeMode::Simple(SimpleType::Signer) => {
                                    signer_singleton(&mut canvas, &mut signer_var)
                                },
                                TypeMode::Simple(SimpleType::Function { .. }) => {
                                    let binding = lambda_bindings
                                        .get(&index)
                                        .expect("lambda binding for Function param");
                                    canvas.add_lambda(binding)
                                },
                                TypeMode::Simple(simple_ty) => {
                                    canvas.add_input_simple_recursive(&simple_ty, 0)
                                },
                                TypeMode::Complex(_) => dt_vars.get(&index).copied()?,
                            },
                            TypeItem::ImmRef(ty_base) => match TypeMode::convert(&ty_base) {
                                TypeMode::Simple(SimpleType::Signer) => {
                                    let base_var = signer_singleton(&mut canvas, &mut signer_var);
                                    canvas.new_stmt_imm_borrow(base_var)
                                },
                                TypeMode::Simple(SimpleType::Function { .. }) => {
                                    let binding = lambda_bindings
                                        .get(&index)
                                        .expect("lambda binding for Function param");
                                    let base_var = canvas.add_lambda(binding);
                                    canvas.new_stmt_imm_borrow(base_var)
                                },
                                TypeMode::Simple(simple_ty) => {
                                    let base_var = canvas.add_input_simple_recursive(&simple_ty, 0);
                                    canvas.new_stmt_imm_borrow(base_var)
                                },
                                TypeMode::Complex(_) => dt_vars.get(&index).copied()?,
                            },
                            TypeItem::MutRef(ty_base) => match TypeMode::convert(&ty_base) {
                                TypeMode::Simple(SimpleType::Signer) => {
                                    let base_var = signer_singleton(&mut canvas, &mut signer_var);
                                    canvas.new_stmt_mut_borrow(base_var)
                                },
                                TypeMode::Simple(SimpleType::Function { .. }) => {
                                    let binding = lambda_bindings
                                        .get(&index)
                                        .expect("lambda binding for Function param");
                                    let base_var = canvas.add_lambda(binding);
                                    canvas.new_stmt_mut_borrow(base_var)
                                },
                                TypeMode::Simple(simple_ty) => {
                                    let base_var = canvas.add_input_simple_recursive(&simple_ty, 0);
                                    canvas.new_stmt_mut_borrow(base_var)
                                },
                                TypeMode::Complex(_) => dt_vars.get(&index).copied()?,
                            },
                        };
                        args.push(arg_var);
                    }

                    // analyze the return values
                    let mut rt_vars = BTreeMap::new();
                    for edge in graph.edges_directed(node, Direction::Outgoing) {
                        let param = match edge.weight() {
                            FlowGraphEdge::Def(param) => *param,
                            _ => unreachable!(
                                "unexpected outgoing edge of non-def kind for a function node"
                            ),
                        };

                        // mark the return variable in the mapping
                        let var = canvas.new_local();
                        let exists = rt_vars.insert(param, var);
                        assert!(exists.is_none());

                        // add the receiver node to mapping
                        let receiver = edge.target();
                        let exists = type_node_vars.insert(receiver, var);
                        assert!(exists.is_none());
                    }

                    let mut rets = vec![];
                    for (index, _) in decl.return_sig.iter().enumerate() {
                        rets.push(rt_vars.get(&index).cloned());
                    }

                    // done with the statement
                    let stmt = DriverStatement::Call {
                        ident: func.ident.clone(),
                        type_args: func.type_args.clone(),
                        arguments: args,
                        ret_binds: rets,
                    };
                    canvas.statements.push(stmt);
                },
            }
        }

        // done
        Some(canvas)
    }

    /// Create an empty canvas
    fn new() -> Self {
        Self {
            generics: vec![],
            parameters: vec![],
            statements: vec![],
            local_var_count: 0,
        }
    }

    /// Add a new generic type parameter
    fn new_param(&mut self, constraints: AbilitySet) -> usize {
        let index = self.generics.len();
        self.generics.push(constraints);
        index
    }

    /// Add a new input
    fn new_input(&mut self, base: BasicInput, depth: usize) -> DriverVariable {
        let index = self.parameters.len();
        self.parameters.push(base.with_depth(depth));
        DriverVariable::Param(index)
    }

    /// Create a new index for a local variable
    fn new_local(&mut self) -> DriverVariable {
        let index = self.local_var_count;
        self.local_var_count += 1;
        DriverVariable::Local(index)
    }

    /// Register parameters for an input type
    fn add_input_simple_recursive(&mut self, t: &SimpleType, depth: usize) -> DriverVariable {
        match t {
            SimpleType::Bool => self.new_input(BasicInput::Bool, depth),
            SimpleType::U8 => self.new_input(BasicInput::U8, depth),
            SimpleType::I8 => self.new_input(BasicInput::I8, depth),
            SimpleType::U16 => self.new_input(BasicInput::U16, depth),
            SimpleType::I16 => self.new_input(BasicInput::I16, depth),
            SimpleType::U32 => self.new_input(BasicInput::U32, depth),
            SimpleType::I32 => self.new_input(BasicInput::I32, depth),
            SimpleType::U64 => self.new_input(BasicInput::U64, depth),
            SimpleType::I64 => self.new_input(BasicInput::I64, depth),
            SimpleType::U128 => self.new_input(BasicInput::U128, depth),
            SimpleType::I128 => self.new_input(BasicInput::I128, depth),
            SimpleType::U256 => self.new_input(BasicInput::U256, depth),
            SimpleType::I256 => self.new_input(BasicInput::I256, depth),
            SimpleType::Bitvec => {
                let src = self.new_input(BasicInput::Bool, depth + 1);
                let dst = self.new_local();
                self.statements
                    .push(DriverStatement::Arg2Bitvec { src, depth, dst });
                dst
            },
            SimpleType::String => self.new_input(BasicInput::String, depth),
            SimpleType::Address => self.new_input(BasicInput::Address, depth),
            SimpleType::Signer => self.new_input(BasicInput::Signer, depth),
            SimpleType::Vector { element } => self.add_input_simple_recursive(element, depth + 1),
            SimpleType::ObjectKnown {
                ident,
                type_args,
                abilities: _,
            } => self.new_input(
                BasicInput::ObjectKnown {
                    ident: ident.clone(),
                    type_args: type_args.clone(),
                },
                depth,
            ),
            SimpleType::ObjectParam {
                index,
                abilities: _,
            } => self.new_input(BasicInput::ObjectParam { index: *index }, depth),
            SimpleType::Function { .. } => {
                unreachable!("Function types should be handled via lambda bindings")
            },
        }
    }

    /// Create a copy statement
    fn new_stmt_copy(&mut self, src: DriverVariable) -> DriverVariable {
        let dst = self.new_local();
        self.statements.push(DriverStatement::Copy { src, dst });
        dst
    }

    /// Create a dereference statement
    fn new_stmt_deref(&mut self, src: DriverVariable) -> DriverVariable {
        let dst = self.new_local();
        self.statements.push(DriverStatement::Deref { src, dst });
        dst
    }

    /// Create a freeze statement
    fn new_stmt_freeze(&mut self, src: DriverVariable) -> DriverVariable {
        let dst = self.new_local();
        self.statements.push(DriverStatement::Freeze { src, dst });
        dst
    }

    /// Create an immutable borrow statement
    fn new_stmt_imm_borrow(&mut self, src: DriverVariable) -> DriverVariable {
        let dst = self.new_local();
        self.statements
            .push(DriverStatement::ImmBorrow { src, dst });
        dst
    }

    /// Create a mutable borrow statement
    fn new_stmt_mut_borrow(&mut self, src: DriverVariable) -> DriverVariable {
        let dst = self.new_local();
        self.statements
            .push(DriverStatement::MutBorrow { src, dst });
        dst
    }

    /// Create a vector to element statement
    fn new_stmt_vector_to_element(&mut self, src: DriverVariable) -> DriverVariable {
        let dst = self.new_local();
        self.statements
            .push(DriverStatement::VectorToElement { src, dst });
        dst
    }

    /// Create an element to vector statement
    fn new_stmt_element_to_vector(&mut self, src: DriverVariable) -> DriverVariable {
        let dst = self.new_local();
        self.statements
            .push(DriverStatement::ElementToVector { src, dst });
        dst
    }

    /// Create a lambda statement from a pre-selected binding
    fn add_lambda(&mut self, binding: &LambdaBinding) -> DriverVariable {
        let dst = self.new_local();
        self.statements.push(DriverStatement::Lambda {
            fn_ident: binding.fn_ident.clone(),
            fn_type_args: binding.fn_type_args.clone(),
            fn_params: binding.fn_params.clone(),
            dst,
        });
        dst
    }

    /// Dump the canvas to a script
    pub fn generate_script(
        &self,
        sequence: usize,
        ident: &FunctionIdent,
        output_dir: &Path,
    ) -> ScriptSignature {
        // script name
        let script_name = format!("fuzz_script_{sequence}");

        // prepare type generics
        let mut generics_decls = vec![];
        for (index, abilities) in self.generics.iter().enumerate() {
            let decl = format!("T{index}: {abilities}");
            generics_decls.push(decl);
        }
        let generics_str = if generics_decls.is_empty() {
            "".to_string()
        } else {
            format!("<{}>", generics_decls.join(", "))
        };

        // prepare for parameter declarations
        let mut param_signer = None;
        let mut param_others = vec![];
        let mut script_params = vec![];
        for (index, param) in self.parameters.iter().enumerate() {
            match param {
                BasicInput::Signer => {
                    assert!(param_signer.is_none());
                    param_signer = Some(format!("p{index}: signer"));
                    script_params.insert(0, param.clone());
                },
                other => {
                    param_others.push(format!("p{index}: {other}"));
                    script_params.push(other.clone());
                },
            }
        }
        let param_decls_str = param_signer.into_iter().chain(param_others).join(", ");

        // prepare for statements
        let mut statements = vec![];
        for stmt in &self.statements {
            statements.push(format!("        {stmt}"));
        }
        let statements_str = statements.join("\n");

        // assemble the script
        let script = format!(
            r#"
script {{
    fun {script_name}{generics_str}({param_decls_str}) {{
{statements_str}
    }}
}}
"#
        );

        // write to file
        let script_path = output_dir.join(format!("{script_name}.move"));
        fs::write(&script_path, &script).expect("failed to write script file");

        // return the details of the generated script
        ScriptSignature {
            name: script_name,
            ident: ident.clone(),
            generics: self.generics.clone(),
            parameters: script_params,
        }
    }

    /// Stable signature key used to deduplicate generated wrappers.
    pub fn dedup_key(&self, ident: &FunctionIdent) -> String {
        let generics_key = self.generics.iter().map(|g| format!("{g:?}")).join("|");
        let params_key = self.parameters.iter().map(|p| p.to_string()).join("|");
        let statements_key = self.statements.iter().map(|s| s.to_string()).join("|");
        format!("{ident}||{generics_key}||{params_key}||{statements_key}")
    }
}

/// Generate Move code to bridge a `vector<...vector<bool>...>` to a (nested vector of) `BitVector`.
///
/// At `depth == 0`: converts `vector<bool>` → `BitVector`
/// At `depth > 0`: maps over `depth` levels of vectors, converting the innermost `vector<bool>`
fn format_bitvec_bridge(src: &str, dst: &str, depth: usize, indent: &str) -> String {
    if depth == 0 {
        // Core conversion: vector<bool> -> BitVector
        format!(
            "let {dst} = std::bit_vector::new(std::vector::length(&{src}));\n\
             {indent}{{ let __i = 0; while (__i < std::vector::length(&{src})) \
             {{ if (*std::vector::borrow(&{src}, __i)) \
             {{ std::bit_vector::set(&mut {dst}, __i); }}; \
             __i = __i + 1; }}; }};"
        )
    } else {
        // Map over vector, recursively converting each element
        let elem = format!("__bv_e{depth}");
        let result = format!("__bv_r{depth}");
        let body_indent = format!("{indent}    ");
        let inner = format_bitvec_bridge(&elem, &result, depth - 1, &body_indent);
        format!(
            "std::vector::reverse(&mut {src});\n\
             {indent}let {dst} = std::vector::empty();\n\
             {indent}while (!std::vector::is_empty(&{src})) {{\n\
             {body_indent}let {elem} = std::vector::pop_back(&mut {src});\n\
             {body_indent}{inner}\n\
             {body_indent}std::vector::push_back(&mut {dst}, {result});\n\
             {indent}}};"
        )
    }
}

/// Utility: print a TypeBase for scripting
fn display_type_base(t: &TypeBase) -> String {
    match t {
        TypeBase::Bool => "bool".to_string(),
        TypeBase::U8 => "u8".to_string(),
        TypeBase::I8 => "i8".to_string(),
        TypeBase::U16 => "u16".to_string(),
        TypeBase::I16 => "i16".to_string(),
        TypeBase::U32 => "u32".to_string(),
        TypeBase::I32 => "i32".to_string(),
        TypeBase::U64 => "u64".to_string(),
        TypeBase::I64 => "i64".to_string(),
        TypeBase::U128 => "u128".to_string(),
        TypeBase::I128 => "i128".to_string(),
        TypeBase::U256 => "u256".to_string(),
        TypeBase::I256 => "i256".to_string(),
        TypeBase::Bitvec => "std::bit_vector::BitVector".to_string(),
        TypeBase::String => "std::string::String".to_string(),
        TypeBase::Address => "address".to_string(),
        TypeBase::Signer => "signer".to_string(),
        TypeBase::Vector { element } => {
            format!("vector<{}>", display_type_base(element))
        },
        TypeBase::Datatype {
            ident,
            type_args,
            abilities: _,
        } => {
            let args_str = if type_args.is_empty() {
                "".to_string()
            } else {
                format!("<{}>", type_args.iter().map(display_type_base).join(", "))
            };
            format!("{}{}", ident, args_str)
        },
        TypeBase::Param {
            index,
            abilities: _,
        } => format!("T{index}"),
        TypeBase::ObjectKnown {
            ident,
            type_args,
            abilities: _,
        } => {
            if type_args.is_empty() {
                format!("aptos_framework::object::Object<{}>", ident)
            } else {
                let args_str = type_args.iter().map(display_type_base).join(", ");
                format!("aptos_framework::object::Object<{}<{}>>", ident, args_str)
            }
        },
        TypeBase::ObjectParam {
            index: param,
            abilities: _,
        } => format!("aptos_framework::object::Object<T{param}>"),
        TypeBase::Function {
            params,
            returns,
            abilities: _,
        } => {
            let params_str = params.iter().map(display_type_item).join(", ");
            let returns_str = returns.iter().map(display_type_item).join(", ");
            format!("|{params_str}| ({returns_str})")
        },
    }
}

/// Utility: print a TypeItem for scripting
fn display_type_item(t: &TypeItem) -> String {
    match t {
        TypeItem::Base(base) => display_type_base(base),
        TypeItem::ImmRef(base) => format!("&{}", display_type_base(base)),
        TypeItem::MutRef(base) => format!("&mut {}", display_type_base(base)),
    }
}

fn display_type_bases_as_generics(ts: &[TypeBase]) -> String {
    if ts.is_empty() {
        "".to_string()
    } else {
        format!("<{}>", ts.iter().map(display_type_base).join(", "))
    }
}

/// Details of generated script
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptSignature {
    pub name: String,
    pub ident: FunctionIdent,
    pub generics: Vec<AbilitySet>,
    pub parameters: Vec<BasicInput>,
}

#[cfg(test)]
mod tests {
    use super::{
        display_type_bases_as_generics, format_bitvec_bridge, BasicInput, DriverCanvas,
        DriverStatement, DriverVariable,
    };
    use crate::{
        deps::PkgKind,
        prep::{
            datatype::DatatypeRegistry,
            function::{FunctionDecl, FunctionRegistry},
            graph::{FlowGraph, FlowGraphNode, FunctionInst},
            ident::{DatatypeIdent, FunctionIdent},
            model::Model,
            typing::{TypeBase, TypeItem, TypeRef, TypeTag},
        },
    };
    use move_core_types::{
        ability::AbilitySet, account_address::AccountAddress, identifier::Identifier,
    };
    use petgraph::stable_graph::StableGraph;
    use std::collections::BTreeMap;

    fn datatype(name: &str) -> DatatypeIdent {
        DatatypeIdent::from_struct_tuple(
            AccountAddress::ONE,
            Identifier::new("m").unwrap(),
            Identifier::new(name).unwrap(),
        )
    }

    fn function(name: &str) -> FunctionIdent {
        FunctionIdent::from_function_tuple(
            AccountAddress::ONE,
            Identifier::new("m").unwrap(),
            Identifier::new(name).unwrap(),
        )
    }

    fn model_with_decl(decl: FunctionDecl) -> Model {
        let mut function_registry = FunctionRegistry::new();
        function_registry.insert_for_test(decl);
        Model {
            datatype_registry: DatatypeRegistry::new(),
            function_registry,
        }
    }

    #[test]
    fn test_basic_input_with_depth_wraps_recursively() {
        let input = BasicInput::Address.with_depth(2);
        assert_eq!(input.to_string(), "vector<vector<address>>");
    }

    #[test]
    fn test_display_type_bases_as_generics_formats_nested_types() {
        let rendered = display_type_bases_as_generics(&[TypeBase::U64, TypeBase::Vector {
            element: Box::new(TypeBase::ObjectKnown {
                ident: datatype("Vault"),
                type_args: vec![TypeBase::Bool],
                abilities: AbilitySet::PRIMITIVES,
            }),
        }]);

        assert_eq!(
            rendered,
            "<u64, vector<aptos_framework::object::Object<0x1::m::Vault<bool>>>>"
        );
    }

    #[test]
    fn test_driver_statement_arg2bitvec_expands_bridge_code() {
        let rendered = DriverStatement::Arg2Bitvec {
            src: DriverVariable::Param(0),
            depth: 1,
            dst: DriverVariable::Local(0),
        }
        .to_string();

        assert!(rendered.contains("std::vector::reverse(&mut p0);"));
        assert!(rendered.contains("let v0 = std::vector::empty();"));
        assert!(rendered.contains("std::bit_vector::new"));
    }

    #[test]
    fn test_format_bitvec_bridge_recursive_shape() {
        let rendered = format_bitvec_bridge("src", "dst", 2, "    ");
        assert!(rendered.contains("std::vector::reverse(&mut src);"));
        assert!(rendered.contains("let __bv_e2 = std::vector::pop_back(&mut src);"));
        assert!(rendered.contains("std::vector::push_back(&mut dst, __bv_r2);"));
    }

    #[test]
    fn test_driver_statement_copy_and_borrow_rendering() {
        let copy = DriverStatement::Copy {
            src: DriverVariable::Param(1),
            dst: DriverVariable::Local(0),
        };
        let borrow = DriverStatement::ImmBorrow {
            src: DriverVariable::Local(0),
            dst: DriverVariable::Local(1),
        };

        assert_eq!(copy.to_string(), "let v0 = p1;");
        assert_eq!(borrow.to_string(), "let v1 = &v0;");
        assert_eq!(
            TypeItem::MutRef(TypeBase::Address).to_string(),
            "&mut address"
        );
    }

    #[test]
    fn test_driver_canvas_try_build_rejects_missing_complex_provider() {
        let ident = function("target");
        let model = model_with_decl(FunctionDecl {
            ident: ident.clone(),
            generics: vec![AbilitySet::PRIMITIVES],
            parameters: vec![TypeRef::Base(TypeTag::Param(0))],
            return_sig: vec![],
            kind: PkgKind::Primary,
            is_entry: true,
        });
        let mut graph = FlowGraph {
            graph: StableGraph::new(),
            generics: BTreeMap::from([(0, AbilitySet::PRIMITIVES)]),
        };
        graph.graph.add_node(FlowGraphNode::Function(FunctionInst {
            ident,
            type_args: vec![TypeBase::Param {
                index: 0,
                abilities: AbilitySet::PRIMITIVES,
            }],
        }));

        assert!(DriverCanvas::try_build(&model, &graph, &BTreeMap::new()).is_none());
    }

    #[test]
    fn test_driver_canvas_try_build_accepts_simple_only_root_call() {
        let ident = function("simple");
        let model = model_with_decl(FunctionDecl {
            ident: ident.clone(),
            generics: vec![],
            parameters: vec![TypeRef::Base(TypeTag::U64)],
            return_sig: vec![],
            kind: PkgKind::Primary,
            is_entry: true,
        });
        let mut graph = FlowGraph {
            graph: StableGraph::new(),
            generics: BTreeMap::new(),
        };
        graph.graph.add_node(FlowGraphNode::Function(FunctionInst {
            ident,
            type_args: vec![],
        }));

        let canvas = DriverCanvas::try_build(&model, &graph, &BTreeMap::new())
            .expect("simple-only root call should build");
        assert_eq!(canvas.parameters, vec![BasicInput::U64]);
        assert_eq!(canvas.local_var_count, 0);
        assert_eq!(canvas.statements.len(), 1);
        match &canvas.statements[0] {
            DriverStatement::Call {
                ident,
                type_args,
                arguments,
                ret_binds,
            } => {
                assert_eq!(ident, &function("simple"));
                assert!(type_args.is_empty());
                assert_eq!(arguments.len(), 1);
                assert!(matches!(arguments[0], DriverVariable::Param(0)));
                assert!(ret_binds.is_empty());
            },
            stmt => panic!("expected call statement, got {stmt:?}"),
        }
    }
}
