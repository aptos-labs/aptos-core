// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
//!
//! Signed integer rewriter
//! - Since language version 2.3, primitive types for signed integers, `i64` and `i128`, are allowed with built-in operations `+ - (as both sub and neg) * / % < > <= >= == != as`. Yet, the types and operations remain syntactic sugar without MoveVM support.
//! - This rewriter rewrites the AST to replace all uses of `i64` and `i128` with their struct-based counterparts `Struct I64` and `Struct I128` from `move-stdlib::i64` and `move-stdlib::i128`. Likewise, operations on `i64` and `i128` will be replaced with those on `Struct I64` and `Struct I128`.
//! - For simplicity of presentation, code below will use `struct modules` to refer to `Struct I64` and `Struct I128`.
//!
//! Example:
//!    fun example(a: i64, b: i64): i64 {
//!        a + b
//!    }
//!    =======>
//!    fun example(a: I64, b: I64): I64 {
//!        std::i64::add(a, b)
//!    }
//!
//! To ensure correctness of the rewriting, the following must be ensured:
//! 1. All occurances of `i64` and `i128` must be replaced with `Struct I64` and `Struct I128`, which include
//!   - field types in struct definition
//!   - argument types and return type of both Move and spec function declaration
//!   - node types of AST expressions (including those in spec blocks)
//!   - node instantiation types of AST expressions
//!   - Important: the type rewriting needs to be done recursively (e.g., `i64` and `i128` nested inside `vector` also need to be replaced)
//! 2. All operations on `i64` and `i128` must be replaced with those on `Struct I64` and `Struct I128`, listed as below (semantics should refer to `move-stdlib`):
//!   - Constant numbers --> `std::i64/i128::pack()` (e.g., `10i64` --> `std::i64::pack(10u64)`)
//!   - `+` -> `std::i64/i128::add()`
//!   - `-` (as sub) -> `std::i64/i128::sub()`
//!   - `-` (as neg) -> `std::i64/i128::neg()`
//!   - `*` -> `std::i64/i128::mul()`
//!   - `/` -> `std::i64/i128::div()`
//!   - `%` -> `std::i64/i128::mod()`
//!   - `<` -> `std::i64/i128::lt()`
//!   - `>` -> `std::i64/i128::gt()`
//!   - `<=` -> `std::i64/i128::lte()`
//!   - `>=` -> `std::i64/i128::gte()`
//!   - `==` -> `std::i64/i128::eq()`
//!   - `!=` -> `std::i64/i128::ne()`
//!   - `i64/i128 as u64/u128` -> `std::i64/i128::unpack()`
//!   - `u64/u128 as i64/i128` -> `std::i64/i128::pack()`
//!
//!
//! Key impls:
//! - `rewrite_struct_def` to rewrite field types in struct definition
//! - `rewrite_func_decl` to rewrite argument and return types in declarations of both Move and spec functions
//! - `rewrite_spec_blocks` to rewrite spec blocks attached to structs, Move and spec functions, and modules
//! - `rewrite_func_def` to rewrite expressions in both Move and spec functions
//! - How to get alerted if we miss anything: all propagation of `i64`/`i128` beyond the AST (to stackless bytecode, to boogie type, or type tags) will abort the compilation!
//!
//! Important notes about linking struct modules:
//! - To ensure the `std::i64` and `std::i128` are preserved in GlobalEnv during compilation,
//!   - code is added in `third_party/move/move-model/src/lib.rs` to keep `std::i64` and `std::i128` and its dependencies during the ast expansion phase
//! - To ensure implicit dependencies introduced by comparison rewriting are maintained
//!   - code is added in `third_party/move/move-compiler-v2/legacy-move-compiler/src/expansion/dependency_ordering.rs` to add dependency between every user module and `std::i64` and `std::i128`.
//! - If the user does not include `std::i64` and `std::i128` for compilation,
//!   - a compilation error "cannot find `std::i64` and `std::i128` module" will be raised.
//!

use crate::env_pipeline::rewrite_target::{
    RewriteState, RewriteTarget, RewriteTargets, RewritingScope,
};
use move_core_types::{account_address::AccountAddress, identifier::Identifier};
use move_model::{
    ast::{Address, Exp, ExpData, ModuleName, Operation, QuantKind},
    exp_rewriter::ExpRewriterFunctions,
    model::{
        FieldId, FunId, FunctionEnv, GlobalEnv, Loc, ModuleEnv, ModuleId, NodeId, Parameter,
        QualifiedId, SpecFunId, StructId,
    },
    ty::*,
    well_known,
};
use num::BigInt;
use std::{
    collections::{BTreeMap, BTreeSet},
    convert::TryFrom,
    iter::Iterator,
    vec,
};

const SIGNED_INT_PKG: &str = "std";
const SIGNED_INT_LIB: &str = "move-stdlib";
const MODULE_COVERAGE_MSG: &str = "library modules for primitive signed int types expected";
const OP_NOT_SUPPORTED: &str = "operation not supported for signed integers";

/// Macro to report errors if struct modules or needed apis are not found in `GlobalEnv`
macro_rules! missing_module_error {
    ($env:expr, $loc:expr, $ty:expr) => {
        $env.error(
            $loc,
            &format!(
                "cannot find {}::{} module. Include `{}` for compilation.",
                SIGNED_INT_PKG,
                SignedIntRewriter::primitive_type_to_module_name($ty).expect(MODULE_COVERAGE_MSG),
                SIGNED_INT_LIB
            ),
        );
    };
}

/// Main interface for the signed integer rewriter
pub fn rewrite(env: &mut GlobalEnv) {
    let mut rewriter = SignedIntRewriter::new(env);
    let mut targets = RewriteTargets::create(env, RewritingScope::CompilationTarget);
    // why clone `targets`: `rewrite_func_def` will overwrite the states set up by `rewrite_func_decl`
    let mut fun_def_targets = targets.clone();
    rewrite_struct_def(&mut rewriter, &mut targets);
    rewrite_func_decl(&mut rewriter, &mut targets);
    rewrite_spec_blocks(&mut rewriter, &mut targets);
    rewrite_func_def(&mut rewriter, &mut fun_def_targets);

    // Write the mutations back all together. This avoids mixed immutable and mutable references to `env`
    targets.write_to_env(env);
    fun_def_targets.write_to_env(env);
}

/// Rewrite struct defs
fn rewrite_struct_def(rewriter: &mut SignedIntRewriter, targets: &mut RewriteTargets) {
    let todo: BTreeSet<_> = targets.keys().collect();
    for target in todo {
        if let RewriteTarget::MoveStruct(struct_id) = target {
            let new_def = rewriter.rewrite_struct_def(struct_id);
            if let Some(field_map) = new_def {
                *targets.state_mut(&target) = RewriteState::StructDef(field_map);
            }
        }
    }
}

/// Rewrite Move and spec function declarations
fn rewrite_func_decl(rewriter: &mut SignedIntRewriter, targets: &mut RewriteTargets) {
    let todo: BTreeSet<_> = targets.keys().collect();
    for target in todo {
        match target {
            RewriteTarget::MoveFun(func_id) => {
                let new_decl = rewriter.rewrite_func_decl(func_id);
                if let Some((params, ret_type)) = new_decl {
                    *targets.state_mut(&target) = RewriteState::Decl(params, ret_type);
                }
            },

            RewriteTarget::SpecFun(_spec_func_id) => {
                let new_decl = rewriter.rewrite_specfunc_decl(_spec_func_id);
                if let Some((params, ret_type)) = new_decl {
                    *targets.state_mut(&target) = RewriteState::Decl(params, ret_type);
                }
            },
            RewriteTarget::MoveStruct(_) | RewriteTarget::SpecBlock(_) => {},
        }
    }
}

/// Rewrite spec blocks attached to structs, Move functions, spec functions, and the module.
fn rewrite_spec_blocks(rewriter: &mut SignedIntRewriter, targets: &mut RewriteTargets) {
    let todo: BTreeSet<_> = targets.keys().collect();
    for target in todo {
        if let RewriteTarget::SpecBlock(ref spblock) = target {
            let spec = rewriter.env.get_spec_block(spblock);
            let (changed, new_spec) = rewriter.rewrite_spec_descent(spblock, &spec);
            if changed {
                *targets.state_mut(&target) = RewriteState::Spec(new_spec);
            }
        }
    }
}

/// Rewrite Move and spec function defs
fn rewrite_func_def(rewriter: &mut SignedIntRewriter, targets: &mut RewriteTargets) {
    let todo: BTreeSet<_> = targets.keys().collect();
    for target in todo {
        match target {
            RewriteTarget::MoveFun(func_id) => {
                let func_env = rewriter.env.get_function(func_id);
                if let Some(def_opt) = func_env.get_def() {
                    let new_def = rewriter.rewrite_exp(def_opt.clone());
                    if *def_opt != new_def {
                        *targets.state_mut(&target) = RewriteState::Def(new_def);
                    }
                }
            },
            RewriteTarget::SpecFun(_spec_func_id) => {
                let spec_fun = rewriter.env.get_spec_fun(_spec_func_id);
                if let Some(def_opt) = spec_fun.body.clone() {
                    let new_def = rewriter.rewrite_exp(def_opt.clone());
                    if def_opt != new_def {
                        *targets.state_mut(&target) = RewriteState::Def(new_def);
                    }
                }
            },
            RewriteTarget::MoveStruct(_) | RewriteTarget::SpecBlock(_) => {},
        }
    }
}

struct SignedIntRewriter<'env> {
    env: &'env GlobalEnv,
    // Mapping from struct modules to the list of APIs needed from each module
    // key: module name in string
    // value: (module_id, (function name in string, function_id))
    signed_int_apis: Option<BTreeMap<&'static str, (ModuleId, BTreeMap<&'static str, FunId>)>>,
    // Mapping from primitive signed int types to their corresponding struct type (e.g., i64 -> Struct std::i64::I64)
    primitive_ty_to_struct: Option<BTreeMap<Type, Type>>,
    // Mapping from struct types to their corresponding primitive signed int type (e.g., Struct std::i64::I64 -> i64)
    struct_to_primitive_ty: Option<BTreeMap<Type, Type>>,
}

impl<'env> SignedIntRewriter<'env> {
    fn new(env: &'env GlobalEnv) -> Self {
        let mut module_map = BTreeMap::new();
        // gather struct modules and the functions needed from them
        for module_name in [well_known::I64_MODULE, well_known::I128_MODULE] {
            if let Some(module) = Self::find_module(env, module_name) {
                let mut api_map = BTreeMap::new();
                for func_name in well_known::SIGNED_INT_FUNCTIONS {
                    if let Some(func) = Self::find_function(&module, func_name) {
                        api_map.insert(func_name, func.get_id());
                    } else {
                        // Abnormal and abort: module found but function not found
                        panic!("expected signed int function {}", func_name);
                    }
                }
                if !api_map.is_empty() {
                    module_map.insert(module_name, (module.get_id(), api_map));
                }
            }
        }

        // Build bi-mapping between primitive types and struct types
        let mut primitive_ty_to_struct = BTreeMap::new();
        let mut struct_to_primitive_ty = BTreeMap::new();
        for primitive_ty in [
            Type::Primitive(PrimitiveType::I64),
            Type::Primitive(PrimitiveType::I128),
        ] {
            let module_name =
                Self::primitive_type_to_module_name(&primitive_ty).expect(MODULE_COVERAGE_MSG);
            let struct_name =
                Self::primitive_type_to_struct_name(&primitive_ty).expect(MODULE_COVERAGE_MSG);

            if let Some(module_id) = Self::find_module_from_map(module_name, &module_map) {
                let module = env.get_module(module_id);
                let struct_identifier = Identifier::new_unchecked(struct_name);
                if let Some(struct_id) = module.find_struct_by_identifier(struct_identifier) {
                    primitive_ty_to_struct.insert(
                        primitive_ty.clone(),
                        Type::Struct(module_id, struct_id, vec![]),
                    );
                    struct_to_primitive_ty.insert(
                        Type::Struct(module_id, struct_id, vec![]),
                        primitive_ty.clone(),
                    );
                }
            }
        }

        Self {
            env,
            signed_int_apis: (!module_map.is_empty()).then_some(module_map),
            primitive_ty_to_struct: (!primitive_ty_to_struct.is_empty())
                .then_some(primitive_ty_to_struct),
            struct_to_primitive_ty: (!struct_to_primitive_ty.is_empty())
                .then_some(struct_to_primitive_ty),
        }
    }

    /// Function to iterate over struct fields and substitute their types from `i64`/`i128` to `Struct I64`/`Struct I128`
    /// Return `None` is no substitution is made.
    fn rewrite_struct_def(
        &self,
        struct_id: QualifiedId<StructId>,
    ) -> Option<BTreeMap<FieldId, Type>> {
        let struct_env = self.env.get_struct(struct_id);
        let old_field_type: BTreeMap<_, _> = struct_env
            .get_fields()
            .map(|field| (field.get_id(), field.get_type()))
            .collect();
        let new_field_type: BTreeMap<_, _> = old_field_type
            .iter()
            .map(|(id, ty)| {
                (
                    *id,
                    self.rewrite_type(ty, &struct_env.get_loc())
                        .unwrap_or_else(|| ty.clone()),
                )
            })
            .collect();

        if old_field_type != new_field_type {
            Some(new_field_type)
        } else {
            None
        }
    }

    /// Function to check function arguments and return value and substitute their types from `i64`/`i128` to `Struct I64`/`Struct I128`
    /// Return `None` is no substitution is made.
    fn rewrite_func_decl(&self, func_id: QualifiedId<FunId>) -> Option<(Vec<Parameter>, Type)> {
        let func_env = self.env.get_function(func_id);
        let params = func_env.get_parameters();
        let ret_type = func_env.get_result_type();

        let new_params: Vec<_> = params
            .iter()
            .map(|param| {
                let param_type = param.get_type();
                let param_loc = param.get_loc();
                self.rewrite_type(&param_type, &param_loc)
                    .map(|new_param_type| Parameter(param.get_name(), new_param_type, param_loc))
                    .unwrap_or_else(|| param.clone())
            })
            .collect();
        let new_ret_type = self.rewrite_type(&ret_type, &func_env.get_loc());
        if new_ret_type.is_some() || new_params != params {
            Some((new_params, new_ret_type.unwrap_or_else(|| ret_type.clone())))
        } else {
            None
        }
    }

    /// Function to check spec function arguments and return value and substitute their types from `i64`/`i128` to `Struct I64`/`Struct I128`
    /// Return `None` is no substitution is made.
    fn rewrite_specfunc_decl(
        &self,
        func_id: QualifiedId<SpecFunId>,
    ) -> Option<(Vec<Parameter>, Type)> {
        let spec_fun = self.env.get_spec_fun(func_id);
        let params = spec_fun.params.clone();
        let ret_type = spec_fun.result_type.clone();

        let new_params: Vec<_> = params
            .iter()
            .map(|param| {
                let param_type = param.get_type();
                let param_loc = param.get_loc();
                self.rewrite_type(&param_type, &param_loc)
                    .map(|new_param_type| Parameter(param.get_name(), new_param_type, param_loc))
                    .unwrap_or_else(|| param.clone())
            })
            .collect();

        let new_ret_type = self.rewrite_type(&ret_type, &spec_fun.loc);

        if new_ret_type.is_some() || new_params != params {
            Some((new_params, new_ret_type.unwrap_or_else(|| ret_type.clone())))
        } else {
            None
        }
    }

    /// Recursively rewrite `i64`/`i128` types to `Struct I64` and `Struct I128`
    fn rewrite_type(&self, ty: &Type, loc: &Loc) -> Option<Type> {
        match ty {
            Type::Primitive(PrimitiveType::I64) | Type::Primitive(PrimitiveType::I128) => {
                if let Some(primitive_ty_to_struct) = &self.primitive_ty_to_struct {
                    primitive_ty_to_struct.get(ty).cloned()
                } else {
                    missing_module_error!(self.env, loc, ty);
                    None
                }
            },
            Type::Reference(ref_kind, ref_ty) => self
                .rewrite_type(ref_ty, loc)
                .map(|new_ty| Type::Reference(*ref_kind, Box::new(new_ty))),
            Type::Tuple(elements) => {
                let new_elements: Vec<_> = elements
                    .iter()
                    .map(|elem| self.rewrite_type(elem, loc).unwrap_or(elem.clone()))
                    .collect();
                if new_elements != *elements {
                    Some(Type::Tuple(new_elements))
                } else {
                    None
                }
            },
            Type::Vector(elem_ty) => {
                let new_elem_ty = self.rewrite_type(elem_ty, loc)?;
                Some(Type::Vector(Box::new(new_elem_ty)))
            },
            Type::Struct(mid, sid, type_args) => {
                let new_type_args: Vec<_> = type_args
                    .iter()
                    .map(|arg| self.rewrite_type(arg, loc).unwrap_or(arg.clone()))
                    .collect();

                if new_type_args != *type_args {
                    Some(Type::Struct(*mid, *sid, new_type_args))
                } else {
                    None
                }
            },
            Type::Fun(param_type, ret_type, abi) => {
                let new_param_ty = self
                    .rewrite_type(param_type, loc)
                    .unwrap_or(*param_type.clone());
                let new_ret_ty = self
                    .rewrite_type(ret_type, loc)
                    .unwrap_or(*ret_type.clone());
                if new_param_ty != **param_type || new_ret_ty != **ret_type {
                    Some(Type::Fun(
                        Box::new(new_param_ty),
                        Box::new(new_ret_ty),
                        *abi,
                    ))
                } else {
                    None
                }
            },
            Type::TypeDomain(ty) => {
                let new_ty = self.rewrite_type(ty, loc).unwrap_or(*ty.clone());
                if new_ty != **ty {
                    Some(Type::TypeDomain(Box::new(new_ty)))
                } else {
                    None
                }
            },
            Type::ResourceDomain(mid, sid, opt_tys) => opt_tys.as_ref().and_then(|tys| {
                let new_opt_types: Vec<_> = tys
                    .iter()
                    .map(|ty| self.rewrite_type(ty, loc).unwrap_or_else(|| ty.clone()))
                    .collect();
                if new_opt_types != *tys {
                    Some(Type::ResourceDomain(*mid, *sid, Some(new_opt_types)))
                } else {
                    None
                }
            }),
            // These types cannot involve `i64` or `i128`
            Type::Primitive(_) | Type::TypeParameter(_) | Type::Var(_) | Type::Error => None,
        }
    }

    /// Recursively rewrite a vector of types
    /// Any type that cannot be rewritten will be returned as it is
    fn rewrite_type_vec(&self, tys: &Vec<Type>, loc: &Loc) -> Option<Vec<Type>> {
        let new_tys: Vec<_> = tys
            .iter()
            .map(|t| self.rewrite_type(t, loc).unwrap_or(t.clone()))
            .collect();
        // if no arg is rewritten, return nothing
        if &new_tys == tys {
            None
        } else {
            Some(new_tys)
        }
    }

    /******* A list of helper functions*******/
    /// Locate a struct module by its name from the GlobalEnv
    fn find_module(env: &'env GlobalEnv, name: &str) -> Option<ModuleEnv<'env>> {
        let module_name = ModuleName::new(
            Address::Numerical(AccountAddress::ONE),
            env.symbol_pool().make(name),
        );
        env.find_module(&module_name)
    }

    /// Locate a function by its string identifier from the given ModuleEnv
    fn find_function(module: &ModuleEnv<'env>, func_name: &str) -> Option<FunctionEnv<'env>> {
        let func_sym = module.symbol_pool().make(func_name);
        module.find_function(func_sym)
    }

    /// Find a struct module id by its string identifier, assuming we already have a map of the struct modules gathered from the GlobalEnv
    /// `module_map` is guaranteed to use Address::ONE
    fn find_module_from_map(
        module_name: &'static str,
        module_map: &BTreeMap<&'static str, (ModuleId, BTreeMap<&'static str, FunId>)>,
    ) -> Option<ModuleId> {
        module_map.get(module_name).map(|(module_id, _)| *module_id)
    }

    /// Mapping a primitive signed int type to its struct module name
    fn primitive_type_to_module_name(ty: &Type) -> Option<&'static str> {
        match ty {
            Type::Primitive(PrimitiveType::I64) => Some(well_known::I64_MODULE),
            Type::Primitive(PrimitiveType::I128) => Some(well_known::I128_MODULE),
            _ => None,
        }
    }

    /// Mapping a primitive signed int type to its counterpart struct name
    fn primitive_type_to_struct_name(ty: &Type) -> Option<&'static str> {
        match ty {
            Type::Primitive(PrimitiveType::I64) => Some(well_known::I64_STRUCT),
            Type::Primitive(PrimitiveType::I128) => Some(well_known::I128_STRUCT),
            _ => None,
        }
    }

    /// Mapping a primitive signed int type to its counterpart struct type
    fn get_primitive_ty_from_struct(&self, ty: &Type) -> Option<Type> {
        self.struct_to_primitive_ty
            .as_ref()
            .and_then(|map| map.get(ty))
            .cloned()
    }

    /// Check if a type indicates the needs of rewriting
    /// - Note: this function does not check nested types except for references!!!
    fn need_rewrite(&self, ty: &Type) -> bool {
        Self::is_signed_primitive_type(ty)
            || self.is_signed_struct_type(ty)
            || self.is_signed_ref_type(ty)
    }

    /// A variant of `need_rewrite` that does not check for references
    fn need_rewrite_no_ref(&self, ty: &Type) -> bool {
        Self::is_signed_primitive_type(ty) || self.is_signed_struct_type(ty)
    }

    fn is_signed_primitive_type(ty: &Type) -> bool {
        matches!(
            ty,
            Type::Primitive(PrimitiveType::I64) | Type::Primitive(PrimitiveType::I128)
        )
    }

    fn is_signed_struct_type(&self, ty: &Type) -> bool {
        self.struct_to_primitive_ty
            .as_ref()
            .and_then(|map| map.get(ty))
            .is_some()
    }

    fn is_signed_ref_type(&self, ty: &Type) -> bool {
        matches!(ty, Type::Reference(_, ref_ty) if self.is_signed_struct_type(ref_ty) || Self::is_signed_primitive_type(ref_ty))
    }

    /// Rewrite an argument of an Operation. It requires that the given argument
    /// - if a primitive signed integer, must have been rewritten to the struct type
    /// - if a reference type, must refer to a signed integer struct type
    fn rewrite_ref_allowed_arg(&mut self, arg: &Exp) -> Option<Exp> {
        let res_ty = self.env.get_node_type(arg.node_id());
        let loc = self.env.get_node_loc(arg.node_id());
        match res_ty {
            // If signed int primitive types, abort!
            Type::Primitive(..) if Self::is_signed_primitive_type(&res_ty) => {
                panic!("Primitive signed integers must have been rewritten")
            },
            // If signed struct types, return
            Type::Struct(..) if self.is_signed_struct_type(&res_ty) => Some(arg.clone()),
            // If reference types, strip the reference or dereference
            Type::Reference(_, ref_ty) if self.is_signed_struct_type(&ref_ty) => {
                // - if borrowed with one arg exp, return the arg exp as an optimization
                if let ExpData::Call(_, Operation::Borrow(_), inner_args) = arg.as_ref() {
                    if inner_args.len() == 1 {
                        return Some(inner_args[0].clone());
                    }
                }
                // - otherwise, dereference
                let new_node_id = self.env.new_node(loc, (*ref_ty).clone());
                Some(ExpData::Call(new_node_id, Operation::Deref, vec![arg.clone()]).into_exp())
            },
            _ => None,
        }
    }

    /// Mapping an operation over signed int to a wrapper function from the struct module
    fn get_operation_api(&self, res_ty: &Type, oper: &Operation) -> Option<(ModuleId, FunId)> {
        // Get the struct module name based on the node type of the operation
        let module_name = match res_ty {
            Type::Primitive(PrimitiveType::I64) | Type::Primitive(PrimitiveType::I128) => {
                Self::primitive_type_to_module_name(res_ty)
            },
            Type::Struct(_, _, _) => self
                .struct_to_primitive_ty
                .as_ref()
                .and_then(|map| map.get(res_ty))
                .and_then(Self::primitive_type_to_module_name),
            _ => None,
        }?;

        let module_map = self.signed_int_apis.as_ref()?.get(module_name)?;
        match oper {
            Operation::Add => Some((module_map.0, *module_map.1.get("add")?)),
            Operation::Sub => Some((module_map.0, *module_map.1.get("sub")?)),
            Operation::Mul => Some((module_map.0, *module_map.1.get("mul")?)),
            Operation::Div => Some((module_map.0, *module_map.1.get("div")?)),
            Operation::Mod => Some((module_map.0, *module_map.1.get("mod")?)),
            Operation::Eq => Some((module_map.0, *module_map.1.get("eq")?)),
            Operation::Neq => Some((module_map.0, *module_map.1.get("neq")?)),
            Operation::Gt => Some((module_map.0, *module_map.1.get("gt")?)),
            Operation::Lt => Some((module_map.0, *module_map.1.get("lt")?)),
            Operation::Ge => Some((module_map.0, *module_map.1.get("gte")?)),
            Operation::Le => Some((module_map.0, *module_map.1.get("lte")?)),
            Operation::Neg => Some((module_map.0, *module_map.1.get("neg")?)),
            _ => None,
        }
    }

    /// Mapping an node type and an api name to a function from the struct module
    fn get_api_by_ty_and_name(&self, res_ty: &Type, name: &str) -> Option<(ModuleId, FunId)> {
        // Get the struct module name based on the node type of the operation
        let module_name = match res_ty {
            Type::Primitive(PrimitiveType::I64) | Type::Primitive(PrimitiveType::I128) => {
                Self::primitive_type_to_module_name(res_ty)
            },
            Type::Struct(_, _, _) => self
                .struct_to_primitive_ty
                .as_ref()
                .and_then(|map| map.get(res_ty))
                .and_then(Self::primitive_type_to_module_name),
            _ => None,
        }?;

        let module_map = self.signed_int_apis.as_ref()?.get(module_name)?;
        Some((module_map.0, *module_map.1.get(name)?))
    }

    /// Rewrite the node type and instantiation type as needed and return an updated node id
    /// Either condition below indicates the need of a rewrite
    /// - The the node type involves a signed int type
    /// - The the node instantiation, if any (indicated by argument `inst`), involves a signed int type
    fn update_node_type(&self, id: NodeId, inst: bool) -> Option<NodeId> {
        let loc = self.env.get_node_loc(id);
        let node_ty = self.env.get_node_type(id);
        let node_inst_vec = self.env.get_node_instantiation(id);

        let new_node_type = self.rewrite_type(&node_ty, &loc);
        let new_inst_tys = if inst {
            self.rewrite_type_vec(&node_inst_vec, &loc)
        } else {
            None
        };

        if new_node_type.is_some() || new_inst_tys.is_some() {
            let new_node = self
                .env
                .new_node(loc.clone(), new_node_type.unwrap_or(node_ty));
            if inst {
                self.env
                    .set_node_instantiation(new_node, new_inst_tys.unwrap_or(node_inst_vec));
            }
            Some(new_node)
        } else {
            None
        }
    }

    /// Create a new node with potentially updated type and instantiation
    fn update_node_op(&self, id: NodeId, inst: bool) -> NodeId {
        self.update_node_type(id, inst).unwrap_or_else(|| {
            self.env
                .new_node(self.env.get_node_loc(id), self.env.get_node_type(id))
        })
    }
}

/// Rewrite different expressions by overriding the `rewrite_*` methods from the `ExpRewriterFunctions` trait
/// - As descendant expressions are processed first, we can safely assume the arguments to an expression are already rewritten
///
/// To ensure handling of all signed int types, we must
/// - rewrite all operations that need to be replaced with their corresponding functions from the struct modules
/// - rewrite all node types that involve signed int
/// - rewrite all type instantiations that involve signed int
///
impl ExpRewriterFunctions for SignedIntRewriter<'_> {
    /// This function rewrite a constant `i64` or `i128`
    /// - If the input node type has signed int type
    ///   - It replaces the constant with a call to the corresponding `pack` function in the struct module to construct a struct value
    ///   - No instantiation is needed for the new node, as `pack` takes none.
    fn rewrite_value(&mut self, id: NodeId, value: &move_model::ast::Value) -> Option<Exp> {
        // Not an integer
        let move_model::ast::Value::Number(num) = value else {
            return None;
        };
        let loc = self.env.get_node_loc(id);
        let node_ty = self.env.get_node_type(id);
        // Not a target of rewriting
        if !self.need_rewrite_no_ref(&node_ty) {
            return None;
        }
        // For ease of matching, get the primitive type if the node type is a struct type
        // Why a value node can have a struct type: it can be unified with other signed int struct types used in user code
        let node_ty = self
            .get_primitive_ty_from_struct(&node_ty)
            .unwrap_or(node_ty);
        match node_ty {
            // `i64` and `i128` constants have been properly stored into `num` with signedness handled by `translate_number`
            // so we check if `num` falls into the correct signed int range
            Type::Primitive(PrimitiveType::I64) if i64::try_from(num).is_ok() => {
                // get the `pack` function from the struct module
                let Some((mid, fid)) = self.get_api_by_ty_and_name(&node_ty, "pack") else {
                    missing_module_error!(self.env, &loc, &node_ty);
                    return None;
                };
                // prepare an `u64` argument for the `pack` function by copying the bits of the `i64` value, following the semantics of `pack`!
                let arg = ExpData::Value(
                    self.env.new_node(loc, Type::Primitive(PrimitiveType::U64)),
                    move_model::ast::Value::Number(BigInt::from(
                        i64::try_from(num).expect("value ensured to be `i64`") as u64,
                    )),
                )
                .into_exp();
                // assemble a call to `pack`
                Some(
                    ExpData::Call(
                        self.update_node_op(id, false),
                        Operation::MoveFunction(mid, fid),
                        vec![arg],
                    )
                    .into_exp(),
                )
            },
            Type::Primitive(PrimitiveType::I128) if i128::try_from(num).is_ok() => {
                let Some((mid, fid)) = self.get_api_by_ty_and_name(&node_ty, "pack") else {
                    missing_module_error!(self.env, &loc, &node_ty);
                    return None;
                };
                // handled similarly to `i64`
                let arg = ExpData::Value(
                    self.env.new_node(loc, Type::Primitive(PrimitiveType::U128)),
                    move_model::ast::Value::Number(BigInt::from(
                        i128::try_from(num).expect("value ensured to be `i128`") as u128,
                    )),
                )
                .into_exp();
                Some(
                    ExpData::Call(
                        self.update_node_op(id, false),
                        Operation::MoveFunction(mid, fid),
                        vec![arg],
                    )
                    .into_exp(),
                )
            },
            _ => panic!("expected signed int type and value"),
        }
    }

    /// ExpData::Temporary
    /// - Function parameters have been rewritten, so only need to rewrite the node type
    /// - Instantiation: following the original node; rewriting if needed
    fn rewrite_temporary(&mut self, id: NodeId, idx: move_model::ast::TempIndex) -> Option<Exp> {
        Some(ExpData::Temporary(self.update_node_type(id, true)?, idx).into_exp())
    }

    /// ExpData::LocalVar
    /// - Node type: rewrite
    /// - Instantiation: following the original node; rewriting if needed
    fn rewrite_local_var(&mut self, id: NodeId, sym: move_model::symbol::Symbol) -> Option<Exp> {
        Some(ExpData::LocalVar(self.update_node_type(id, true)?, sym).into_exp())
    }

    /// ExpData::Invoke
    /// - Node type: rewrite
    /// - Instantiation: following the original node; rewriting if needed
    fn rewrite_invoke(&mut self, id: NodeId, target: &Exp, args: &[Exp]) -> Option<Exp> {
        Some(
            ExpData::Invoke(
                self.update_node_type(id, true)?,
                target.clone(),
                args.to_vec(),
            )
            .into_exp(),
        )
    }

    /// ExpData::Lambda
    /// - Node type: rewrite
    /// - Instantiation: following the original node; rewriting if needed
    fn rewrite_lambda(
        &mut self,
        id: NodeId,
        pat: &move_model::ast::Pattern,
        body: &Exp,
        capture_kind: move_model::ast::LambdaCaptureKind,
        spec_opt: &Option<Exp>,
    ) -> Option<Exp> {
        Some(
            ExpData::Lambda(
                self.update_node_type(id, true)?,
                pat.clone(),
                body.clone(),
                capture_kind,
                spec_opt.clone(),
            )
            .into_exp(),
        )
    }

    /// ExpData::IfElse
    /// - Node type: rewrite
    /// - Instantiation: following the original node; rewriting if needed
    fn rewrite_if_else(&mut self, id: NodeId, cond: &Exp, then: &Exp, else_: &Exp) -> Option<Exp> {
        Some(
            ExpData::IfElse(
                self.update_node_type(id, true)?,
                cond.clone(),
                then.clone(),
                else_.clone(),
            )
            .into_exp(),
        )
    }

    /// ExpData::Sequence
    /// - Node type: rewrite
    /// - Instantiation: following the original node; rewriting if needed
    fn rewrite_sequence(&mut self, id: NodeId, seq: &[Exp]) -> Option<Exp> {
        Some(ExpData::Sequence(self.update_node_type(id, true)?, seq.to_vec()).into_exp())
    }

    /// ExpData::Block
    /// - Node type: rewrite
    /// - Instantiation: following the original node; rewriting if needed
    fn rewrite_block(
        &mut self,
        id: NodeId,
        pat: &move_model::ast::Pattern,
        binding: &Option<Exp>,
        body: &Exp,
    ) -> Option<Exp> {
        Some(
            ExpData::Block(
                self.update_node_type(id, true)?,
                pat.clone(),
                binding.clone(),
                body.clone(),
            )
            .into_exp(),
        )
    }

    /// ExpData::Quant (spec only)
    /// - Node type: rewrite
    /// - Instantiation: following the original node; rewriting if needed
    fn rewrite_quant(
        &mut self,
        id: NodeId,
        kind: &QuantKind,
        ranges: &[(move_model::ast::Pattern, Exp)],
        triggers: &[Vec<Exp>],
        cond: &Option<Exp>,
        body: &Exp,
    ) -> Option<Exp> {
        Some(
            ExpData::Quant(
                self.update_node_type(id, true)?,
                *kind,
                ranges.to_vec(),
                triggers.to_vec(),
                cond.clone(),
                body.clone(),
            )
            .into_exp(),
        )
    }

    /// Pattern
    /// `_creating_scope` or not does not matter here, since we only care about the `Var` type.
    /// - For stand alone `Var`:
    ///   - Node type: rewrite
    ///   - Instantiation: following the original node; rewriting if needed
    fn rewrite_pattern(
        &mut self,
        pat: &move_model::ast::Pattern,
        _creating_scope: bool,
    ) -> Option<move_model::ast::Pattern> {
        match pat {
            move_model::ast::Pattern::Var(id, sym) => Some(move_model::ast::Pattern::Var(
                self.update_node_type(*id, true)?,
                *sym,
            )),
            // Nested patterns are recursively handled by the rewriting framework, so safe to skip them
            _ => None,
        }
    }

    /// ExpData::Call
    /// - Handled case by case
    fn rewrite_call(&mut self, call_id: NodeId, oper: &Operation, args: &[Exp]) -> Option<Exp> {
        let node_ty = self.env.get_node_type(call_id);
        let node_loc = self.env.get_node_loc(call_id);
        match oper {
            // These operations will be replaced with their corresponding Move function calls
            // - Arguments: should have all been rewritten
            // - Node type: rewrite
            // - Instantiation: not needed, following the definition of the Move functions
            Operation::Add
            | Operation::Sub
            | Operation::Mul
            | Operation::Div
            | Operation::Mod
            | Operation::Neg => {
                if !self.need_rewrite_no_ref(&node_ty) {
                    return None;
                }
                // Get the Move function for the operation
                let Some((module_id, function_id)) = self.get_operation_api(&node_ty, oper) else {
                    // Error if target module is missing
                    missing_module_error!(self.env, &node_loc, &node_ty);
                    return None;
                };
                let new_node_id = self.update_node_op(call_id, false);
                Some(
                    ExpData::Call(
                        new_node_id,
                        Operation::MoveFunction(module_id, function_id),
                        args.to_vec(),
                    )
                    .into_exp(),
                )
            },
            // Comparison operations will also be replaced with their corresponding Move function calls
            // why handle them separately: they are allowed to take reference types by the built-in operators!
            // - Arguments: should have all been rewritten
            // - Node type: no rewrite needed, as they always return `bool`
            // - Instantiation: not needed, following the definition of the Move functions
            Operation::Eq
            | Operation::Neq
            | Operation::Lt
            | Operation::Gt
            | Operation::Ge
            | Operation::Le => {
                // incorrect arg number or type
                if args.len() != 2
                    || args
                        .iter()
                        .any(|arg| !self.need_rewrite(&self.env.get_node_type(arg.node_id())))
                {
                    return None;
                }

                // rewrite all the args
                let new_args: Vec<_> = args
                    .iter()
                    .filter_map(|arg| self.rewrite_ref_allowed_arg(arg))
                    .collect();

                let Some((module_id, function_id)) =
                    self.get_operation_api(&self.env.get_node_type(new_args[0].node_id()), oper)
                else {
                    let report_ty = self
                        .struct_to_primitive_ty
                        .as_ref()?
                        .get(&self.env.get_node_type(new_args[0].node_id()))?;
                    missing_module_error!(self.env, &node_loc, &report_ty);
                    return None;
                };
                let new_node_id = self.env.new_node(node_loc.clone(), node_ty);
                Some(
                    ExpData::Call(
                        new_node_id,
                        Operation::MoveFunction(module_id, function_id),
                        new_args,
                    )
                    .into_exp(),
                )
            },
            // Cast needs special handling; Now we only allow two kinds of casting:
            // - `i64/i128` -> `u64/u128` (via `std::i64/i128::unpack`)
            // - `u64/u128` -> `i64/i128` (via `std::u64/u128::pack`)
            // - both kinds follow the casting rule of Rust. They basically copy the bits without changing them.
            Operation::Cast => {
                // wrong arg number
                if args.len() != 1 {
                    return None;
                }
                let src_ty = self.env.get_node_type(args[0].node_id());
                let dst_ty = node_ty.clone();
                // no need to rewrite
                if !self.need_rewrite_no_ref(&src_ty) && !self.need_rewrite_no_ref(&dst_ty) {
                    return None;
                }

                // let's get the primitive types for easier check
                let src_ty = self.get_primitive_ty_from_struct(&src_ty).unwrap_or(src_ty);
                let dst_ty = self.get_primitive_ty_from_struct(&dst_ty).unwrap_or(dst_ty);

                match (&src_ty, &dst_ty) {
                    // src: i64/i128 -> dst: u64/u128
                    (Type::Primitive(PrimitiveType::I64), Type::Primitive(PrimitiveType::U64))
                    | (
                        Type::Primitive(PrimitiveType::I128),
                        Type::Primitive(PrimitiveType::U128),
                    ) => {
                        let Some((mid, fid)) = self.get_api_by_ty_and_name(&src_ty, "unpack")
                        else {
                            missing_module_error!(self.env, &node_loc, &src_ty);
                            return None;
                        };
                        // arguments: have been rewritten
                        // node type: no need to rewrite
                        // instantiation: not needed as per `unpack`
                        let new_node_id = self.env.new_node(node_loc.clone(), node_ty);
                        Some(
                            ExpData::Call(new_node_id, Operation::MoveFunction(mid, fid), vec![
                                args[0].clone(),
                            ])
                            .into_exp(),
                        )
                    },
                    // src: u64/u128 -> dst: i64/i128
                    (Type::Primitive(PrimitiveType::U64), Type::Primitive(PrimitiveType::I64))
                    | (
                        Type::Primitive(PrimitiveType::U128),
                        Type::Primitive(PrimitiveType::I128),
                    ) => {
                        let Some((mid, fid)) = self.get_api_by_ty_and_name(&dst_ty, "pack") else {
                            missing_module_error!(self.env, &node_loc, &dst_ty);
                            return None;
                        };
                        // arguments: no rewrite needed
                        // node type: rewrite
                        // instantiation: not needed as per `pack`
                        Some(
                            ExpData::Call(
                                self.update_node_op(call_id, false),
                                Operation::MoveFunction(mid, fid),
                                vec![args[0].clone()],
                            )
                            .into_exp(),
                        )
                    },
                    _ => {
                        self.env.error(&node_loc, OP_NOT_SUPPORTED);
                        None
                    },
                }
            },

            // The following operations may need to update their node types and instantiation types
            Operation::MoveFunction(..)
            | Operation::Pack(..)
            | Operation::Closure(..)
            | Operation::Tuple
            | Operation::Select(..)
            | Operation::SelectVariants(..)
            | Operation::TestVariants(..)
            | Operation::Copy
            | Operation::Move
            | Operation::Exists(..)
            | Operation::BorrowGlobal(..)
            | Operation::Borrow(..)
            | Operation::Deref
            | Operation::MoveTo
            | Operation::MoveFrom
            | Operation::Freeze(..)
            | Operation::Vector => Some(
                ExpData::Call(
                    self.update_node_type(call_id, true)?,
                    oper.clone(),
                    args.to_vec(),
                )
                .into_exp(),
            ),
            // These operations are not supported for signed int types
            Operation::BitAnd
            | Operation::BitOr
            | Operation::Shl
            | Operation::Shr
            | Operation::Xor
            | Operation::And
            | Operation::Or
            | Operation::Not => {
                if args.is_empty()
                    || !self.need_rewrite_no_ref(&self.env.get_node_type(args[0].node_id()))
                {
                    return None;
                }
                self.env.error(&node_loc, OP_NOT_SUPPORTED);
                None
            },

            // Nothing needs to be done
            Operation::Abort
            | Operation::NoOp
            | Operation::SpecFunction(..)
            | Operation::UpdateField(..)
            | Operation::Result(_)
            | Operation::Index
            | Operation::Slice
            | Operation::Range
            | Operation::Implies
            | Operation::Iff
            | Operation::Identical
            | Operation::Len
            | Operation::TypeValue
            | Operation::TypeDomain
            | Operation::ResourceDomain
            | Operation::Global(_)
            | Operation::CanModify
            | Operation::Old
            | Operation::Trace(_)
            | Operation::EmptyVec
            | Operation::SingleVec
            | Operation::UpdateVec
            | Operation::ConcatVec
            | Operation::IndexOfVec
            | Operation::ContainsVec
            | Operation::InRangeRange
            | Operation::InRangeVec
            | Operation::RangeVec
            | Operation::MaxU8
            | Operation::MaxU16
            | Operation::MaxU32
            | Operation::MaxU64
            | Operation::MaxU128
            | Operation::MaxU256
            | Operation::Bv2Int
            | Operation::Int2Bv
            | Operation::AbortFlag
            | Operation::AbortCode
            | Operation::WellFormed
            | Operation::BoxValue
            | Operation::UnboxValue
            | Operation::EmptyEventStore
            | Operation::ExtendEventStore
            | Operation::EventStoreIncludes
            | Operation::EventStoreIncludedIn => None,
        }
    }
}
